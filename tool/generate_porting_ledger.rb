#!/usr/bin/env ruby
# frozen_string_literal: true

require "yaml"
require "optparse"
require "set"

VALID_STATUSES = ["Not Started", "In Progress", "Ported", "Blocked", "N/A"].freeze

def number_with_delimiter(number)
  number.to_s.gsub(/(\d)(?=(\d\d\d)+(?!\d))/, '\1,')
end

def resolve_rust_files(repo_root, target_str)
  return [] if target_str.nil? || target_str == "-" || target_str == "N/A"

  case target_str
  when "yjit"
    Dir.glob(File.join(repo_root, "yjit", "src", "**", "*.rs")).sort
  when "zjit"
    Dir.glob(File.join(repo_root, "zjit", "src", "**", "*.rs")).sort
  when "jit"
    Dir.glob(File.join(repo_root, "jit", "src", "**", "*.rs")).sort
  when "gc::mmtk"
    Dir.glob(File.join(repo_root, "gc", "mmtk", "src", "**", "*.rs")).sort
  else
    if target_str.start_with?("core_rs::")
      mod_name = target_str.sub("core_rs::", "").tr("::", "/")
      f1 = File.join(repo_root, "core_rs", "src", "#{mod_name}.rs")
      f2 = File.join(repo_root, "core_rs", "src", mod_name, "mod.rs")
      [f1, f2].select { |f| File.file?(f) }
    elsif target_str.start_with?("crate::")
      mod_name = target_str.sub("crate::", "").tr("::", "/")
      f1 = File.join(repo_root, "core_rs", "src", "#{mod_name}.rs")
      f2 = File.join(repo_root, "src", "#{mod_name}.rs")
      f3 = File.join(repo_root, "core_rs", "src", mod_name, "mod.rs")
      [f1, f2, f3].select { |f| File.file?(f) }
    else
      []
    end
  end
end

def analyze_rust_files(files)
  return { rust_loc: 0, unsafe_blocks: 0, unsafe_lines: 0, unsafe_pct: 0.0 } if files.empty?

  total_rust_loc = 0
  total_unsafe_blocks = 0
  total_unsafe_lines = 0

  files.each do |file_path|
    next unless File.file?(file_path)
    lines = File.readlines(file_path, encoding: "binary")
    loc = lines.size
    total_rust_loc += loc
    next if loc == 0

    unsafe_line_set = Set.new

    lines.each_with_index do |raw_line, idx|
      line_num = idx + 1
      clean = raw_line.sub(%r{//.*$}, "")
      # Ignore #[unsafe(...)] attributes when counting unsafe blocks
      code_without_attrs = clean.gsub(/#\[unsafe\([^\]]*\)\]/, "")

      if code_without_attrs =~ /\bunsafe\b/
        total_unsafe_blocks += 1
        unsafe_line_set.add(line_num)

        depth = 0
        started = false
        (idx...lines.size).each do |j|
          l = lines[j].sub(%r{//.*$}, "").gsub(/"([^"\\]|\\.)*"/, "").gsub(/\x27[^\x27]*\x27/, "")
          if !started && l.include?(";") && (!l.include?("{") || l.index(";") < l.index("{"))
            break
          end
          l.chars.each do |ch|
            if ch == "{"
              depth += 1
              started = true
            elsif ch == "}"
              depth -= 1 if started
            end
          end
          if started
            unsafe_line_set.add(j + 1)
            break if depth <= 0
          end
        end
      end
    end

    total_unsafe_lines += unsafe_line_set.size
  end

  unsafe_pct = total_rust_loc.positive? ? (total_unsafe_lines.to_f / total_rust_loc * 100.0).round(1) : 0.0

  {
    rust_loc: total_rust_loc,
    unsafe_blocks: total_unsafe_blocks,
    unsafe_lines: total_unsafe_lines,
    unsafe_pct: unsafe_pct
  }
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

# Validate schema of status configuration
unless subsystems_config.is_a?(Hash) && files_config.is_a?(Hash)
  warn "Error: Invalid schema in #{status_file_path}"
  exit 1
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

  target = config["target"] || "-"
  rust_files = resolve_rust_files(repo_root, target)
  rust_metrics = analyze_rust_files(rust_files)

  {
    basename: basename,
    loc: loc,
    status: status,
    target: target,
    subsystem: config["subsystem"] || "Utilities & Support",
    notes: config["notes"] || "",
    rust_loc: rust_metrics[:rust_loc],
    unsafe_blocks: rust_metrics[:unsafe_blocks],
    unsafe_lines: rust_metrics[:unsafe_lines],
    unsafe_pct: rust_metrics[:unsafe_pct]
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

# Collect unique active Rust targets with metrics
active_rust_targets = file_entries.reject { |e| e[:target] == "-" || e[:target] == "N/A" }
                                 .group_by { |e| e[:target] }
                                 .map do |target_name, entries|
  sample = entries.first
  {
    target: target_name,
    rust_loc: sample[:rust_loc],
    unsafe_blocks: sample[:unsafe_blocks],
    unsafe_lines: sample[:unsafe_lines],
    unsafe_pct: sample[:unsafe_pct]
  }
end.select { |t| t[:rust_loc] > 0 }.sort_by { |t| t[:target] }

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

unless active_rust_targets.empty?
  lines << ""
  lines << "## Rust Unsafe Block Density Metrics"
  lines << ""
  lines << "| Target Rust Crate/Module | Rust Lines (LOC) | Unsafe Blocks | Unsafe Lines | Unsafe Line % |"
  lines << "| :--- | :--- | :--- | :--- | :--- |"

  active_rust_targets.each do |t|
    lines << "| `#{t[:target]}` | #{number_with_delimiter(t[:rust_loc])} | #{number_with_delimiter(t[:unsafe_blocks])} | #{number_with_delimiter(t[:unsafe_lines])} | #{t[:unsafe_pct]}% |"
  end
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
  lines << "| C Source File | Lines (LOC) | Status | Target Rust Crate/Module | Unsafe Blocks | Unsafe Lines (%) | Notes |"
  lines << "| :--- | :--- | :--- | :--- | :--- | :--- | :--- |"

  sub_entries.sort_by { |e| e[:basename] }.each do |e|
    lines << "| `#{e[:basename]}` | #{number_with_delimiter(e[:loc])} | #{e[:status]} | `#{e[:target]}` | #{e[:unsafe_blocks]} | #{e[:unsafe_pct]}% | #{e[:notes]} |"
  end
  lines << ""
end

lines << "## Complete C Source File Ledger"
lines << ""
lines << "| C Source File | Subsystem | Lines (LOC) | Status | Target Rust Crate/Module | Unsafe Blocks | Unsafe Lines (%) | Notes |"
lines << "| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |"

file_entries.sort_by { |e| e[:basename] }.each do |e|
  lines << "| `#{e[:basename]}` | #{e[:subsystem]} | #{number_with_delimiter(e[:loc])} | #{e[:status]} | `#{e[:target]}` | #{e[:unsafe_blocks]} | #{e[:unsafe_pct]}% | #{e[:notes]} |"
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
      if cells.any? { |c| c.include?("LOC") || c.include?("Unsafe") }
        masked_columns = cells.each_index.select { |i| cells[i].include?("LOC") || cells[i].include?("Unsafe") }
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
    puts "PORTING.md is up to date (line counts/metrics have drifted; run 'ruby tool/generate_porting_ledger.rb' to refresh them)."
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
