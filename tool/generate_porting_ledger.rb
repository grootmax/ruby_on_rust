#!/usr/bin/env ruby
# frozen_string_literal: true

require "yaml"
require "optparse"
require "set"

VALID_STATUSES = ["Not Started", "In Progress", "Ported", "Blocked", "N/A"].freeze

def number_with_delimiter(number)
  number.to_s.gsub(/(\d)(?=(\d\d\d)+(?!\d))/, '\1,')
end

def analyze_rust_file(file_path)
  lines = File.readlines(file_path, encoding: "UTF-8")
  total_loc = lines.size
  return { loc: 0, unsafe_blocks: 0, unsafe_loc: 0 } if total_loc == 0

  unsafe_blocks = 0
  unsafe_lines = Set.new
  in_block_comment = false
  unsafe_depth = 0
  pending_unsafe = false

  lines.each_with_index do |line_content, idx|
    line_num = idx + 1

    code = +""
    i = 0
    chars = line_content.chars
    len = chars.size

    while i < len
      if in_block_comment
        if chars[i] == "*" && chars[i + 1] == "/"
          in_block_comment = false
          i += 2
        else
          i += 1
        end
      elsif chars[i] == "/" && chars[i + 1] == "*"
        in_block_comment = true
        i += 2
      elsif chars[i] == "/" && chars[i + 1] == "/"
        break
      elsif chars[i] == '"' || chars[i] == "'"
        quote = chars[i]
        code << " "
        i += 1
        while i < len && chars[i] != quote
          i += 2 if chars[i] == "\\"
          i += 1
        end
        i += 1
      else
        code << chars[i]
        i += 1
      end
    end

    if unsafe_depth > 0 || pending_unsafe
      unsafe_lines.add(line_num)
    end

    unsafe_matches = code.scan(/\bunsafe\b/)
    if !unsafe_matches.empty?
      unsafe_blocks += unsafe_matches.size
      unsafe_lines.add(line_num)
      unless code.include?("unsafe(")
        pending_unsafe = true
      end
    end

    code.chars.each do |ch|
      if ch == "{"
        if pending_unsafe || unsafe_depth > 0
          unsafe_depth += 1
          pending_unsafe = false
        end
      elsif ch == "}"
        if unsafe_depth > 0
          unsafe_depth -= 1
        end
      elsif ch == ";" && pending_unsafe && unsafe_depth == 0
        pending_unsafe = false
      end
    end
  end

  {
    loc: total_loc,
    unsafe_blocks: unsafe_blocks,
    unsafe_loc: unsafe_lines.size
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
# Add any subsystems present in file entries that were not explicitly in config
extra_subsystems = file_entries.map { |e| e[:subsystem] }.uniq - subsystem_names
all_subsystems = subsystem_names + extra_subsystems

all_subsystems.each do |subsystem_name|
  sub_entries = file_entries.select { |e| e[:subsystem] == subsystem_name }
  next if sub_entries.empty?

  sub_loc = sub_entries.sum { |e| e[:loc] }
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
end

lines << "## Complete C Source File Ledger"
lines << ""
lines << "| C Source File | Subsystem | Lines (LOC) | Status | Target Rust Crate/Module | Notes |"
lines << "| :--- | :--- | :--- | :--- | :--- | :--- |"

file_entries.sort_by { |e| e[:basename] }.each do |e|
  lines << "| `#{e[:basename]}` | #{e[:subsystem]} | #{number_with_delimiter(e[:loc])} | #{e[:status]} | `#{e[:target]}` | #{e[:notes]} |"
end
lines << ""

rust_crates = ["jit", "yjit", "zjit"].select { |dir| File.directory?(File.join(repo_root, dir)) }

rust_crate_entries = rust_crates.map do |crate_name|
  crate_dir = File.join(repo_root, crate_name)
  rust_files = Dir.glob(File.join(crate_dir, "**", "*.rs")).sort

  if !rust_files.empty? && system("git", "rev-parse", "--is-inside-work-tree", out: File::NULL, err: File::NULL, chdir: repo_root)
    IO.popen(["git", "-C", repo_root, "check-ignore", *rust_files]) do |io|
      ignored_files = io.read.split("\n")
      rust_files -= ignored_files
    end
  end

  crate_loc = 0
  crate_unsafe_blocks = 0
  crate_unsafe_loc = 0

  rust_files.each do |f|
    res = analyze_rust_file(f)
    crate_loc += res[:loc]
    crate_unsafe_blocks += res[:unsafe_blocks]
    crate_unsafe_loc += res[:unsafe_loc]
  end

  pct = crate_loc.positive? ? (crate_unsafe_loc.to_f / crate_loc * 100.0).round(1) : 0.0

  {
    crate: crate_name,
    file_count: rust_files.size,
    loc: crate_loc,
    unsafe_blocks: crate_unsafe_blocks,
    unsafe_loc: crate_unsafe_loc,
    pct: pct
  }
end

total_rust_files = rust_crate_entries.sum { |e| e[:file_count] }
total_rust_loc = rust_crate_entries.sum { |e| e[:loc] }
total_rust_unsafe_blocks = rust_crate_entries.sum { |e| e[:unsafe_blocks] }
total_rust_unsafe_loc = rust_crate_entries.sum { |e| e[:unsafe_loc] }
total_rust_pct = total_rust_loc.positive? ? (total_rust_unsafe_loc.to_f / total_rust_loc * 100.0).round(1) : 0.0

lines << "## Rust Safety & Unsafe Code Metrics"
lines << ""
lines << "Automated safety metrics for Rust source files across JIT workspace crates."
lines << ""
lines << "| Rust Crate / Module | Source Files | Lines of Code (LOC) | Unsafe Blocks | Unsafe Lines (LOC) | Unsafe Line Density |"
lines << "| :--- | :--- | :--- | :--- | :--- | :--- |"

rust_crate_entries.each do |e|
  lines << "| `#{e[:crate]}` | #{number_with_delimiter(e[:file_count])} | #{number_with_delimiter(e[:loc])} | #{number_with_delimiter(e[:unsafe_blocks])} | #{number_with_delimiter(e[:unsafe_loc])} | #{e[:pct]}% |"
end

lines << "| **Total Rust Workspace** | #{number_with_delimiter(total_rust_files)} | #{number_with_delimiter(total_rust_loc)} | #{number_with_delimiter(total_rust_unsafe_blocks)} | #{number_with_delimiter(total_rust_unsafe_loc)} | #{total_rust_pct}% |"
lines << ""

generated_markdown = lines.join("\n")

if check_mode
  unless File.file?(porting_md_path)
    warn "Error: PORTING.md does not exist."
    exit 1
  end

  existing_markdown = File.read(porting_md_path)

  # Line counts change with every upstream sync, so --check ignores them:
  # it masks the "Lines (LOC)", "Lines of Code (LOC)", "Unsafe Blocks",
  # "Unsafe Lines (LOC)", "Unsafe Line Density" and "% of Total LOC"
  # cells (found by their column headers) and compares everything else --
  # the file list, statuses, file counts, targets, subsystems and notes.
  mask_loc = lambda do |markdown|
    masked_columns = []
    markdown.lines.map do |line|
      unless line.start_with?("|")
        masked_columns = []
        next line
      end
      cells = line.chomp.split("|", -1)
      if cells.any? { |c| c.include?("LOC") || c.include?("Density") || c.include?("Unsafe Blocks") }
        masked_columns = cells.each_index.select { |i| cells[i].include?("LOC") || cells[i].include?("Density") || cells[i].include?("Unsafe Blocks") }
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
