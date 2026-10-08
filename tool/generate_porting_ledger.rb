#!/usr/bin/env ruby
# frozen_string_literal: true

require "yaml"
require "optparse"
require_relative "core_rs/inventory"

VALID_STATUSES = ["Not Started", "In Progress", "Ported", "Blocked", "N/A"].freeze

def number_with_delimiter(number)
  number.to_s.gsub(/(\d)(?=(\d\d\d)+(?!\d))/, '\1,')
end

def normalize_units(units_raw, file_default_target = "-")
  return [] if units_raw.nil?

  units_list = []
  if units_raw.is_a?(Array)
    units_list = units_raw.map do |u|
      u_hash = u.dup
      u_hash["target"] ||= file_default_target
      u_hash
    end
  elsif units_raw.is_a?(Hash)
    units_list = units_raw.map do |id, u|
      u_hash = (u || {}).dup
      u_hash["id"] ||= id.to_s
      u_hash["target"] ||= file_default_target
      u_hash
    end
  else
    abort "Error: Invalid 'units' format. Expected Array or Hash, got #{units_raw.class}"
  end

  units_list.map do |u|
    {
      id: u["id"].to_s,
      status: u["status"].to_s,
      target: u["target"].to_s,
      functions: Array(u["functions"]).map(&:to_s),
      notes: u["notes"] ? u["notes"].to_s : ""
    }
  end
end

def validate_file_entry!(basename, config)
  unless config.is_a?(Hash)
    abort "Error: Configuration for '#{basename}' in porting_status.yml must be a Hash"
  end

  status = config["status"]
  if status.nil? || status.to_s.strip.empty? || !VALID_STATUSES.include?(status)
    abort "Error: Invalid or missing status '#{status}' for '#{basename}' in porting_status.yml. Must be one of #{VALID_STATUSES.join(', ')}"
  end

  target = config["target"]
  if target.nil? || target.to_s.strip.empty?
    abort "Error: Missing target for '#{basename}' in porting_status.yml"
  end

  if config.key?("units")
    units_raw = config["units"]
    if units_raw.nil? || (!units_raw.is_a?(Array) && !units_raw.is_a?(Hash))
      abort "Error: Invalid 'units' for '#{basename}' in porting_status.yml; expected Array or Hash"
    end

    units = normalize_units(units_raw, target)
    units.each do |unit|
      if unit[:id].strip.empty?
        abort "Error: Unit in '#{basename}' missing required 'id'"
      end

      unless VALID_STATUSES.include?(unit[:status])
        abort "Error: Invalid unit status '#{unit[:status]}' for unit '#{unit[:id]}' in '#{basename}'. Must be one of #{VALID_STATUSES.join(', ')}"
      end

      if unit[:target].strip.empty?
        abort "Error: Missing target for unit '#{unit[:id]}' in '#{basename}'"
      end

      if unit[:functions].empty?
        abort "Error: Unit '#{unit[:id]}' in '#{basename}' must contain at least one function in 'functions'"
      end
    end
  end
end

check_mode = false
check_kits_mode = false

OptionParser.new do |opts|
  opts.banner = "Usage: ruby tool/generate_porting_ledger.rb [options]"

  opts.on("--check", "Check if PORTING.md is up-to-date without modifying it") do
    check_mode = true
  end

  opts.on("--check-kits", "Check kit definitions against C function inventory") do
    check_kits_mode = true
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

files_config.each do |basename, config|
  validate_file_entry!(basename, config)
end

inventory_funcs = inventory
inventory_by_file = inventory_funcs.group_by { |f| File.basename(f.file) }

if check_kits_mode
  errors = []
  checked_files_count = 0

  files_config.each do |basename, config|
    next unless config.key?("units") && !config["units"].nil?

    checked_files_count += 1
    units = normalize_units(config["units"], config["target"])
    inv_funcs = inventory_by_file[basename] || []
    inv_func_names = inv_funcs.map(&:name)

    declared_func_names = units.flat_map { |u| u[:functions] }

    duplicates = declared_func_names.select { |fn| declared_func_names.count(fn) > 1 }.uniq
    missing = inv_func_names - declared_func_names
    extra = declared_func_names - inv_func_names

    if duplicates.any?
      errors << "#{basename}: #{duplicates.size} duplicate function(s) declared in units: #{duplicates.join(', ')}"
    end
    if missing.any?
      errors << "#{basename}: #{missing.size} unmapped function(s) missing from units: #{missing.join(', ')}"
    end
    if extra.any?
      errors << "#{basename}: #{extra.size} extra function(s) declared in units not in inventory: #{extra.join(', ')}"
    end
  end

  if errors.any?
    warn "Kit inventory validation failed with #{errors.size} error(s):"
    errors.each { |e| warn "  - #{e}" }
    exit 1
  else
    puts "Kit inventory check passed: #{checked_files_count} file(s) validated against function inventory."
    exit 0
  end
end

c_files = Dir.glob(File.join(repo_root, "*.c")).sort

if !c_files.empty? && system("git", "rev-parse", "--is-inside-work-tree", out: File::NULL, err: File::NULL, chdir: repo_root)
  IO.popen(["git", "-C", repo_root, "check-ignore", *c_files]) do |io|
    ignored_files = io.read.split("\n")
    c_files -= ignored_files
  end
end

func_loc_lookup = {}
inventory_funcs.each do |f|
  func_loc_lookup[[File.basename(f.file), f.name]] = f.loc
end

file_entries = c_files.map do |file_path|
  basename = File.basename(file_path)
  loc = File.foreach(file_path).count
  config = files_config[basename] || {}

  status = config["status"] || "Not Started"
  target = config["target"] || "-"
  subsystem = config["subsystem"] || "Utilities & Support"
  notes = config["notes"] || ""

  units = normalize_units(config["units"], target)
  units_with_loc = units.map do |u|
    u_loc = u[:functions].sum { |fn| func_loc_lookup[[basename, fn]] || 0 }
    u.merge(loc: u_loc)
  end

  {
    basename: basename,
    loc: loc,
    status: status,
    target: target,
    subsystem: subsystem,
    notes: notes,
    units: units_with_loc
  }
end

total_files = file_entries.size
total_loc = file_entries.sum { |e| e[:loc] }

stats = VALID_STATUSES.each_with_object({}) do |st, hash|
  count = file_entries.count { |e| e[:status] == st }

  st_loc = 0
  file_entries.each do |e|
    if e[:units].empty?
      st_loc += e[:loc] if e[:status] == st
    else
      units_total_loc = e[:units].sum { |u| u[:loc] }
      remaining_loc = [0, e[:loc] - units_total_loc].max
      st_loc += remaining_loc if e[:status] == st
      st_loc += e[:units].select { |u| u[:status] == st }.sum { |u| u[:loc] }
    end
  end

  pct = total_loc.positive? ? (st_loc.to_f / total_loc * 100.0).round(1) : 0.0
  hash[st] = { count: count, loc: st_loc, pct: pct }
end

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
  lines << "| C Source File | Lines (LOC) | Status | Target Rust Crate/Module | Notes |"
  lines << "| :--- | :--- | :--- | :--- | :--- |"

  sub_entries.sort_by { |e| e[:basename] }.each do |e|
    lines << "| `#{e[:basename]}` | #{number_with_delimiter(e[:loc])} | #{e[:status]} | `#{e[:target]}` | #{e[:notes]} |"
  end
  lines << ""

  sub_entries.select { |e| !e[:units].empty? }.sort_by { |e| e[:basename] }.each do |e|
    lines << "#### `#{e[:basename]}` Function Kits"
    lines << ""
    lines << "| Kit ID | Lines (LOC) | Functions | Status | Target Rust Module | Notes |"
    lines << "| :--- | :--- | :--- | :--- | :--- | :--- |"
    e[:units].each do |u|
      lines << "| `#{u[:id]}` | #{number_with_delimiter(u[:loc])} | #{u[:functions].size} | #{u[:status]} | `#{u[:target]}` | #{u[:notes]} |"
    end
    lines << ""
  end
end

lines << "## Complete C Source File Ledger"
lines << ""
lines << "| C Source File | Subsystem | Lines (LOC) | Status | Target Rust Crate/Module | Notes |"
lines << "| :--- | :--- | :--- | :--- | :--- | :--- |"

file_entries.sort_by { |e| e[:basename] }.each do |e|
  lines << "| `#{e[:basename]}` | #{e[:subsystem]} | #{number_with_delimiter(e[:loc])} | #{e[:status]} | `#{e[:target]}` | #{e[:notes]} |"
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
      if cells.any? { |c| c.include?("LOC") }
        masked_columns = cells.each_index.select { |i| cells[i].include?("LOC") }
        next line
      end
      masked_columns.each { |i| cells[i] = " # " if cells[i] }
      cells.join("|") + "\n"
    end.join
  end

  if existing_markdown == generated_markdown
    puts "PORTING.md is up to date."
    exit 0
  elsif mask_loc.(existing_markdown) == mask_loc.(generated_markdown)
    puts "PORTING.md is up to date (line counts have drifted; run 'ruby tool/generate_porting_ledger.rb' to refresh them)."
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
