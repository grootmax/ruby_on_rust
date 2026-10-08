# frozen_string_literal: true

require 'benchmark_driver'

class BenchmarkDriver::Output::Parity
  OPTIONS = {
    threshold: ['--output-threshold THRESHOLD', Float, 'Minimum parity threshold ratio (default: 1.0)'],
  }.freeze

  # @param [Array<BenchmarkDriver::Metric>] metrics
  # @param [Array<BenchmarkDriver::Job>] jobs
  # @param [Array<BenchmarkDriver::Context>] contexts
  # @param [Hash{ Symbol => Object }] options
  def initialize(metrics:, jobs:, contexts:, options: {})
    @metrics = metrics
    @jobs = jobs
    @contexts = contexts
    @options = options || {}

    threshold_val = @options[:threshold] || ENV['PARITY_THRESHOLD']
    @threshold = threshold_val ? threshold_val.to_f : 1.0

    @job_context_result = Hash.new { |h, k| h[k] = {} }
  end

  def with_warmup(&block)
    without_stdout_buffering do
      $stdout.puts 'Warming up --------------------------------------'
      block.call
    end
  end

  def with_benchmark(&block)
    @with_benchmark = true
    result = without_stdout_buffering do
      $stdout.puts 'Calculating -------------------------------------'
      block.call
    end
    print_parity_report
    result
  ensure
    @with_benchmark = false
  end

  # @param [BenchmarkDriver::Job] job
  def with_job(job, &block)
    @job = job
    block.call
  end

  # @param [BenchmarkDriver::Context] context
  def with_context(context, &block)
    @context = context
    block.call
  end

  # @param [BenchmarkDriver::Result] result
  def report(result)
    if defined?(@job_context_result) && @job && @context
      @job_context_result[@job][@context] = result
    end
  end

  private

  def without_stdout_buffering
    sync, $stdout.sync = $stdout.sync, true
    yield
  ensure
    $stdout.sync = sync
  end

  def print_parity_report
    baseline_ctx, current_ctx = select_contexts

    name_length = ([12] + @jobs.map { |j| j.name.to_s.length }).max
    base_name = baseline_ctx ? baseline_ctx.name : 'Baseline'
    curr_name = current_ctx ? current_ctx.name : 'Current'

    $stdout.puts
    $stdout.puts "# Performance Parity Report (threshold: #{sprintf('%.2f', @threshold)}x)"
    $stdout.puts
    $stdout.puts "| %-*s | %12s | %12s | %8s | %6s |" % [name_length, 'Benchmark', base_name, curr_name, 'Ratio', 'Status']
    $stdout.puts "|:%s--|-%s:|-%s:|-%s:|-%s:|" % ['-' * name_length, '-' * 12, '-' * 12, '-' * 8, '-' * 6]

    failed = false
    metric = @metrics.first

    @jobs.each do |job|
      base_res = baseline_ctx ? @job_context_result[job][baseline_ctx] : nil
      curr_res = current_ctx ? @job_context_result[job][current_ctx] : nil

      base_val = extract_value(base_res, metric)
      curr_val = extract_value(curr_res, metric)

      ratio = compute_ratio(base_val, curr_val, metric)
      passed = ratio >= @threshold

      status = passed ? 'PASS' : 'FAIL'
      failed ||= !passed

      ratio_str = (ratio > 0) ? sprintf('%.2fx', ratio) : 'N/A'

      $stdout.puts "| %-*s | %12s | %12s | %8s | %6s |" % [
        name_length,
        job.name,
        humanize(base_val),
        humanize(curr_val),
        ratio_str,
        status
      ]
    end

    $stdout.puts

    if failed
      $stdout.puts "Parity check FAILED: One or more benchmark ratios fell below the threshold (#{sprintf('%.2f', @threshold)}x)."
      Kernel.exit(1)
    else
      $stdout.puts "Parity check PASSED: All benchmark ratios met or exceeded threshold (#{sprintf('%.2f', @threshold)}x)."
    end
  end

  def select_contexts
    return [nil, nil] if @contexts.nil? || @contexts.empty?
    return [@contexts[0], @contexts[0]] if @contexts.size == 1

    baseline = @contexts.find { |c| c.name =~ /compare|baseline|c-ruby|reference/i } || @contexts[0]
    current  = @contexts.find { |c| c.name =~ /built|current|target|rust/i && c != baseline } ||
               @contexts.reject { |c| c == baseline }.first ||
               @contexts[1]

    [baseline, current]
  end

  def extract_value(result, metric)
    return nil if result.nil? || result.values.nil?

    if metric && result.values.key?(metric)
      result.values[metric]
    else
      result.values.values.first
    end
  end

  def compute_ratio(base_val, curr_val, metric)
    return 0.0 if base_val.nil? || curr_val.nil? || !base_val.is_a?(Numeric) || !curr_val.is_a?(Numeric)
    return 0.0 if base_val <= 0 || curr_val <= 0

    larger_better = metric ? metric.larger_better : true
    if larger_better
      curr_val.to_f / base_val.to_f
    else
      base_val.to_f / curr_val.to_f
    end
  end

  def humanize(value)
    return 'N/A' if value.nil?
    return 'ERROR' if defined?(BenchmarkDriver::Result::ERROR) && BenchmarkDriver::Result::ERROR.equal?(value)
    return '0.0' if value == 0.0
    return value.inspect unless value.is_a?(Numeric)

    scale = (Math.log10(value) / 3).to_i
    if scale <= 0
      sprintf('%.2f', value)
    else
      suffix = case scale
               when 1; 'k'
               when 2; 'M'
               when 3; 'G'
               when 4; 'T'
               else; ''
               end
      sprintf('%.2f%s', value.to_f / (1000**scale), suffix)
    end
  end
end
