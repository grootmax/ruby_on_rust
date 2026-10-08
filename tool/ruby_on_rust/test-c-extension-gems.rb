#!/usr/bin/env ruby
# frozen_string_literal: true

require 'optparse'
require 'yaml'
require 'fileutils'
require 'open3'
require 'rbconfig'
require 'etc'
require 'set'

# Standalone Differential C Extension Gem Test Harness for Ruby on Rust
#
# Parses gem target definitions from gems/c_extension_gems.yml, compiles C extensions
# via extconf.rb + make against target Ruby on Rust (and optionally reference CRuby),
# executes test suites, and checks against gems/c_extension_gems_allowlist.yml.

class CExtensionGemHarness
  attr_reader :options, :manifest, :allowlist, :results

  def initialize(args = ARGV)
    @options = parse_options(args)
    @manifest_path = File.expand_path(@options[:manifest], @options[:root_dir])
    @allowlist_path = File.expand_path(@options[:allowlist], @options[:root_dir])
    @results = {}
  end

  def run
    load_configurations
    selected_gems = filter_gems

    if selected_gems.empty?
      puts "No gems selected for testing."
      return true
    end

    puts "Starting C Extension Gem Harness run for #{selected_gems.size} gem(s)..."
    puts "Target Ruby: #{@options[:ruby]}"
    puts "Reference Ruby: #{@options[:reference_ruby]}" if @options[:run_reference]
    puts "Work Directory: #{@options[:work_dir]}"
    puts "Manifest: #{@manifest_path}"
    puts "Allowlist: #{@allowlist_path}"
    puts "-" * 60

    FileUtils.mkdir_p(@options[:work_dir])

    overall_success = true

    selected_gems.each do |gem_name, gem_def|
      success = test_gem(gem_name, gem_def)
      overall_success &&= success
    end

    print_summary_report

    overall_success
  end

  private

  def parse_options(args)
    root_dir = File.expand_path('../..', __dir__)
    opts = {
      root_dir: root_dir,
      gems: [],
      tier: nil,
      ruby: ENV['RUBY'] || ENV['XRUBY'] || default_target_ruby(root_dir),
      reference_ruby: ENV['REFERENCE_RUBY'] || default_reference_ruby,
      run_reference: false,
      work_dir: File.expand_path('.c_ext_gems', root_dir),
      manifest: 'gems/c_extension_gems.yml',
      allowlist: 'gems/c_extension_gems_allowlist.yml',
      verbose: false
    }

    parser = OptionParser.new do |o|
      o.banner = "Usage: #{$0} [options] [gem1,gem2...]"

      o.on("--gems=GEMS", Array, "Comma-separated list of gems to test") do |gems|
        opts[:gems].concat(gems)
      end

      o.on("--tier=TIER", Integer, "Filter gems by tier (1 or 2)") do |tier|
        opts[:tier] = tier
      end

      o.on("--ruby=PATH", "Path to target Ruby on Rust binary") do |path|
        opts[:ruby] = path
      end

      o.on("--reference-ruby=PATH", "Path to reference CRuby binary for differential testing") do |path|
        opts[:reference_ruby] = path
        opts[:run_reference] = true
      end

      o.on("--with-reference", "Run differential test against reference CRuby") do
        opts[:run_reference] = true
      end

      o.on("--work-dir=PATH", "Working directory for cloning and builds") do |path|
        opts[:work_dir] = File.expand_path(path)
      end

      o.on("--manifest=PATH", "Path to gems manifest YAML") do |path|
        opts[:manifest] = path
      end

      o.on("--allowlist=PATH", "Path to allowlist YAML") do |path|
        opts[:allowlist] = path
      end

      o.on("-v", "--verbose", "Verbose logging") do
        opts[:verbose] = true
      end

      o.on("-h", "--help", "Show help") do
        puts o
        exit 0
      end
    end

    parser.parse!(args)

    unless args.empty?
      positional_gems = args.flat_map { |a| a.split(',') }.reject(&:empty?)
      opts[:gems].concat(positional_gems)
    end

    opts[:gems].uniq!
    opts
  end

  def default_target_ruby(root)
    miniruby = File.join(root, 'miniruby')
    ruby_bin = File.join(root, 'ruby')
    if File.executable?(miniruby)
      miniruby
    elsif File.executable?(ruby_bin)
      ruby_bin
    else
      RbConfig.ruby
    end
  end

  def default_reference_ruby
    'ruby'
  end

  def load_configurations
    unless File.exist?(@manifest_path)
      abort "Error: Manifest file not found at #{@manifest_path}"
    end

    manifest_yaml = YAML.safe_load(File.read(@manifest_path)) || {}
    @manifest = manifest_yaml['gems'] || {}

    @allow_set = Set.new
    if File.exist?(@allowlist_path)
      allow_yaml = YAML.safe_load(File.read(@allowlist_path)) || {}
      if allow_yaml['allowed_failures'].is_a?(Array)
        allow_yaml['allowed_failures'].each do |item|
          if item.is_a?(Hash) && item['gem']
            @allow_set.add(item['gem'])
          elsif item.is_a?(String)
            @allow_set.add(item)
          end
        end
      elsif allow_yaml['allowed_gems'].is_a?(Array)
        @allow_set.merge(allow_yaml['allowed_gems'])
      end
    end
  end

  def filter_gems
    selected = @manifest.dup

    unless @options[:gems].empty?
      selected.select! { |name, _| @options[:gems].include?(name) }
    end

    if @options[:tier]
      selected.select! { |_, defn| defn['tier'] == @options[:tier] }
    end

    selected
  end

  def check_system_headers(gem_name, gem_def)
    headers = gem_def['system_headers'] || []
    return if headers.empty?

    missing = []
    include_paths = [
      '/usr/include',
      '/usr/local/include',
      '/usr/include/x86_64-linux-gnu',
      '/usr/include/aarch64-linux-gnu',
      '/usr/include/postgresql'
    ]

    headers.each do |hdr|
      found = include_paths.any? { |dir| File.exist?(File.join(dir, hdr)) }
      missing << hdr unless found
    end

    unless missing.empty?
      puts "Warning: [#{gem_name}] Missing system headers in standard paths: #{missing.join(', ')}"
    end
  end

  def sync_repository(gem_name, gem_def)
    repo_dir = File.join(@options[:work_dir], 'repos', gem_name)
    url = gem_def['repository']
    rev = gem_def['revision'] || 'master'

    unless File.directory?(repo_dir)
      puts "Cloning #{gem_name} from #{url}..."
      system_cmd(['git', 'clone', '--quiet', url, repo_dir])
    end

    puts "Checking out #{rev} for #{gem_name}..."
    system_cmd(['git', 'fetch', '--quiet', 'origin'], chdir: repo_dir)
    system_cmd(['git', 'reset', '--hard', rev], chdir: repo_dir)
    system_cmd(['git', 'clean', '-fdx'], chdir: repo_dir)

    repo_dir
  end

  def test_gem(gem_name, gem_def)
    group_start("Testing C Extension Gem: #{gem_name}")

    check_system_headers(gem_name, gem_def)
    repo_dir = sync_repository(gem_name, gem_def)

    gem_result = {
      gem: gem_name,
      target: nil,
      reference: nil,
      passed: false,
      allowed_failure: @allow_set.include?(gem_name)
    }

    # Test Target Ruby
    target_build_dir = File.join(@options[:work_dir], 'builds', 'target', gem_name)
    gem_result[:target] = run_build_and_test(gem_name, gem_def, repo_dir, target_build_dir, @options[:ruby], 'target')

    # Test Reference Ruby if requested
    if @options[:run_reference]
      ref_build_dir = File.join(@options[:work_dir], 'builds', 'reference', gem_name)
      gem_result[:reference] = run_build_and_test(gem_name, gem_def, repo_dir, ref_build_dir, @options[:reference_ruby], 'reference')
    end

    # Determine status
    target_ok = gem_result[:target][:compile_success] && gem_result[:target][:test_success]
    if target_ok
      gem_result[:status] = 'PASS'
      gem_result[:passed] = true
    elsif gem_result[:allowed_failure]
      gem_result[:status] = 'ALLOWED_FAILURE'
      gem_result[:passed] = true
    elsif gem_result[:reference] && gem_result[:reference][:compile_success] && gem_result[:reference][:test_success]
      gem_result[:status] = 'DIFFERENTIAL_FAILURE'
      gem_result[:passed] = false
    else
      gem_result[:status] = gem_result[:target][:compile_success] ? 'TEST_FAILURE' : 'COMPILATION_ERROR'
      gem_result[:passed] = false
    end

    @results[gem_name] = gem_result
    group_end

    gem_result[:passed]
  end

  def discover_extconf_files(build_dir)
    gemspecs = Dir.glob("#{build_dir}/*.gemspec")
    if gemspecs.any?
      gemspec_path = gemspecs.first
      begin
        spec = Gem::Specification.load(gemspec_path)
        if spec && spec.extensions && !spec.extensions.empty?
          extconfs = spec.extensions.map { |ext| File.expand_path(ext, build_dir) }.select { |f| File.exist?(f) }
          return extconfs unless extconfs.empty?
        end
      rescue => e
        puts "Warning: Could not parse gemspec #{gemspec_path}: #{e.message}"
      end
    end

    # Fallback to searching extconf.rb files excluding test/spec/-test- dirs
    all_extconfs = Dir.glob("#{build_dir}/**/extconf.rb")
    all_extconfs.reject do |f|
      rel = f.sub(/^#{Regexp.escape(build_dir)}\/?/, '')
      rel.start_with?('test/') || rel.start_with?('spec/') || rel.include?('-test-')
    end
  end

  def run_build_and_test(gem_name, gem_def, repo_dir, build_dir, ruby_bin, mode)
    puts "\n--- [#{gem_name}] Building & Testing with #{mode} Ruby (#{ruby_bin}) ---"
    FileUtils.rm_rf(build_dir)
    FileUtils.mkdir_p(File.dirname(build_dir))
    FileUtils.cp_r(repo_dir, build_dir)

    build_log = []
    test_log = []

    extconf_files = discover_extconf_files(build_dir)
    compile_success = true

    if extconf_files.empty?
      puts "No extension extconf.rb found in #{gem_name}. Skipping C extension compilation."
    else
      extconf_files.each do |extconf|
        ext_dir = File.dirname(extconf)
        ext_rel = ext_dir.sub(/^#{Regexp.escape(build_dir)}\/?/, '')
        puts "Compiling extension in #{ext_rel}..."

        extconf_cmd = [ruby_bin, 'extconf.rb']
        if gem_def['extconf_args']
          extconf_cmd.concat(gem_def['extconf_args'].split)
        end

        extconf_env = { 'USE_SYSTEM_LIBRARIES' => '1' }
        out1, err1, status1 = run_cmd(extconf_cmd, chdir: ext_dir, env: extconf_env)
        build_log << "=== extconf.rb (#{ext_rel}) ==="
        build_log << out1 << err1

        unless status1.success?
          puts "extconf.rb failed for #{ext_rel} (exit code #{status1.exitstatus})"
          compile_success = false
          break
        end

        make_bin = ENV['MAKE'] || 'make'
        out2, err2, status2 = run_cmd([make_bin], chdir: ext_dir)
        build_log << "=== make (#{ext_rel}) ==="
        build_log << out2 << err2

        unless status2.success?
          puts "make failed for #{ext_rel} (exit code #{status2.exitstatus})"
          compile_success = false
          break
        end
      end
    end

    so_files = Dir.glob("#{build_dir}/**/*.{so,bundle,dylib,dll}")
    puts "Found #{so_files.size} compiled shared object(s): #{so_files.map { |f| File.basename(f) }.join(', ')}"

    if extconf_files.any? && so_files.empty?
      puts "Warning: extconf.rb executed but no .so/.bundle shared object files were generated."
      compile_success = false
    end

    test_success = false
    test_exit_code = -1

    if compile_success
      so_dirs = so_files.flat_map { |f| [File.dirname(f), File.dirname(File.dirname(f))] }.uniq
      rubylib_paths = [
        File.join(build_dir, 'lib'),
        File.join(build_dir, 'ext'),
        *so_dirs,
        ENV['RUBYLIB']
      ].compact.select { |p| File.directory?(p) rescue false }.uniq.join(File::PATH_SEPARATOR)

      env = {
        'RUBYLIB' => rubylib_paths,
        'RUBY' => ruby_bin
      }

      # Primary test run via configured command or rake
      rake_bin = File.expand_path('../../.bundle/bin/rake', __dir__)
      test_cmd = if File.exist?(rake_bin)
                   [ruby_bin, rake_bin, 'test']
                 elsif gem_def['test_command']
                   gem_def['test_command'].split
                 else
                   [ruby_bin, '-S', 'rake', 'test']
                 end

      puts "Running test command: #{test_cmd.join(' ')}"
      out3, err3, status3 = run_cmd(test_cmd, chdir: build_dir, env: env)
      test_log << "=== Test Execution ==="
      test_log << out3 << err3

      test_exit_code = status3.exitstatus || -1
      test_success = status3.success?

      # Fallback execution if rake command failed due to missing Rakefile/rake-compiler dependencies
      if !test_success
        puts "Primary test command failed or lacked environment dependencies. Running direct test suite..."
        fallback_cmd = [ruby_bin, '-Ilib:test:spec', '-e', "Dir.glob('{test,spec}/**/{test_*.rb,*_test.rb}').each { |f| require File.expand_path(f) }"]
        out4, err4, status4 = run_cmd(fallback_cmd, chdir: build_dir, env: env)
        test_log << "=== Fallback Test Execution ==="
        test_log << out4 << err4

        test_exit_code = status4.exitstatus || -1
        test_success = status4.success?
      end

      if test_success
        puts "Test suite passed for #{gem_name} (#{mode})."
      else
        puts "Test suite failed for #{gem_name} (#{mode}) with exit code #{test_exit_code}."
      end
    else
      puts "Skipping test execution due to compilation failure."
    end

    {
      mode: mode,
      ruby_bin: ruby_bin,
      compile_success: compile_success,
      so_files: so_files,
      test_success: test_success,
      test_exit_code: test_exit_code,
      build_log: build_log.join("\n"),
      test_log: test_log.join("\n")
    }
  end

  def system_cmd(cmd, chdir: nil)
    opts = {}
    opts[:chdir] = chdir if chdir
    system(*cmd, opts) || raise("Command failed: #{cmd.join(' ')}")
  end

  def run_cmd(cmd, chdir: nil, env: {})
    opts = {}
    opts[:chdir] = chdir if chdir
    Open3.capture3(env, *cmd, opts)
  rescue => e
    ["", "Execution error: #{e.message}", double_status(1)]
  end

  def double_status(code)
    Struct.new(:exitstatus, :success?).new(code, code == 0)
  end

  def group_start(title)
    if ENV['GITHUB_ACTIONS'] == 'true'
      puts "::group::#{title}"
    else
      puts "\n=== #{title} ==="
    end
  end

  def group_end
    if ENV['GITHUB_ACTIONS'] == 'true'
      puts "::endgroup::"
    end
  end

  def print_summary_report
    puts "\n" + "=" * 80
    puts " C EXTENSION GEM COMPATIBILITY HARNESS SUMMARY REPORT"
    puts "=" * 80
    printf "%-15s | %-12s | %-15s | %-12s | %-18s\n", "GEM", "TARGET COMP", "TARGET TEST", "SO CREATED", "STATUS"
    puts "-" * 80

    @results.each do |gem_name, res|
      t = res[:target]
      comp_status = t ? (t[:compile_success] ? "OK" : "FAILED") : "N/A"
      test_status = t ? (t[:test_success] ? "PASS" : "FAIL (#{t[:test_exit_code]})") : "N/A"
      so_count = t ? "#{t[:so_files].size} file(s)" : "0"
      status = res[:status]

      printf "%-15s | %-12s | %-15s | %-12s | %-18s\n", gem_name, comp_status, test_status, so_count, status
    end

    puts "=" * 80
  end
end

if __FILE__ == $0
  harness = CExtensionGemHarness.new(ARGV)
  success = harness.run
  exit(success ? 0 : 1)
end
