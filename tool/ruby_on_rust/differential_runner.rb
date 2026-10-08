#!/usr/bin/env ruby
# frozen_string_literal: true

require "optparse"
require "open3"
require "fileutils"
require "tempfile"
require "json"
require "shellwords"
require "rbconfig"

class DifferentialRunner
  BUILTIN_SNIPPETS = {
    "string_operations" => <<~RUBY,
      s = "Ruby on Rust"
      puts s.downcase
      puts s.upcase
      puts s.reverse
      puts s.split(" ").inspect
      puts s.bytes.inspect
      puts "hello \u{1F600}".encoding.name
    RUBY

    "array_transformations" => <<~RUBY,
      arr = [1, 2, 3, 4, 5, 6]
      puts arr.map { |x| x * 2 }.inspect
      puts arr.select(&:even?).inspect
      puts arr.reduce(0, :+).inspect
      puts arr.flatten.inspect
      puts arr.rotate(2).inspect
    RUBY

    "hash_manipulation" => <<~RUBY,
      h = { a: 1, b: 2, c: 3 }
      puts h.keys.inspect
      puts h.values.inspect
      puts h.transform_values { |v| v * 10 }.inspect
      puts h.merge({ d: 4 }).inspect
    RUBY

    "integer_float_math" => <<~RUBY,
      puts (2**63 - 1).to_s
      puts (10 / 3.0).round(4)
      puts Math.sqrt(16)
      puts 15.gcd(20)
      puts 100.clamp(0, 50)
    RUBY

    "range_and_enumerable" => <<~RUBY,
      r = (1..10)
      puts r.to_a.inspect
      puts r.cover?(5)
      puts r.step(2).to_a.inspect
      puts (1..).take(5).inspect
    RUBY

    "struct_and_class" => <<~RUBY,
      Point = Struct.new(:x, :y) do
        def norm
          Math.sqrt(x**2 + y**2)
        end
      end
      p = Point.new(3, 4)
      puts p.norm
      puts p.to_h.inspect
    RUBY

    "time_and_comparable" => <<~RUBY,
      t = Time.utc(2026, 10, 8, 12, 0, 0)
      puts t.iso8601 rescue puts t.strftime("%Y-%m-%dT%H:%M:%SZ")
      puts t.year
      puts t.month
      puts t.day
    RUBY

    "exception_handling" => <<~RUBY,
      begin
        1 / 0
      rescue ZeroDivisionError => e
        puts e.class.name
      ensure
        puts "cleaned_up"
      end
    RUBY
  }.freeze

  def initialize(options)
    @options = options
    @ruby = options[:ruby] || ENV["RUBY"] || RbConfig.ruby
    @reference = options[:reference] || ENV["REF_RUBY"] || RbConfig.ruby
    @snippets_dir = options[:snippets_dir] || File.expand_path("snippets", __dir__)
    @verbose = options[:verbose]
  end

  def run
    snippets = load_snippets
    puts "Running differential snippet tests against Ruby target: '#{@ruby}' and Reference: '#{@reference}'" if @verbose || @options[:check]

    passed = 0
    failed = 0
    failures = []

    snippets.each do |name, code|
      result = test_snippet(name, code)
      if result[:success]
        passed += 1
        print "." if !@verbose && @options[:check]
      else
        failed += 1
        failures << result
        print "F" if !@verbose && @options[:check]
      end
    end

    puts "" if !@verbose && @options[:check]

    if failed > 0
      puts "\n--- Differential Divergences Detected (#{failed}/#{snippets.size}) ---"
      failures.each do |f|
        puts "\nSnippet [#{f[:name]}] divergence:"
        puts "Target Out:\n#{f[:target_out]}"
        puts "Reference Out:\n#{f[:ref_out]}"
        puts "Target Err:\n#{f[:target_err]}"
        puts "Reference Err:\n#{f[:ref_err]}"
        puts "Target Exit: #{f[:target_status]}, Reference Exit: #{f[:ref_status]}"
      end
    else
      puts "All #{passed} differential snippet tests passed with zero divergence."
    end

    if @options[:output]
      report = {
        total: snippets.size,
        passed: passed,
        failed: failed,
        failures: failures.map { |f| { name: f[:name], target_status: f[:target_status], ref_status: f[:ref_status] } }
      }
      File.write(@options[:output], JSON.pretty_generate(report))
    end

    failed == 0
  end

  private

  def load_snippets
    snippets = BUILTIN_SNIPPETS.dup
    if File.directory?(@snippets_dir)
      Dir.glob(File.join(@snippets_dir, "*.rb")).each do |file|
        name = File.basename(file, ".rb")
        snippets[name] = File.read(file)
      end
    end
    snippets
  end

  def test_snippet(name, code)
    temp = Tempfile.new(["snippet_#{name}", ".rb"])
    temp.write(code)
    temp.close

    target_out, target_err, target_status = exec_ruby(@ruby, temp.path)
    ref_out, ref_err, ref_status = exec_ruby(@reference, temp.path)

    success = (target_out == ref_out) &&
              (target_err == ref_err) &&
              (target_status.exitstatus == ref_status.exitstatus)

    {
      name: name,
      success: success,
      target_out: target_out,
      ref_out: ref_out,
      target_err: target_err,
      ref_err: ref_err,
      target_status: target_status.exitstatus,
      ref_status: ref_status.exitstatus
    }
  ensure
    temp&.unlink
  end

  def exec_ruby(ruby_bin, script_path)
    cmd = "#{ruby_bin} #{Shellwords.escape(script_path)}"
    stdout, stderr, status = Open3.capture3(cmd)
    [stdout, stderr, status]
  end
end

if __FILE__ == $0
  options = { check: false, verbose: false }
  OptionParser.new do |opts|
    opts.banner = "Usage: ruby tool/ruby_on_rust/differential_runner.rb [options]"

    opts.on("--ruby PATH", "Ruby target executable command") { |v| options[:ruby] = v }
    opts.on("--reference PATH", "Reference Ruby executable command") { |v| options[:reference] = v }
    opts.on("--snippets-dir DIR", "Directory containing snippet files") { |v| options[:snippets_dir] = v }
    opts.on("--check", "Run differential checks and return non-zero on failure") { options[:check] = true }
    opts.on("--output FILE", "Path to save JSON execution summary") { |v| options[:output] = v }
    opts.on("-v", "--verbose", "Enable verbose log output") { options[:verbose] = true }
    opts.on("-h", "--help", "Show help message") do
      puts opts
      exit 0
    end
  end.parse!

  runner = DifferentialRunner.new(options)
  success = runner.run
  exit(success ? 0 : 1)
end
