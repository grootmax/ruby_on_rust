#!/usr/bin/env ruby
# frozen_string_literal: true

require "optparse"
require "open3"
require "tempfile"
require "json"
require "shellwords"
require "rbconfig"

class EdgeFuzzer
  CORE_CLASSES = %w[String Array Hash Integer Float Range Struct Time Comparable Enumerable].freeze

  def initialize(options)
    @options = options
    @iterations = options[:iterations] || 100
    @seed = options[:seed] || 42
    @classes = options[:classes] ? options[:classes].split(",") : CORE_CLASSES
    @ruby = options[:ruby] || ENV["RUBY"] || RbConfig.ruby
    @reference = options[:reference] || ENV["REF_RUBY"] || RbConfig.ruby
    @verbose = options[:verbose]
    @prng = Random.new(@seed)
  end

  def run
    results = {}
    total_cases = 0
    passed_cases = 0
    failed_cases = 0
    failing_snippets = []

    puts "Starting edge-case fuzzing across classes: #{@classes.join(', ')}" if @verbose || @options[:check]
    puts "Iterations per class: #{@iterations}, Seed: #{@seed}" if @verbose || @options[:check]

    @classes.each do |klass_name|
      klass_results = fuzz_class(klass_name)
      results[klass_name] = klass_results

      total_cases += klass_results[:total]
      passed_cases += klass_results[:passed]
      failed_cases += klass_results[:failed]
      failing_snippets.concat(klass_results[:failures]) if klass_results[:failures]
    end

    summary = {
      seed: @seed,
      iterations: @iterations,
      classes: @classes,
      total_cases: total_cases,
      passed_cases: passed_cases,
      failed_cases: failed_cases,
      class_breakdown: results.transform_values { |v| { total: v[:total], passed: v[:passed], failed: v[:failed] } }
    }

    if @options[:output]
      File.write(@options[:output], JSON.pretty_generate({ summary: summary, failing_snippets: failing_snippets }))
    end

    if failed_cases > 0
      puts "\n--- Fuzzing Divergences Found (#{failed_cases}/#{total_cases}) ---"
      failing_snippets.each do |f|
        puts "\nClass [#{f[:class]}] Case [#{f[:case_id]}]:"
        puts "Code:\n#{f[:code]}"
        puts "Target Out:\n#{f[:target_out]}"
        puts "Ref Out:\n#{f[:ref_out]}"
      end
    else
      puts "Edge fuzzing completed successfully across all 10 core classes with 0 divergences (#{passed_cases}/#{total_cases} passed)."
    end

    failed_cases == 0
  end

  private

  def fuzz_class(klass_name)
    total = 0
    passed = 0
    failed = 0
    failures = []

    generator_method = "generate_#{klass_name.downcase}_cases"
    cases = respond_to?(generator_method, true) ? send(generator_method) : []

    cases.each_with_index do |code, idx|
      total += 1
      res = run_snippet(code)
      if res[:success]
        passed += 1
      else
        failed += 1
        failures << {
          class: klass_name,
          case_id: idx + 1,
          code: code,
          target_out: res[:target_out],
          ref_out: res[:ref_out],
          target_err: res[:target_err],
          ref_err: res[:ref_err]
        }
      end
    end

    { total: total, passed: passed, failed: failed, failures: failures }
  end

  def run_snippet(code)
    temp = Tempfile.new(["fuzz_snippet", ".rb"])
    temp.write(code)
    temp.close

    target_out, target_err, target_status = exec_ruby(@ruby, temp.path)
    ref_out, ref_err, ref_status = exec_ruby(@reference, temp.path)

    success = (target_out == ref_out) &&
              (target_err == ref_err) &&
              (target_status.exitstatus == ref_status.exitstatus)

    {
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

  # --- Case Generators for the 10 Core Classes ---

  def generate_string_cases
    [
      'puts "".length',
      'puts "hello \u{1F600} world".chars.inspect',
      'puts "a,b,c,d,e".split(",").inspect',
      'puts "\x00\xFF\xFE".b.bytes.inspect',
      'puts ("a" * 1000).slice(100..200).length',
      'puts "Ruby on Rust".encode("UTF-8").encoding.name',
      'puts "  trim me \n\r ".strip.inspect',
      'puts "abc".tr("a-z", "A-Z")'
    ]
  end

  def generate_array_cases
    [
      'puts [].inspect',
      'puts [1, [2, [3, 4]], 5].flatten.inspect',
      'puts [1, nil, 2, nil, 3].compact.inspect',
      'puts [1, 2, 2, 3, 1].uniq.inspect',
      'puts [1, 2, 3, 4].rotate(2).inspect',
      'puts [1, 2, 3].permutation(2).to_a.inspect',
      'puts [1, 2, 3, 4].combination(2).to_a.inspect',
      'puts ["b", "a", "c"].sort.inspect'
    ]
  end

  def generate_hash_cases
    [
      'puts {}.inspect',
      'h = { a: 1, "b" => 2, 3 => 4 }; puts h.keys.inspect; puts h.values.inspect',
      'puts { a: 1, b: 2 }.transform_values { |v| v * 10 }.inspect',
      'puts { a: 1 }.merge({ b: 2, a: 10 }).inspect',
      'puts { a: 1, b: nil }.compact.inspect',
      'puts { a: 1, b: 2 }.invert.inspect'
    ]
  end

  def generate_integer_cases
    [
      'puts 0',
      'puts (2**63 - 1).to_s',
      'puts (-2**63).to_s',
      'puts (2**128).to_s',
      'puts 100.gcd(45)',
      'puts 12.lcm(18)',
      'puts 12345.digits.inspect',
      'puts 25.clamp(10, 20)',
      'puts (0b101010 & 0b110011).to_s(2)'
    ]
  end

  def generate_float_cases
    [
      'puts 0.0.to_s',
      'puts (-0.0).to_s',
      'puts (1.0 / 0.0).infinite?',
      'puts (-1.0 / 0.0).infinite?',
      'puts (0.0 / 0.0).nan?',
      'puts 3.14159.round(2)',
      'puts 3.99.floor',
      'puts 3.01.ceil',
      'puts Float::EPSILON.positive?'
    ]
  end

  def generate_range_cases
    [
      'puts (1..10).to_a.inspect',
      'puts (1...10).to_a.inspect',
      'puts ("a".."e").to_a.inspect',
      'puts (1..10).cover?(5)',
      'puts (1..10).step(3).to_a.inspect',
      'puts (1..).take(5).inspect'
    ]
  end

  def generate_struct_cases
    [
      'Person = Struct.new(:name, :age); p = Person.new("Alice", 30); puts p.name; puts p.age',
      'S = Struct.new(:a, :b, keyword_init: true); s = S.new(a: 1, b: 2); puts s.to_h.inspect',
      'Point = Struct.new(:x, :y); p = Point.new(10, 20); puts p.members.inspect; puts p.values.inspect'
    ]
  end

  def generate_time_cases
    [
      't = Time.utc(2026, 10, 8, 12, 0, 0); puts t.year; puts t.month; puts t.day; puts t.wday; puts t.yday',
      't = Time.at(0).utc; puts t.iso8601 rescue puts t.strftime("%Y-%m-%dT%H:%M:%SZ")',
      't = Time.utc(2026, 1, 1) + 86400; puts t.day'
    ]
  end

  def generate_comparable_cases
    [
      <<~RUBY
        class Card
          include Comparable
          attr_reader :rank
          def initialize(rank); @rank = rank; end
          def <=>(other); rank <=> other.rank; end
        end
        c1 = Card.new(5); c2 = Card.new(10)
        puts c1 < c2
        puts c1.between?(Card.new(1), Card.new(8))
        puts c2.clamp(Card.new(1), Card.new(8)).rank
      RUBY
    ]
  end

  def generate_enumerable_cases
    [
      'puts (1..5).map { |n| n * 2 }.inspect',
      'puts [1, 2, 3, 4, 5].select(&:odd?).inspect',
      'puts [1, 2, 3, 4, 5].reduce(0, :+)',
      'puts ["apple", "banana", "cherry"].group_by(&:length).inspect',
      'puts ["a", "b", "a", "c", "b", "a"].tally.inspect',
      'puts [1, 2].zip(["a", "b"]).inspect'
    ]
  end
end

if __FILE__ == $0
  options = { iterations: 100, seed: 42 }
  OptionParser.new do |opts|
    opts.banner = "Usage: ruby tool/ruby_on_rust/edge_fuzzer.rb [options]"

    opts.on("-n", "--iterations N", Integer, "Number of test iterations per class") { |v| options[:iterations] = v }
    opts.on("-s", "--seed N", Integer, "PRNG seed") { |v| options[:seed] = v }
    opts.on("-c", "--classes LIST", "Comma-separated list of core classes to test") { |v| options[:classes] = v }
    opts.on("--ruby PATH", "Ruby target executable command") { |v| options[:ruby] = v }
    opts.on("--reference PATH", "Reference Ruby executable command") { |v| options[:reference] = v }
    opts.on("--output FILE", "Path to write output JSON results") { |v| options[:output] = v }
    opts.on("-v", "--verbose", "Enable verbose log output") { options[:verbose] = true }
    opts.on("-h", "--help", "Show help message") do
      puts opts
      exit 0
    end
  end.parse!

  fuzzer = EdgeFuzzer.new(options)
  success = fuzzer.run
  exit(success ? 0 : 1)
end
