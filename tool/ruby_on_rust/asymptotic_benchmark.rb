#!/usr/bin/env ruby
# frozen_string_literal: true

# Native Ordinary Least Squares (OLS) Complexity Benchmark Harness
# Measures algorithmic time complexity scaling across variable geometric input sizes (N in [10^1, 10^6]).

require 'optparse'
require 'shellwords'

module AsymptoticBenchmark
  DEFAULT_SIZES = [10, 100, 1_000, 10_000, 100_000, 1_000_000].freeze
  DEFAULT_TOLERANCE = 0.15
  DEFAULT_WARMUP_RUNS = 3
  DEFAULT_SAMPLE_RUNS = 5

  # Pure Ruby simple YAML parser fallback when psych/yaml extension is unavailable
  def self.parse_manifest(content)
    begin
      require 'yaml'
      return YAML.safe_load(content) || {}
    rescue LoadError, StandardError
      data = {}
      current_key = nil
      current_val = []

      content.each_line do |line|
        line_rstrip = line.rstrip
        next if line_rstrip.empty? || line_rstrip.start_with?('#')

        if line =~ /^([a_zA-Z0-9_]+):\s*(.*)$/
          if current_key
            data[current_key] = current_val.join("\n").strip
            current_val = []
          end
          key = Regexp.last_match(1)
          val = Regexp.last_match(2).strip
          if val == '|' || val == '>'
            current_key = key
          else
            val = val.sub(/^["']/, '').sub(/["']$/, '') if val.start_with?('"') || val.start_with?("'")
            data[key] = val
            current_key = nil
          end
        elsif current_key && line =~ /^\s+(.*)$/
          current_val << Regexp.last_match(1)
        end
      end

      if current_key
        data[current_key] = current_val.join("\n").strip
      end

      data
    end
  end

  class Manifest
    attr_reader :name, :target_class, :method_name, :expected_complexity, :expected_exponent, :tolerance, :setup_code, :bench_code

    def initialize(file_path, default_tolerance = DEFAULT_TOLERANCE)
      content = File.read(file_path)
      raw = AsymptoticBenchmark.parse_manifest(content)

      @name = raw['name'] || File.basename(file_path, '.*')
      @target_class = raw['target_class'] || 'Unknown'
      @method_name = raw['method'] || 'unknown'
      @expected_complexity = raw['expected_complexity'] || 'O(1)'
      
      if raw.key?('expected_exponent')
        @expected_exponent = raw['expected_exponent'].to_f
      else
        @expected_exponent = parse_complexity_to_exponent(@expected_complexity)
      end

      @tolerance = raw.key?('tolerance') ? raw['tolerance'].to_f : default_tolerance
      @setup_code = raw['setup'] || ''
      @bench_code = raw['code'] || raw['benchmark'] || ''
    end

    private

    def parse_complexity_to_exponent(complexity_str)
      case complexity_str.to_s.gsub(/\s+/, '').downcase
      when 'o(1)', 'constant'
        0.0
      when 'o(n)', 'linear'
        1.0
      when 'o(nlogn)', 'o(n*logn)'
        1.0
      else
        0.0
      end
    end
  end

  class RegressionEngine
    Result = Struct.new(:name, :expected_exponent, :empirical_exponent, :tolerance, :passed, :r_squared, :points, :details, keyword_init: true)

    # Calculates OLS log-log regression slope alpha
    def self.compute_ols(data_points)
      valid_points = data_points.select { |_n, t| t && t > 0 }
      return { alpha: 0.0, r_squared: 0.0 } if valid_points.size < 2

      xs = valid_points.map { |n, _t| Math.log(n) }
      ys = valid_points.map { |_n, t| Math.log(t) }

      n_count = xs.size
      mean_x = xs.sum / n_count.to_f
      mean_y = ys.sum / n_count.to_f

      num = xs.zip(ys).map { |x, y| (x - mean_x) * (y - mean_y) }.sum
      den = xs.map { |x| (x - mean_x)**2 }.sum

      alpha = den.zero? ? 0.0 : num / den.to_f

      y_pred = xs.map { |x| mean_y + alpha * (x - mean_x) }
      ss_res = ys.zip(y_pred).map { |y, yp| (y - yp)**2 }.sum
      ss_tot = ys.map { |y| (y - mean_y)**2 }.sum
      r_squared = ss_tot.zero? ? 1.0 : [0.0, 1.0 - (ss_res / ss_tot)].max

      { alpha: alpha, r_squared: r_squared }
    end
  end

  class Runner
    def initialize(options = {})
      @options = options
      @sizes = options[:sizes] || DEFAULT_SIZES
      @tolerance = options[:tolerance] || DEFAULT_TOLERANCE
      @verbose = options[:verbose] || false
      @ruby_cmd = options[:ruby]
    end

    def run_manifest(manifest)
      puts "Running benchmark: #{manifest.name} (#{manifest.target_class}##{manifest.method_name})..." if @verbose

      data_points = []
      @sizes.each do |size|
        time_per_op = measure_size(manifest.setup_code, manifest.bench_code, size)
        data_points << [size, time_per_op]
        puts "  N=#{size}: #{'%.9f' % time_per_op} s/op" if @verbose
      end

      ols = RegressionEngine.compute_ols(data_points)
      alpha = ols[:alpha]
      passed = (alpha - manifest.expected_exponent).abs <= manifest.tolerance

      RegressionEngine::Result.new(
        name: manifest.name,
        expected_exponent: manifest.expected_exponent,
        empirical_exponent: alpha,
        tolerance: manifest.tolerance,
        passed: passed,
        r_squared: ols[:r_squared],
        points: data_points,
        details: "#{manifest.target_class}##{manifest.method_name} expected #{manifest.expected_complexity}"
      )
    end

    def run_synthetic
      puts "Executing synthetic benchmark self-test..." if @verbose

      # 1. Synthetic O(1)
      o1_points = @sizes.map do |size|
        a = Array.new(size) { 0 }
        t_op = measure_block(calibrated_loop_count: 100_000) { a[0] }
        [size, t_op]
      end
      o1_ols = RegressionEngine.compute_ols(o1_points)
      o1_passed = (o1_ols[:alpha] - 0.0).abs <= @tolerance

      o1_res = RegressionEngine::Result.new(
        name: "synthetic_O(1)",
        expected_exponent: 0.0,
        empirical_exponent: o1_ols[:alpha],
        tolerance: @tolerance,
        passed: o1_passed,
        r_squared: o1_ols[:r_squared],
        points: o1_points,
        details: "Synthetic O(1) constant scaling"
      )

      # 2. Synthetic O(N)
      oN_points = @sizes.map do |size|
        t_op = measure_block(calibrated_target_sec: 0.005) do
          i = 0
          while i < size
            i += 1
          end
        end
        [size, t_op]
      end
      oN_ols = RegressionEngine.compute_ols(oN_points)
      oN_passed = (oN_ols[:alpha] - 1.0).abs <= @tolerance

      oN_res = RegressionEngine::Result.new(
        name: "synthetic_O(N)",
        expected_exponent: 1.0,
        empirical_exponent: oN_ols[:alpha],
        tolerance: @tolerance,
        passed: oN_passed,
        r_squared: oN_ols[:r_squared],
        points: oN_points,
        details: "Synthetic O(N) linear scaling"
      )

      [o1_res, oN_res]
    end

    private

    def measure_size(setup_code, bench_code, size)
      script = <<~RUBY
        n = #{size}
        #{setup_code.empty? ? 'target = nil' : "target = (#{setup_code})"}
        
        # Warmup and calibration
        calib_start = Process.clock_gettime(Process::CLOCK_MONOTONIC)
        10.times do
          #{bench_code_bound(bench_code)}
        end
        calib_dt = [Process.clock_gettime(Process::CLOCK_MONOTONIC) - calib_start, 1e-9].max
        per_iter = calib_dt / 10.0
        target_sec = 0.005
        k = (target_sec / per_iter).to_i.clamp(10, 500_000)

        # Warmup runs
        3.times do
          k.times do
            #{bench_code_bound(bench_code)}
          end
        end

        # Sample measurement runs with GC pause trimming
        samples = []
        5.times do
          GC.disable
          t0 = Process.clock_gettime(Process::CLOCK_MONOTONIC)
          k.times do
            #{bench_code_bound(bench_code)}
          end
          t1 = Process.clock_gettime(Process::CLOCK_MONOTONIC)
          GC.enable
          samples << (t1 - t0) / k.to_f
        end

        samples.sort!
        trimmed = samples[1..-2] || samples
        avg = trimmed.sum / trimmed.size.to_f
        avg
      RUBY

      run_ruby_script(script)
    end

    def measure_block(calibrated_loop_count: nil, calibrated_target_sec: 0.005, &block)
      if calibrated_loop_count
        k = calibrated_loop_count
      else
        calib_t0 = Process.clock_gettime(Process::CLOCK_MONOTONIC)
        10.times(&block)
        calib_dt = [Process.clock_gettime(Process::CLOCK_MONOTONIC) - calib_t0, 1e-9].max
        k = (calibrated_target_sec / (calib_dt / 10.0)).to_i.clamp(5, 500_000)
      end

      # Warmup
      3.times { k.times(&block) }

      # Samples
      samples = []
      5.times do
        GC.disable
        t0 = Process.clock_gettime(Process::CLOCK_MONOTONIC)
        k.times(&block)
        t1 = Process.clock_gettime(Process::CLOCK_MONOTONIC)
        GC.enable
        samples << (t1 - t0) / k.to_f
      end

      samples.sort!
      trimmed = samples[1..-2] || samples
      trimmed.sum / trimmed.size.to_f
    end

    def bench_code_bound(code)
      if code.include?('a.')
        "a = target; #{code}"
      elsif code.include?('h.')
        "h = target; #{code}"
      elsif code.include?('s.')
        "s = target; #{code}"
      else
        "a = h = s = target; #{code}"
      end
    end

    def run_ruby_script(script)
      if @ruby_cmd && !@ruby_cmd.empty?
        sub_script = "#{script}\nputs avg"
        cmd_args = Shellwords.split(@ruby_cmd) + ['-e', sub_script]
        output = IO.popen(cmd_args, 'r') { |io| io.read }
        output.to_f
      else
        eval(script).to_f
      end
    end
  end
end

def main
  options = {
    manifests_dir: File.expand_path('../../benchmark/complexity', __dir__),
    tolerance: AsymptoticBenchmark::DEFAULT_TOLERANCE,
    sizes: AsymptoticBenchmark::DEFAULT_SIZES,
    verbose: false,
    synthetic: false,
    ruby: nil
  }

  OptionParser.new do |opts|
    opts.banner = "Usage: asymptotic_benchmark.rb [options]"

    opts.on("--synthetic", "Run synthetic benchmark self-test for O(1) and O(N)") do
      options[:synthetic] = true
    end

    opts.on("--manifests=DIR", "Path to benchmark complexity manifests directory") do |dir|
      options[:manifests_dir] = File.expand_path(dir)
    end

    opts.on("--ruby=PATH", "Ruby binary path to execute benchmarks") do |ruby|
      options[:ruby] = ruby
    end

    opts.on("--tolerance=NUM", Float, "Tolerance threshold for empirical exponent (default: 0.15)") do |tol|
      options[:tolerance] = tol
    end

    opts.on("--sizes=SIZES", "Comma-separated input sizes (default: 10,100,1000,10000,100000,1000000)") do |s|
      options[:sizes] = s.split(',').map(&:to_i)
    end

    opts.on("-v", "--verbose", "Output detailed benchmark timing metrics") do
      options[:verbose] = true
    end

    opts.on("-h", "--help", "Show help message") do
      puts opts
      exit 0
    end
  end.parse!

  runner = AsymptoticBenchmark::Runner.new(options)
  results = []

  if options[:synthetic]
    results = runner.run_synthetic
  else
    manifest_files = Dir.glob(File.join(options[:manifests_dir], '*.{yml,yaml}')).sort
    if manifest_files.empty?
      puts "No benchmark manifests found in #{options[:manifests_dir]}"
      exit 1
    end

    manifest_files.each do |mfile|
      manifest = AsymptoticBenchmark::Manifest.new(mfile, options[:tolerance])
      results << runner.run_manifest(manifest)
    end
  end

  # Print Summary Table
  puts "\n" + "=" * 78
  puts "ASYMPTOTIC COMPLEXITY BENCHMARK REPORT"
  puts "=" * 78
  puts sprintf("%-22s %-15s %-18s %-10s %-8s", "Benchmark", "Target Exponent", "Empirical Exponent", "Tolerance", "Status")
  puts "-" * 78

  all_passed = true
  results.each do |res|
    status_str = res.passed ? "PASS" : "FAIL"
    all_passed &&= res.passed

    puts sprintf(
      "%-22s %-15.2f %-18.3f +/-%-8.2f [%s]",
      res.name,
      res.expected_exponent,
      res.empirical_exponent,
      res.tolerance,
      status_str
    )

    if options[:verbose]
      puts "  Details: #{res.details}"
      puts "  R^2: #{'%.4f' % res.r_squared}"
      res.points.each do |n, t|
        puts "    N=#{n}: #{'%.9f' % t} s"
      end
    end
  end
  puts "=" * 78

  if all_passed
    puts "\nSUCCESS: All benchmarks passed expected asymptotic complexity bounds."
    exit 0
  else
    puts "\nFAILURE: Algorithmic complexity regression detected!"
    exit 1
  end
end

if __FILE__ == $0
  main
end
