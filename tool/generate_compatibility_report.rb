#!/usr/bin/env ruby
# frozen_string_literal: true

require "json"
require "optparse"
require "fileutils"

def number_with_delimiter(number)
  number.to_s.gsub(/(\d)(?=(\d\d\d)+(?!\d))/, '\1,')
end

def calculate_pass_rate(passed, total)
  return "0.0%" if total.nil? || total.zero?

  rate = (passed.to_f / total * 100.0).round(1)
  format("%.1f%%", rate)
end

def parse_core_rs_log(text)
  return nil if text.nil? || text.empty?

  if text =~ /test result: \w+\.\s+(\d+)\s+passed;\s+(\d+)\s+failed;\s+(\d+)\s+ignored/
    passed = $1.to_i
    failed = $2.to_i
    skipped = $3.to_i
    total = passed + failed + skipped
    { "total" => total, "passed" => passed, "failed" => failed, "skipped" => skipped }
  end
end

def parse_btest_log(text)
  return nil if text.nil? || text.empty?

  if text =~ /PASS all (\d+) tests/
    total = $1.to_i
    { "total" => total, "passed" => total, "failed" => 0, "skipped" => 0 }
  elsif text =~ /FAIL (\d+)\/(\d+) tests failed/
    failed = $1.to_i
    total = $2.to_i
    passed = [total - failed, 0].max
    { "total" => total, "passed" => passed, "failed" => failed, "skipped" => 0 }
  end
end

def parse_test_all_log(text)
  return nil if text.nil? || text.empty?

  if text =~ /(\d+)\s+tests?,\s+(\d+)\s+assertions?,\s+(\d+)\s+failures?,\s+(\d+)\s+errors?,\s+(\d+)\s+skips?/
    total = $1.to_i
    failures = $3.to_i
    errors = $4.to_i
    skipped = $5.to_i
    failed = failures + errors
    passed = [total - failed - skipped, 0].max
    { "total" => total, "passed" => passed, "failed" => failed, "skipped" => skipped }
  end
end

def parse_test_spec_log(text)
  return nil if text.nil? || text.empty?

  if text =~ /(\d+)\s+files?,\s+(\d+)\s+examples?,\s+(\d+)\s+expectations?,\s+(\d+)\s+failures?,\s+(\d+)\s+errors?,\s+(\d+)\s+tagged/
    total = $2.to_i
    failures = $4.to_i
    errors = $5.to_i
    skipped = $6.to_i
    failed = failures + errors
    passed = [total - failed - skipped, 0].max
    { "total" => total, "passed" => passed, "failed" => failed, "skipped" => skipped }
  end
end

check_mode = false
run_mode = false
log_dir = nil
btest_log_file = nil
test_all_log_file = nil
test_spec_log_file = nil
core_rs_log_file = nil
json_results_path = nil
output_file = nil

OptionParser.new do |opts|
  opts.banner = "Usage: ruby tool/generate_compatibility_report.rb [options]"

  opts.on("--check", "Check if COMPATIBILITY.md is up-to-date without modifying it") do
    check_mode = true
  end

  opts.on("--run", "Run test suites live and update compatibility stats") do
    run_mode = true
  end

  opts.on("--log-dir DIR", "Directory containing test log files") do |dir|
    log_dir = dir
  end

  opts.on("--btest-log FILE", "Path to bootstraptest log file") do |f|
    btest_log_file = f
  end

  opts.on("--test-all-log FILE", "Path to test-all log file") do |f|
    test_all_log_file = f
  end

  opts.on("--test-spec-log FILE", "Path to test-spec log file") do |f|
    test_spec_log_file = f
  end

  opts.on("--core-rs-log FILE", "Path to core-rs-test log file") do |f|
    core_rs_log_file = f
  end

  opts.on("--json FILE", "Path to JSON results file") do |f|
    json_results_path = f
  end

  opts.on("-o", "--output FILE", "Path to output Markdown file (default: COMPATIBILITY.md)") do |f|
    output_file = f
  end

  opts.on("-h", "--help", "Show this help message") do
    puts opts
    exit 0
  end
end.parse!

repo_root = File.expand_path("..", __dir__)
json_results_path ||= File.join(repo_root, "tool", "compatibility_results.json")
output_file ||= File.join(repo_root, "COMPATIBILITY.md")

results = {
  "core_rs" => { "total" => 0, "passed" => 0, "failed" => 0, "skipped" => 0 },
  "btest" => { "total" => 0, "passed" => 0, "failed" => 0, "skipped" => 0 },
  "test_all" => { "total" => 0, "passed" => 0, "failed" => 0, "skipped" => 0 },
  "test_spec" => { "total" => 0, "passed" => 0, "failed" => 0, "skipped" => 0 }
}

if File.file?(json_results_path)
  begin
    existing_data = JSON.parse(File.read(json_results_path))
    existing_data.each do |k, v|
      results[k] = v if results.key?(k) && v.is_a?(Hash)
    end
  rescue JSON::ParserError => e
    warn "Warning: Could not parse existing JSON results file: #{e.message}"
  end
end

if log_dir && File.directory?(log_dir)
  btest_log_file ||= File.join(log_dir, "btest.log")
  test_all_log_file ||= File.join(log_dir, "test_all.log")
  test_spec_log_file ||= File.join(log_dir, "test_spec.log")
  core_rs_log_file ||= File.join(log_dir, "core_rs.log")
end

if core_rs_log_file && File.file?(core_rs_log_file)
  parsed = parse_core_rs_log(File.read(core_rs_log_file))
  results["core_rs"] = parsed if parsed
end

if btest_log_file && File.file?(btest_log_file)
  parsed = parse_btest_log(File.read(btest_log_file))
  results["btest"] = parsed if parsed
end

if test_all_log_file && File.file?(test_all_log_file)
  parsed = parse_test_all_log(File.read(test_all_log_file))
  results["test_all"] = parsed if parsed
end

if test_spec_log_file && File.file?(test_spec_log_file)
  parsed = parse_test_spec_log(File.read(test_spec_log_file))
  results["test_spec"] = parsed if parsed
end

if run_mode
  puts "Running core-rs-test..."
  core_rs_output = `make core-rs-test 2>&1`
  parsed = parse_core_rs_log(core_rs_output)
  results["core_rs"] = parsed if parsed

  puts "Running btest..."
  btest_output = `make btest 2>&1`
  parsed = parse_btest_log(btest_output)
  results["btest"] = parsed if parsed

  puts "Running test-all..."
  test_all_output = `make test-all TESTS="test/ruby/" 2>&1`
  parsed = parse_test_all_log(test_all_output)
  results["test_all"] = parsed if parsed

  puts "Running test-spec..."
  test_spec_output = `make test-spec 2>&1`
  parsed = parse_test_spec_log(test_spec_output)
  results["test_spec"] = parsed if parsed
end

suites_config = [
  {
    "id" => "core_rs",
    "name" => "core_rs Unit Tests",
    "make_target" => "make core-rs-test",
    "description" => "Rust unit tests for `core_rs` ported modules."
  },
  {
    "id" => "btest",
    "name" => "Bootstrap Tests (btest)",
    "make_target" => "make btest",
    "description" => "Core language syntax and VM functionality tests."
  },
  {
    "id" => "test_all",
    "name" => "Ruby Test Suite (test-all)",
    "make_target" => "make test-all",
    "description" => "Standard library and core class unit tests."
  },
  {
    "id" => "test_spec",
    "name" => "RubySpec (test-spec)",
    "make_target" => "make test-spec",
    "description" => "Ruby language and library specifications."
  }
]

total_tests = suites_config.sum { |s| results[s["id"]]["total"].to_i }
total_passed = suites_config.sum { |s| results[s["id"]]["passed"].to_i }
total_failed = suites_config.sum { |s| results[s["id"]]["failed"].to_i }
total_skipped = suites_config.sum { |s| results[s["id"]]["skipped"].to_i }
overall_pass_rate = calculate_pass_rate(total_passed, total_tests)

lines = []
lines << "# Ruby on Rust Compatibility Report"
lines << ""
lines << "Automated test compliance report tracking Ruby test suite compatibility for Ruby on Rust."
lines << "Generated automatically by `ruby tool/generate_compatibility_report.rb`. Do not edit manually."
lines << ""
lines << "## Summary Statistics"
lines << ""
lines << "| Test Suite | Total Tests | Passed | Failed | Skipped | Pass Rate |"
lines << "| :--- | :--- | :--- | :--- | :--- | :--- |"

suites_config.each do |s|
  res = results[s["id"]]
  tot = res["total"].to_i
  pas = res["passed"].to_i
  fai = res["failed"].to_i
  ski = res["skipped"].to_i
  rate = calculate_pass_rate(pas, tot)
  lines << "| **#{s['name']}** | #{number_with_delimiter(tot)} | #{number_with_delimiter(pas)} | #{number_with_delimiter(fai)} | #{number_with_delimiter(ski)} | #{rate} |"
end

lines << "| **Total / Overall** | #{number_with_delimiter(total_tests)} | #{number_with_delimiter(total_passed)} | #{number_with_delimiter(total_failed)} | #{number_with_delimiter(total_skipped)} | #{overall_pass_rate} |"
lines << ""
lines << "## Detailed Test Suite Breakdown"
lines << ""

suites_config.each do |s|
  res = results[s["id"]]
  tot = res["total"].to_i
  pas = res["passed"].to_i
  fai = res["failed"].to_i
  ski = res["skipped"].to_i
  rate = calculate_pass_rate(pas, tot)

  lines << "### #{s['name']}"
  lines << "- **Description:** #{s['description']} (`#{s['make_target']}`)."
  lines << "- **Total Tests:** #{number_with_delimiter(tot)}"
  lines << "- **Passed:** #{number_with_delimiter(pas)}"
  lines << "- **Failed:** #{number_with_delimiter(fai)}"
  lines << "- **Skipped:** #{number_with_delimiter(ski)}"
  lines << "- **Pass Rate:** #{rate}"
  lines << ""
end

generated_markdown = lines.join("\n") + "\n"

if check_mode
  unless File.file?(output_file)
    warn "Error: COMPATIBILITY.md does not exist at #{output_file}."
    exit 1
  end

  existing_markdown = File.read(output_file)

  if existing_markdown == generated_markdown
    puts "COMPATIBILITY.md is up to date."
    exit 0
  else
    warn "Error: COMPATIBILITY.md is out of date. Please run 'ruby tool/generate_compatibility_report.rb' to update."
    exit 1
  end
else
  FileUtils.mkdir_p(File.dirname(json_results_path))
  File.write(json_results_path, JSON.pretty_generate(results) + "\n")
  File.write(output_file, generated_markdown)
  puts "Updated COMPATIBILITY.md and #{json_results_path} successfully."
  exit 0
end
