#!/usr/bin/env ruby
# frozen_string_literal: true

require "yaml"
require "optparse"
require "date"

VALID_STATUSES = ["Not Started", "In Progress", "Ported", "Blocked", "N/A"].freeze
SOAK_GATE_DAYS = 28

def number_with_delimiter(number)
  number.to_s.gsub(/(\d)(?=(\d\d\d)+(?!\d))/, '\1,')
end

check_mode = false
OptionParser.new do |opts|
  opts.banner = "Usage: ruby tool/generate_porting_ledger.rb [options]"

  opts.on("--check", "Check if PORTING.md is up-to-date without modifying it") do
    check_mode = true
  end

  opts.on("-h", "--help", "Show this help message") do
    puts opts
    exit 0
  end
end.parse!

repo_root = File.expand_path("..", __dir__)
status_file_path = File.join(repo_root, "tool", "porting_status.yml")
porting_md_path = File.join(repo_root, "PORTING.md")

unless File.file?(status_file_path)
  warn "Error: Status configuration file not found at #{status_file_path}"
  exit 1
end

status_data = YAML.load_file(status_file_path) || {}
subsystems_config = status_data["subsystems"] || {}
files_config = status_data["files"] || {}

unless subsystems_config.is_a?(Hash) && files_config.is_a?(Hash)
  warn "Error: Invalid schema in #{status_file_path}: 'subsystems' and 'files' must be Hashes"
  exit 1
end

# Validate ported_date entries in status configuration
files_config.each do |filename, config|
  next unless config.is_a?(Hash)
  pdate = config["ported_date"]
  if pdate
    begin
      Date.parse(pdate.to_s)
    rescue ArgumentError
      warn "Error: Invalid ported_date '#{pdate}' for #{filename} in #{status_file_path}. Expected YYYY-MM-DD format."
      exit 1
    end
  end
end

c_files = Dir.glob(File.join(repo_root, "*.c")).sort

if !c_files.empty? && system("git", "rev-parse", "--is-inside-work-tree", out: File::NULL, err: File::NULL, chdir: repo_root)
  IO.popen(["git", "-C", repo_root, "check-ignore", *c_files]) do |io|
    ignored_files = io.read.split("\n")
    c_files -= ignored_files
  end
end

file_entries = c_files.map do |file_path|
  basename = File.basename(file_path)
  loc = File.foreach(file_path).count
  config = files_config[basename] || {}

  status = config["status"] || "Not Started"
  unless VALID_STATUSES.include?(status)
    warn "Warning: Invalid status '#{status}' for #{basename}, defaulting to 'Not Started'"
    status = "Not Started"
  end

  ported_date_raw = config["ported_date"]
  ported_date = nil
  soak_days = nil
  soak_status = "-"
  soak_passed = false

  if ported_date_raw
    begin
      parsed_date = Date.parse(ported_date_raw.to_s)
      ported_date = parsed_date.strftime("%Y-%m-%d")
      elapsed = (Date.today - parsed_date).to_i
      soak_days = [elapsed, 0].max
      soak_passed = soak_days >= SOAK_GATE_DAYS
      soak_status = soak_passed ? "Passed (#{soak_days} days)" : "In Soak (#{soak_days}/#{SOAK_GATE_DAYS} days)"
    rescue ArgumentError
      soak_status = "Invalid Date"
    end
  elsif status == "Ported"
    soak_status = "Pending Date"
  end

  {
    basename: basename,
    loc: loc,
    status: status,
    target: config["target"] || "-",
    subsystem: config["subsystem"] || "Utilities & Support",
    notes: config["notes"] || "",
    ported_date: ported_date,
    soak_days: soak_days,
    soak_status: soak_status,
    soak_passed: soak_passed
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

ported_entries = file_entries.select { |e| e[:status] == "Ported" || e[:ported_date] }

# Build Markdown content
lines = []
lines << "# C-to-Rust Porting Ledger"
lines << ""
lines << "Automated migration ledger tracking C source files, line counts, and Rust porting status."
lines << "Generated automatically by `ruby tool/generate_porting_ledger.rb`. Do not edit manually."
lines << ""
lines << "## Summary Statistics"
lines << ""
lines << "| Metric | File Count | Lines of Code (LOC) | % of Total LOC |"
lines << "| :--- | :--- | :--- | :--- |"
lines << "| **Total C Source Files** | #{number_with_delimiter(total_files)} | #{number_with_delimiter(total_loc)} | 100.0% |"

VALID_STATUSES.each do |st|
  s = stats[st]
  lines << "| **#{st}** | #{number_with_delimiter(s[:count])} | #{number_with_delimiter(s[:loc])} | #{s[:pct]}% |"
end

lines << ""
lines << "## Soak Gate Verification (4-Week Stability Gate)"
lines << ""
lines << "Mandatory 28-day stability soak period tracking for ported modules before final hardening classification."
lines << ""

if ported_entries.empty?
  lines << "_No ported modules currently in soak verification._"
  lines << ""
else
  lines << "| C Source File | Target Rust Crate/Module | Ported Date | Days Elapsed | Soak Gate Status |"
  lines << "| :--- | :--- | :--- | :--- | :--- |"
  ported_entries.sort_by { |e| e[:basename] }.each do |e|
    pdate_str = e[:ported_date] || "-"
    days_str = e[:soak_days] ? "#{e[:soak_days]} days" : "-"
    lines << "| `#{e[:basename]}` | `#{e[:target]}` | #{pdate_str} | #{days_str} | #{e[:soak_status]} |"
  end
  lines << ""
end

lines << "## Migration Progress by Subsystem"
lines << ""

subsystem_names = subsystems_config.keys
extra_subsystems = file_entries.map { |e| e[:subsystem] }.uniq - subsystem_names
all_subsystems = subsystem_names + extra_subsystems

all_subsystems.each do |subsystem_name|
  sub_entries = file_entries.select { |e| e[:subsystem] == subsystem_name }
  next if sub_entries.empty?

  sub_desc = subsystems_config.dig(subsystem_name, "description")

  lines << "### #{subsystem_name}"
  lines << "_#{sub_desc}_" if sub_desc && !sub_desc.empty?
  lines << ""
  lines << "| C Source File | Lines (LOC) | Status | Target Rust Crate/Module | Ported Date | Soak Status | Notes |"
  lines << "| :--- | :--- | :--- | :--- | :--- | :--- | :--- |"

  sub_entries.sort_by { |e| e[:basename] }.each do |e|
    pdate_str = e[:ported_date] || "-"
    lines << "| `#{e[:basename]}` | #{number_with_delimiter(e[:loc])} | #{e[:status]} | `#{e[:target]}` | #{pdate_str} | #{e[:soak_status]} | #{e[:notes]} |"
  end
  lines << ""
end

lines << "## Complete C Source File Ledger"
lines << ""
lines << "| C Source File | Subsystem | Lines (LOC) | Status | Target Rust Crate/Module | Ported Date | Soak Status | Notes |"
lines << "| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |"

file_entries.sort_by { |e| e[:basename] }.each do |e|
  pdate_str = e[:ported_date] || "-"
  lines << "| `#{e[:basename]}` | #{e[:subsystem]} | #{number_with_delimiter(e[:loc])} | #{e[:status]} | `#{e[:target]}` | #{pdate_str} | #{e[:soak_status]} | #{e[:notes]} |"
end
lines << ""

generated_markdown = lines.join("\n")

if check_mode
  unless File.file?(porting_md_path)
    warn "Error: PORTING.md does not exist."
    exit 1
  end

  existing_markdown = File.read(porting_md_path)

  mask_loc = lambda do |markdown|
    masked_columns = []
    markdown.lines.map do |line|
      unless line.start_with?("|")
        masked_columns = []
        next line
      end
      cells = line.chomp.split("|", -1)
      if cells.any? { |c| c.include?("LOC") || c.include?("Unsafe") || c.include?("Date") || c.include?("Soak") || c.include?("Days") }
        masked_columns = cells.each_index.select { |i| cells[i].include?("LOC") || cells[i].include?("Unsafe") || cells[i].include?("Date") || cells[i].include?("Soak") || cells[i].include?("Days") }
        next line
      end
      masked_columns.each { |i| cells[i] = " # " if cells[i] && !cells[i].strip.empty? }
      cells.join("|") + "\n"
    end.join
  end

  if existing_markdown == generated_markdown
    puts "PORTING.md is up to date."
    exit 0
  elsif mask_loc.(existing_markdown) == mask_loc.(generated_markdown)
    puts "PORTING.md is up to date (line counts/soak metrics have drifted; run 'ruby tool/generate_porting_ledger.rb' to refresh them)."
    exit 0
  else
    warn "Error: PORTING.md is out of date. Please run 'ruby tool/generate_porting_ledger.rb' to update."
    exit 1
  end
else
  File.write(porting_md_path, generated_markdown)
  puts "Updated PORTING.md successfully (#{total_files} C source files, #{number_with_delimiter(total_loc)} LOC)."
  exit 0
end
