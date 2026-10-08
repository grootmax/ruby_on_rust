#!/usr/bin/env ruby
# frozen_string_literal: true

require "yaml"
require "optparse"
require "fileutils"

# Compatibility Report Generator for Ruby on Rust
# Aggregates quality gate lock results and porting status into COMPATIBILITY.md at repo root.

class CompatibilityReportGenerator
  VALID_STATUSES = ["Not Started", "In Progress", "Ported", "Blocked", "N/A"].freeze

  def initialize(repo_root)
    @repo_root = repo_root
    @porting_status_path = File.join(repo_root, "tool", "porting_status.yml")
    @compatibility_md_path = File.join(repo_root, "COMPATIBILITY.md")
    @allowlist_dir = File.join(repo_root, "tool", "ruby_on_rust", "allowlists")
  end

  def count_allowlist_entries(filename)
    path = File.join(@allowlist_dir, filename)
    return 0 unless File.exist?(path)

    count = 0
    File.foreach(path) do |line|
      line = line.strip
      next if line.empty? || line.start_with?("#")
      count += 1
    end
    count
  end

  def check_quality_gate(script_rel_path, args = [])
    script_path = File.join(@repo_root, script_rel_path)
    return "UNKNOWN" unless File.exist?(script_path)

    cmd = [RbConfig.ruby, script_path, *args]
    system(*cmd, out: File::NULL, err: File::NULL) ? "PASSED" : "FAILED"
  end

  def number_with_delimiter(number)
    number.to_s.gsub(/(\d)(?=(\d\d\d)+(?!\d))/, '\1,')
  end

  def generate_markdown
    # Read porting status
    status_data = File.exist?(@porting_status_path) ? (YAML.load_file(@porting_status_path) || {}) : {}
    subsystems_config = status_data["subsystems"] || {}
    files_config = status_data["files"] || {}

    c_files = Dir.glob(File.join(@repo_root, "*.c")).sort
    file_entries = c_files.map do |file_path|
      basename = File.basename(file_path)
      loc = File.foreach(file_path).count
      config = files_config[basename] || {}

      status = config["status"] || "Not Started"
      status = "Not Started" unless VALID_STATUSES.include?(status)

      {
        basename: basename,
        loc: loc,
        status: status,
        target: config["target"] || "-",
        subsystem: config["subsystem"] || "Utilities & Support",
        notes: config["notes"] || ""
      }
    end

    total_files = file_entries.size
    total_loc = file_entries.sum { |e| e[:loc] }

    stats = VALID_STATUSES.each_with_object({}) do |st, hash|
      entries = file_entries.select { |e| e[:status] == st }
      count = entries.size
      loc = entries.sum { |e| e[:loc] }
      pct = total_loc.positive? ? (loc.to_f / total_loc * 100.0).round(1) : 0.0
      hash[st] = { count: count, loc: loc, pct: pct }
    end

    # Check Quality Gates
    api_gate = check_quality_gate("tool/ruby_on_rust/check_api_lock.rb", ["--check"])
    abi_gate = check_quality_gate("tool/ruby_on_rust/check_abi_lock.rb", ["--check"])
    docs_gate = check_quality_gate("tool/ruby_on_rust/check_docs_lock.rb", ["--check"])
    ledger_gate = check_quality_gate("tool/generate_porting_ledger.rb", ["--check"])

    api_allow_count = count_allowlist_entries("api_allowlist.txt")
    abi_allow_count = count_allowlist_entries("abi_allowlist.txt")
    docs_allow_count = count_allowlist_entries("docs_allowlist.txt")

    lines = []
    lines << "# Ruby on Rust Compatibility & Parity Report"
    lines << ""
    lines << "Automated compatibility, parity, and quality gate report tracking baseline CRuby interface lock status and C-to-Rust migration progress."
    lines << ""
    lines << "Generated automatically by `ruby tool/ruby_on_rust/update_compatibility.rb`. Do not edit manually."
    lines << ""
    lines << "## Quality Gate Status Summary"
    lines << ""
    lines << "| Quality Gate | Status | Allowlisted Exceptions | Description |"
    lines << "| :--- | :--- | :--- | :--- |"
    lines << "| **Core API Lock** | **#{api_gate}** | #{api_allow_count} | Core class/module reflection, method arity, and constant parity |"
    lines << "| **C ABI & Struct Offset Lock** | **#{abi_gate}** | #{abi_allow_count} | Header macros in `include/ruby/*.h`, exported C symbols, and struct field offsets |"
    lines << "| **Core Documentation Lock** | **#{docs_gate}** | #{docs_allow_count} | Core RDoc/RI documentation coverage and parity |"
    lines << "| **Porting Ledger Drift** | **#{ledger_gate}** | - | Synchronization between top-level C files, `PORTING.md`, and `porting_status.yml` |"
    lines << ""
    lines << "## C-to-Rust Migration Summary"
    lines << ""
    lines << "| Metric | File Count | Lines of Code (LOC) | % of Total LOC |"
    lines << "| :--- | :--- | :--- | :--- |"
    lines << "| **Total C Source Files** | #{number_with_delimiter(total_files)} | #{number_with_delimiter(total_loc)} | 100.0% |"

    VALID_STATUSES.each do |st|
      s = stats[st]
      lines << "| **#{st}** | #{number_with_delimiter(s[:count])} | #{number_with_delimiter(s[:loc])} | #{s[:pct]}% |"
    end

    lines << ""
    lines << "## Subsystem Porting Breakdown"
    lines << ""
    lines << "| Subsystem | Total Files | Total LOC | Ported / In Progress | Status |"
    lines << "| :--- | :--- | :--- | :--- | :--- |"

    subsystems = file_entries.map { |e| e[:subsystem] }.uniq.sort
    subsystems.each do |sub|
      sub_entries = file_entries.select { |e| e[:subsystem] == sub }
      s_files = sub_entries.size
      s_loc = sub_entries.sum { |e| e[:loc] }
      active_count = sub_entries.count { |e| %w[Ported In\ Progress].include?(e[:status]) }
      status_str = active_count.positive? ? "#{active_count} active" : "Baseline C"
      lines << "| **#{sub}** | #{s_files} | #{number_with_delimiter(s_loc)} | #{active_count} / #{s_files} | #{status_str} |"
    end

    lines << ""
    lines << "## Quality Gate Allowlists"
    lines << ""
    lines << "Plain-text allowlists filter approved temporary deviations:"
    lines << "- `tool/ruby_on_rust/allowlists/api_allowlist.txt`: #{api_allow_count} exception pattern(s)"
    lines << "- `tool/ruby_on_rust/allowlists/abi_allowlist.txt`: #{abi_allow_count} exception pattern(s)"
    lines << "- `tool/ruby_on_rust/allowlists/docs_allowlist.txt`: #{docs_allow_count} exception pattern(s)"
    lines << ""

    lines.join("\n") + "\n"
  end

  def run(check_mode: false)
    content = generate_markdown

    if check_mode
      unless File.exist?(@compatibility_md_path)
        warn "Error: COMPATIBILITY.md is missing at #{@compatibility_md_path}"
        return false
      end

      existing_content = File.read(@compatibility_md_path)
      if existing_content == content
        puts "COMPATIBILITY.md is up-to-date."
        true
      else
        warn "Error: COMPATIBILITY.md is out of date. Run `ruby tool/ruby_on_rust/update_compatibility.rb` to update."
        false
      end
    else
      File.write(@compatibility_md_path, content)
      puts "Updated COMPATIBILITY.md at #{@compatibility_md_path}"
      true
    end
  end
end

if __FILE__ == $0
  check_mode = false

  OptionParser.new do |opts|
    opts.banner = "Usage: ruby tool/ruby_on_rust/update_compatibility.rb [options]"

    opts.on("--check", "Check if COMPATIBILITY.md is up-to-date without modifying it") do
      check_mode = true
    end

    opts.on("-h", "--help", "Show help message") do
      puts opts
      exit 0
    end
  end.parse!

  repo_root = File.expand_path("../../", __dir__)
  generator = CompatibilityReportGenerator.new(repo_root)
  success = generator.run(check_mode: check_mode)
  exit(success ? 0 : 1)
end
