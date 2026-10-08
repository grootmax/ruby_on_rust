#!/usr/bin/env ruby
# frozen_string_literal: true

require 'optparse'
require 'fileutils'
require 'tempfile'
require 'open3'
require 'timeout'

require_relative 'type_generators/generator_factory'
require_relative 'shrinker'

module RubyOnRust
  class Result
    attr_accessor :exit_code, :stdout, :stderr, :inspect_str, :exception_class, :exception_message, :timed_out

    def initialize
      @exit_code = 0
      @stdout = ''
      @stderr = ''
      @inspect_str = nil
      @exception_class = nil
      @exception_message = nil
      @timed_out = false
    end
  end

  class SubprocessEvaluator
    attr_reader :c_ruby, :rust_ruby, :timeout_sec, :dry_run, :verbose

    def initialize(c_ruby:, rust_ruby:, timeout_sec: 5, dry_run: false, verbose: false)
      @c_ruby = c_ruby
      @rust_ruby = rust_ruby
      @timeout_sec = timeout_sec
      @dry_run = dry_run
      @verbose = verbose
    end

    def divergent?(snippet)
      return false if dry_run

      res_c = eval_snippet(c_ruby, snippet)
      res_rust = eval_snippet(rust_ruby, snippet)

      divergent_results?(res_c, res_rust)
    end

    def evaluate_both(snippet)
      if dry_run
        r = Result.new
        r.inspect_str = "dry_run_inspect"
        return [r, r]
      end

      [eval_snippet(c_ruby, snippet), eval_snippet(rust_ruby, snippet)]
    end

    def divergent_results?(res_c, res_rust)
      return true if res_c.timed_out != res_rust.timed_out
      return true if res_c.exit_code != res_rust.exit_code
      return true if res_c.exception_class != res_rust.exception_class
      return true if res_c.inspect_str != res_rust.inspect_str
      return true if res_c.stdout != res_rust.stdout
      return true if normalize_stderr(res_c.stderr) != normalize_stderr(res_rust.stderr)

      false
    end

    def eval_snippet(ruby_bin, snippet)
      res = Result.new
      tempfile = Tempfile.new(['fuzz_', '.rb'])
      tempfile.write(wrap_snippet(snippet))
      tempfile.flush
      tempfile.close

      cmd = [ruby_bin, '-w', tempfile.path]

      begin
        stdout_str, stderr_str, status = Open3.capture3(*cmd, timeout: timeout_sec)
        res.exit_code = status.exitstatus || (status.termsig ? 128 + status.termsig : 1)
        res.stderr = stderr_str

        parse_stdout(stdout_str, res)
      rescue Timeout::Error
        res.timed_out = true
        res.exit_code = 124
        res.stderr = 'Execution timed out'
      rescue StandardError => e
        res.exit_code = 1
        res.stderr = "Evaluator error: #{e.message}"
      ensure
        tempfile.unlink
      end

      res
    end

    private

    def wrap_snippet(snippet)
      <<~RUBY
        # frozen_string_literal: false
        $VERBOSE = true

        begin
          __fuzz_val__ = begin
            #{snippet}
          end
          $stdout.puts "===FUZZ_INSPECT_START==="
          $stdout.puts __fuzz_val__.inspect
          $stdout.puts "===FUZZ_INSPECT_END==="
        rescue Exception => e
          $stdout.puts "===FUZZ_EXCEPTION_START==="
          $stdout.puts "\#{e.class.name}: \#{e.message}"
          $stdout.puts "===FUZZ_EXCEPTION_END==="
        end
      RUBY
    end

    def parse_stdout(stdout_str, res)
      if stdout_str.include?('===FUZZ_INSPECT_START===')
        parts = stdout_str.split("===FUZZ_INSPECT_START===\n")
        user_stdout = parts[0] || ''
        inspect_part = parts[1] ? parts[1].split("===FUZZ_INSPECT_END===")[0] : nil
        res.stdout = user_stdout.strip
        res.inspect_str = inspect_part ? inspect_part.chomp : nil
      elsif stdout_str.include?('===FUZZ_EXCEPTION_START===')
        parts = stdout_str.split("===FUZZ_EXCEPTION_START===\n")
        user_stdout = parts[0] || ''
        exc_part = parts[1] ? parts[1].split("===FUZZ_EXCEPTION_END===")[0] : nil
        res.stdout = user_stdout.strip
        if exc_part
          line = exc_part.chomp
          if line =~ /^([^:]+):\s*(.*)$/
            res.exception_class = Regexp.last_match(1)
            res.exception_message = Regexp.last_match(2)
          else
            res.exception_class = 'Exception'
            res.exception_message = line
          end
        end
      else
        res.stdout = stdout_str.strip
      end
    end

    def normalize_stderr(stderr_str)
      return '' if stderr_str.nil?

      stderr_str.gsub(/\/.*fuzz_[^\s:]+/, '<eval>')
                .gsub(/line \d+/, 'line <N>')
    end
  end

  class FuzzRunner
    attr_reader :options, :evaluator, :shrinker

    def initialize(args = ARGV)
      @options = parse_options(args)
      @evaluator = SubprocessEvaluator.new(
        c_ruby: options[:c_ruby],
        rust_ruby: options[:rust_ruby],
        timeout_sec: options[:timeout],
        dry_run: options[:dry_run],
        verbose: options[:verbose]
      )
      @shrinker = Shrinker.new(@evaluator)
    end

    def run
      random = Random.new(options[:seed])
      generators = TypeGenerators::GeneratorFactory.create(options[:target], random)

      total_iterations = options[:iters] * generators.length
      total_divergences = 0

      puts "Starting Fuzz Runner:"
      puts "  Target(s): #{options[:target]}"
      puts "  Iterations per target: #{options[:iters]}"
      puts "  Dry Run: #{options[:dry_run]}"
      puts "  CRuby Binary: #{options[:c_ruby]}"
      puts "  Ruby on Rust Binary: #{options[:rust_ruby]}"
      puts "  Seed: #{options[:seed]}"
      puts "  Output Dir: #{options[:output_dir]}"
      puts "----------------------------------------"

      generators.each do |gen|
        target_type = gen.target_name
        puts "Fuzzing target: #{target_type} (#{options[:iters]} iterations)..." if options[:verbose]

        options[:iters].times do |i|
          snippet = gen.generate_snippet

          if evaluator.divergent?(snippet)
            total_divergences += 1
            puts "DIVERGENCE DETECTED [#{target_type} - iter #{i + 1}]!"

            res_c, res_rust = evaluator.evaluate_both(snippet)
            log_divergence(res_c, res_rust) if options[:verbose]

            shrunk_snippet = shrinker.shrink(snippet, target_type)
            test_file = save_test_case(target_type, shrunk_snippet)
            puts "Saved minimal test case to #{test_file}"
          end
        end
      end

      puts "----------------------------------------"
      puts "Fuzz Run Completed:"
      puts "  Total Iterations: #{total_iterations}"
      puts "  Divergences Found: #{total_divergences}"

      total_divergences
    end

    private

    def parse_options(args)
      opts = {
        target: 'all',
        iters: 100,
        c_ruby: ENV['CRUBY'] || 'ruby',
        rust_ruby: ENV['RUST_RUBY'] || './miniruby',
        timeout: 5,
        seed: Random.new_seed,
        dry_run: false,
        output_dir: File.expand_path('../../test/ruby_on_rust', __dir__),
        verbose: false
      }

      OptionParser.new do |parser|
        parser.banner = "Usage: fuzz_runner.rb [options]"

        parser.on("-t", "--target TYPE", "Target core type to fuzz (e.g. String, Array, Hash, Integer, Float, Range, Struct, Time, Comparable, Enumerable, or all)") do |v|
          opts[:target] = v
        end

        parser.on("-n", "--iters N", Integer, "Number of fuzz iterations per target (default: 100)") do |v|
          opts[:iters] = v
        end

        parser.on("--c-ruby PATH", "Path to reference CRuby binary (default: 'ruby')") do |v|
          opts[:c_ruby] = v
        end

        parser.on("--rust-ruby PATH", "Path to Ruby on Rust binary (default: './miniruby')") do |v|
          opts[:rust_ruby] = v
        end

        parser.on("--timeout SEC", Integer, "Timeout in seconds per snippet (default: 5)") do |v|
          opts[:timeout] = v
        end

        parser.on("--seed SEED", Integer, "Random seed (default: random)") do |v|
          opts[:seed] = v
        end

        parser.on("--dry-run", "Dry run mode (generates snippets without running subprocesses)") do
          opts[:dry_run] = true
        end

        parser.on("--output-dir PATH", "Output directory for test cases (default: 'test/ruby_on_rust')") do |v|
          opts[:output_dir] = v
        end

        parser.on("-v", "--verbose", "Verbose output") do
          opts[:verbose] = true
        end
      end.parse!(args)

      opts
    end

    def log_divergence(res_c, res_rust)
      puts "--- CRuby Result ---"
      puts "Exit: #{res_c.exit_code}, Exception: #{res_c.exception_class}: #{res_c.exception_message}"
      puts "Inspect: #{res_c.inspect_str}"
      puts "--- Ruby on Rust Result ---"
      puts "Exit: #{res_rust.exit_code}, Exception: #{res_rust.exception_class}: #{res_rust.exception_message}"
      puts "Inspect: #{res_rust.inspect_str}"
      puts "---------------------"
    end

    def save_test_case(target_type, snippet)
      FileUtils.mkdir_p(options[:output_dir])
      file_name = "test_fuzz_#{target_type.downcase}.rb"
      file_path = File.join(options[:output_dir], file_name)

      class_name = "TestFuzz#{target_type.capitalize}"

      if File.exist?(file_path)
        content = File.read(file_path)
        existing_tests = content.scan(/def test_fuzz_divergence_(\d+)/).flatten.map(&:to_i)
        next_id = (existing_tests.max || 0) + 1

        new_test_method = <<~RUBY

          def test_fuzz_divergence_#{next_id}
        #{indent(snippet, 4)}
          end
        RUBY

        if content.rindex(/^end\b/)
          end_pos = content.rindex(/^end\b/)
          updated_content = content[0...end_pos] + new_test_method + "end\n"
          File.write(file_path, updated_content)
        else
          File.write(file_path, content + new_test_method)
        end
      else
        new_file_content = <<~RUBY
          # frozen_string_literal: true

          require 'test/unit'

          class #{class_name} < Test::Unit::TestCase
            def test_fuzz_divergence_1
          #{indent(snippet, 4)}
            end
          end
        RUBY
        File.write(file_path, new_file_content)
      end

      file_path
    end

    def indent(str, spaces)
      str.lines.map { |l| " " * spaces + l }.join
    end
  end
end

if __FILE__ == $0
  runner = RubyOnRust::FuzzRunner.new
  runner.run
end
