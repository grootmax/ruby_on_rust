#!/usr/bin/env ruby
# frozen_string_literal: true

require 'open3'
require 'optparse'
require 'json'
require 'fileutils'
require 'pathname'
require 'tempfile'
require 'timeout'
require 'rbconfig'
require 'shellwords'

module RubyOnRust
  class DiffRunner
    Result = Struct.new(
      :file,
      :exit_status,
      :stdout,
      :stderr,
      :exception_class,
      :exception_message,
      :inspect_return,
      :error,
      keyword_init: true
    )

    attr_reader :options, :ref_ruby, :ror_ruby, :start_time

    def initialize(options)
      @options = options
      @ref_ruby = options[:ref_ruby]
      @ror_ruby = options[:ror_ruby]
      @start_time = Time.now
    end

    def run(paths)
      files = resolve_files(paths)
      if files.empty?
        warn "No test snippet files found."
        return false
      end

      results = []
      passed = 0
      failed = 0

      if options[:format] == 'tap'
        puts "1..#{files.size}"
      end

      files.each_with_index do |file, idx|
        ref_res = run_snippet(@ref_ruby, file)
        ror_res = run_snippet(@ror_ruby, file)

        diffs = compare_results(ref_res, ror_res)
        is_pass = diffs.empty?

        if is_pass
          passed += 1
        else
          failed += 1
        end

        entry = {
          file: relative_path(file),
          pass: is_pass,
          diffs: diffs,
          ref: ref_res.to_h,
          ror: ror_res.to_h
        }
        results << entry

        report_snippet(entry, idx + 1, files.size)

        if !is_pass && options[:fail_fast]
          warn "\n[FAIL-FAST] Stopping on first failure: #{relative_path(file)}" if options[:format] == 'pretty'
          break
        end
      end

      report_summary(results, passed, failed, files.size)
      failed == 0
    end

    private

    def resolve_files(paths)
      if paths.empty?
        default_dir = File.expand_path('../../test/ruby_on_rust', __dir__)
        if Dir.exist?(default_dir)
          paths = [File.join(default_dir, '**', '*.rb')]
        else
          paths = []
        end
      end

      files = []
      paths.each do |p|
        abs_p = File.expand_path(p, Dir.pwd)
        if File.directory?(abs_p)
          files.concat(Dir.glob(File.join(abs_p, '**', '*.rb')))
        elsif File.file?(abs_p)
          files << abs_p
        else
          matched = Dir.glob(abs_p)
          matched = Dir.glob(p) if matched.empty?
          files.concat(matched)
        end
      end
      files.uniq.select { |f| File.file?(f) }.sort
    end

    def run_snippet(ruby_bin, file)
      wrapper_code = <<~RUBY
        # frozen_string_literal: true
        snippet_path = ARGV[0]
        begin
          code = File.read(snippet_path)
          val = eval(code, TOPLEVEL_BINDING, snippet_path, 1)
          $stdout.puts "\\n__ROR_DIFF_RESULT__:" + val.inspect
        rescue SystemExit => e
          exit e.status
        rescue Exception => e
          $stderr.puts "\\n__ROR_DIFF_EXCEPTION__:" + e.class.name + ":" + e.message.to_s
          exit 1
        end
      RUBY

      tmp_wrapper = Tempfile.new(['ror_wrapper', '.rb'])
      tmp_wrapper.write(wrapper_code)
      tmp_wrapper.close

      stdout_raw = ''
      stderr_raw = ''
      exit_status = 0
      err_msg = nil

      cmd = Shellwords.split(ruby_bin) + [tmp_wrapper.path, file]
      begin
        Timeout.timeout(options[:timeout]) do
          stdout_raw, stderr_raw, status = Open3.capture3(*cmd)
          exit_status = status.exitstatus || status.termsig || 1
        end
      rescue Timeout::Error
        err_msg = "Execution timed out after #{options[:timeout]}s"
        exit_status = 124
      rescue StandardError => e
        err_msg = e.message
        exit_status = 127
      ensure
        tmp_wrapper.unlink rescue nil
      end

      inspect_return = nil
      stdout_clean = stdout_raw
      if stdout_raw.include?('__ROR_DIFF_RESULT__:')
        parts = stdout_raw.split('__ROR_DIFF_RESULT__:', 2)
        stdout_clean = parts[0].chomp
        inspect_return = parts[1]&.strip
      end

      exception_class = nil
      exception_message = nil
      stderr_clean = stderr_raw
      if stderr_raw.include?('__ROR_DIFF_EXCEPTION__:')
        parts = stderr_raw.split('__ROR_DIFF_EXCEPTION__:', 2)
        stderr_clean = parts[0].chomp
        ex_parts = (parts[1] || '').strip.split(':', 2)
        exception_class = ex_parts[0]
        exception_message = ex_parts[1]
      end

      if options[:normalize]
        stdout_clean = normalize(stdout_clean, file)
        stderr_clean = normalize(stderr_clean, file)
        exception_message = normalize(exception_message, file) if exception_message
        inspect_return = normalize(inspect_return, file) if inspect_return
      end

      Result.new(
        file: relative_path(file),
        exit_status: exit_status,
        stdout: stdout_clean,
        stderr: stderr_clean,
        exception_class: exception_class,
        exception_message: exception_message,
        inspect_return: inspect_return,
        error: err_msg
      )
    end

    def normalize(str, file_path)
      return str if str.nil? || str.empty?

      res = str.dup
      # Replace memory addresses 0x00007f... or 0x7f...
      res.gsub!(/0x[0-9a-fA-F]{8,16}/, '0x000000000000')

      # Replace absolute repository/system paths
      repo_root = File.expand_path('../..', __dir__)
      res.gsub!(repo_root + '/', '')
      res.gsub!(File.dirname(file_path) + '/', '')

      # Replace PIDs like PID 12345 or process 12345
      res.gsub!(/(PID|pid|process)[:=\s]+\d+/i, '\1 <PID>')

      # Replace temp wrapper path references
      res.gsub!(/ror_wrapper[^\s:]+/, 'ror_wrapper.rb')

      res.strip
    end

    def relative_path(file)
      repo_root = File.expand_path('../..', __dir__)
      Pathname.new(file).relative_path_from(Pathname.new(repo_root)).to_s
    rescue StandardError
      file
    end

    def compare_results(ref, ror)
      diffs = {}
      if ref.error || ror.error
        diffs[:runner_error] = { ref: ref.error, ror: ror.error }
      end
      if ref.exit_status != ror.exit_status
        diffs[:exit_status] = { ref: ref.exit_status, ror: ror.exit_status }
      end
      if ref.stdout != ror.stdout
        diffs[:stdout] = { ref: ref.stdout, ror: ror.stdout }
      end
      if ref.stderr != ror.stderr
        diffs[:stderr] = { ref: ref.stderr, ror: ror.stderr }
      end
      if ref.exception_class != ror.exception_class
        diffs[:exception_class] = { ref: ref.exception_class, ror: ror.exception_class }
      end
      if ref.exception_message != ror.exception_message
        diffs[:exception_message] = { ref: ref.exception_message, ror: ror.exception_message }
      end
      if ref.inspect_return != ror.inspect_return
        diffs[:inspect_return] = { ref: ref.inspect_return, ror: ror.inspect_return }
      end
      diffs
    end

    def report_snippet(entry, index, total)
      file = entry[:file]
      is_pass = entry[:pass]

      case options[:format]
      when 'pretty'
        status_str = is_pass ? "\e[32m[PASS]\e[0m" : "\e[31m[FAIL]\e[0m"
        # Check if terminal supports colors; if not or in pipe, plain text
        if !$stdout.tty?
          status_str = is_pass ? "[PASS]" : "[FAIL]"
        end
        puts "#{status_str} (#{index}/#{total}) #{file}"

        if !is_pass
          puts "  --------------------------------------------------------------------------------"
          puts "  Divergence in: #{file}"
          puts "  Mismatched attributes:"
          entry[:diffs].each do |attr, values|
            puts "    - #{attr}:"
            puts "        CRuby (Ref)  : #{values[:ref].inspect}"
            puts "        Ruby on Rust : #{values[:ror].inspect}"
          end
          puts "  --------------------------------------------------------------------------------"
        end
      when 'brief'
        if is_pass
          puts "#{file}: PASS"
        else
          mismatched = entry[:diffs].keys.join(', ')
          puts "#{file}: FAIL (#{mismatched})"
        end
      when 'tap'
        if is_pass
          puts "ok #{index} - #{file}"
        else
          puts "not ok #{index} - #{file}"
          entry[:diffs].each do |attr, values|
            puts "#   #{attr} mismatch:"
            puts "#     ref: #{values[:ref].inspect}"
            puts "#     ror: #{values[:ror].inspect}"
          end
        end
      when 'json'
        # JSON output is produced in report_summary
      end
    end

    def report_summary(results, passed, failed, total)
      duration = (Time.now - start_time).round(3)

      case options[:format]
      when 'pretty'
        puts "\n" + "=" * 80
        puts "Differential Runner Test Summary"
        puts "=" * 80
        puts "Total Snippets : #{total}"
        puts "Passed         : #{passed}"
        puts "Failed         : #{failed}"
        puts "Duration       : #{duration}s"
        puts "=" * 80
        if failed > 0
          puts "\nResult: FAIL (#{failed} snippet(s) diverged between CRuby and Ruby on Rust)"
        else
          puts "\nResult: SUCCESS (100% parity across all snippets)"
        end
      when 'brief'
        puts "\nSummary: Total: #{total}, Passed: #{passed}, Failed: #{failed}, Time: #{duration}s"
      when 'tap'
        puts "# Total: #{total}, Passed: #{passed}, Failed: #{failed}, Time: #{duration}s"
      when 'json'
        data = {
          summary: {
            total: total,
            passed: passed,
            failed: failed,
            duration_seconds: duration,
            success: failed == 0
          },
          results: results
        }
        puts JSON.pretty_generate(data)
      end
    end

    def self.parse_options(argv)
      options = {
        ref_ruby: ENV['REF_RUBY'] || RbConfig.ruby,
        ror_ruby: ENV['ROR_RUBY'] || (File.executable?('./miniruby') ? './miniruby' : (File.executable?('./ruby') ? './ruby' : RbConfig.ruby)),
        format: 'pretty',
        fail_fast: false,
        normalize: true,
        verbose: false,
        timeout: 10
      }

      opt_parser = OptionParser.new do |opts|
        opts.banner = "Usage: ruby tool/ruby_on_rust/diff_runner.rb [options] [test_file_or_dir ...]"

        opts.on('--ref-ruby PATH', 'Path to reference CRuby binary (default: REF_RUBY or system ruby)') do |v|
          options[:ref_ruby] = v
        end

        opts.on('--ror-ruby PATH', 'Path to Ruby on Rust binary (default: ROR_RUBY or ./miniruby)') do |v|
          options[:ror_ruby] = v
        end

        opts.on('-f', '--fail-fast', 'Stop execution on the first failure or divergence') do
          options[:fail_fast] = true
        end

        opts.on('--format FORMAT', %w[pretty json tap brief], 'Output format (pretty, json, tap, brief; default: pretty)') do |v|
          options[:format] = v
        end

        opts.on('--[no-]normalize', 'Normalize non-deterministic outputs (memory addresses, PIDs, paths; default: true)') do |v|
          options[:normalize] = v
        end

        opts.on('-v', '--verbose', 'Enable verbose output') do
          options[:verbose] = true
        end

        opts.on('--timeout SECONDS', Float, 'Timeout in seconds per snippet execution (default: 10)') do |v|
          options[:timeout] = v
        end

        opts.on('-h', '--help', 'Show this help message') do
          puts opts
          exit 0
        end
      end

      opt_parser.parse!(argv)
      options
    end
  end
end

if __FILE__ == $0
  options = RubyOnRust::DiffRunner.parse_options(ARGV)
  runner = RubyOnRust::DiffRunner.new(options)
  success = runner.run(ARGV)
  exit(success ? 0 : 1)
end
