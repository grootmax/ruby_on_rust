#!/usr/bin/env ruby
# frozen_string_literal: true

require "yaml"
require "optparse"
require "set"

VALID_STATUSES = ["Not Started", "In Progress", "Ported", "Blocked", "N/A"].freeze

def number_with_delimiter(number)
  number.to_s.gsub(/(\d)(?=(\d\d\d)+(?!\d))/, '\1,')
end

def scan_rust_file(file_path)
  content = File.read(file_path)
  lines = content.lines
  total_loc = lines.size

  unsafe_blocks_count = 0
  unsafe_fn_count = 0
  unsafe_lines_set = Set.new

  in_block_comment_depth = 0
  in_string = false
  raw_string_hashes = nil

  brace_depth = 0
  unsafe_stack = []
  pending_unsafe_count = 0

  lines.each_with_index do |line_str, line_idx|
    line_number = line_idx + 1

    i = 0
    chars = line_str.chars
    len = chars.size

    while i < len
      c = chars[i]

      if in_block_comment_depth > 0
        if c == "/" && i + 1 < len && chars[i + 1] == "*"
          in_block_comment_depth += 1
          i += 2
          next
        elsif c == "*" && i + 1 < len && chars[i + 1] == "/"
          in_block_comment_depth -= 1
          i += 2
          next
        else
          i += 1
          next
        end
      end

      if raw_string_hashes
        if c == "\""
          matching = true
          raw_string_hashes.times do |h_idx|
            if i + 1 + h_idx >= len || chars[i + 1 + h_idx] != "#"
              matching = false
              break
            end
          end
          if matching
            i += 1 + raw_string_hashes
            raw_string_hashes = nil
            next
          end
        end
        i += 1
        next
      end

      if in_string
        if c == "\\"
          i += 2
          next
        elsif c == "\""
          in_string = false
          i += 1
          next
        else
          i += 1
          next
        end
      end

      if c == "/" && i + 1 < len && chars[i + 1] == "/"
        break
      end

      if c == "/" && i + 1 < len && chars[i + 1] == "*"
        in_block_comment_depth += 1
        i += 2
        next
      end

      if c == "r" && i + 1 < len && (chars[i + 1] == "\"" || chars[i + 1] == "#")
        j = i + 1
        hashes = 0
        while j < len && chars[j] == "#"
          hashes += 1
          j += 1
        end
        if j < len && chars[j] == "\""
          raw_string_hashes = hashes
          i = j + 1
          next
        end
      end

      if c == "\""
        in_string = true
        i += 1
        next
      end

      if c == "'"
        if i + 2 < len && chars[i + 2] == "'" && chars[i + 1] != "\\"
          i += 3
          next
        elsif i + 3 < len && chars[i + 3] == "'" && chars[i + 1] == "\\"
          i += 4
          next
        end
        i += 1
        next
      end

      if c == "u" && (i == 0 || !chars[i - 1].match?(/[a-zA-Z0-9_]/))
        if line_str[i..].match?(/\Aunsafe\b/)
          unsafe_blocks_count += 1
          pending_unsafe_count += 1
          rest = line_str[(i + 6)..]
          if rest.match?(/\A\s*(?:async\s+|const\s+|extern\s+(?:"[^"]*"\s+)?)*fn\b/)
            unsafe_fn_count += 1
          end
          i += 6
          next
        end
      end

      if c == "{"
        if pending_unsafe_count > 0
          pending_unsafe_count.times do
            unsafe_stack.push(brace_depth + 1)
          end
          pending_unsafe_count = 0
        end
        brace_depth += 1
        i += 1
        next
      end

      if c == "}"
        if brace_depth > 0
          brace_depth -= 1
          while !unsafe_stack.empty? && unsafe_stack.last > brace_depth
            unsafe_stack.pop
          end
        end
        i += 1
        next
      end

      if c == ";"
        if pending_unsafe_count > 0 && brace_depth == 0
          pending_unsafe_count = 0
        end
        i += 1
        next
      end

      i += 1
    end

    if !unsafe_stack.empty? || pending_unsafe_count > 0
      unsafe_lines_set.add(line_number)
    end
  end

  {
    loc: total_loc,
    unsafe_blocks: unsafe_blocks_count,
    unsafe_fns: unsafe_fn_count,
    unsafe_lines: unsafe_lines_set.size
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

# Rust Workspace Unsafe Code Scanner
rust_crate_dirs = {
  "core_rs" => Dir.glob(File.join(repo_root, "core_rs", "**", "*.rs")),
  "jit" => Dir.glob(File.join(repo_root, "jit", "**", "*.rs")),
  "ruby" => Dir.glob(File.join(repo_root, "ruby.rs")),
  "yjit" => Dir.glob(File.join(repo_root, "yjit", "**", "*.rs")),
  "zjit" => Dir.glob(File.join(repo_root, "zjit", "**", "*.rs"))
}

rust_metrics = []
grand_rust_files = 0
grand_rust_loc = 0
grand_unsafe_blocks = 0
grand_unsafe_fns = 0
grand_unsafe_lines = 0

rust_crate_dirs.keys.sort.each do |crate_name|
  files = rust_crate_dirs[crate_name].reject { |f| f.include?("/target/") }.sort
  c_totals = { files: files.size, loc: 0, unsafe_blocks: 0, unsafe_fns: 0, unsafe_lines: 0 }
  files.each do |f|
    res = scan_rust_file(f)
    c_totals[:loc] += res[:loc]
    c_totals[:unsafe_blocks] += res[:unsafe_blocks]
    c_totals[:unsafe_fns] += res[:unsafe_fns]
    c_totals[:unsafe_lines] += res[:unsafe_lines]
  end

  density = c_totals[:loc].positive? ? (c_totals[:unsafe_lines].to_f / c_totals[:loc] * 100.0).round(1) : 0.0

  grand_rust_files += c_totals[:files]
  grand_rust_loc += c_totals[:loc]
  grand_unsafe_blocks += c_totals[:unsafe_blocks]
  grand_unsafe_fns += c_totals[:unsafe_fns]
  grand_unsafe_lines += c_totals[:unsafe_lines]

  rust_metrics << {
    name: crate_name,
    files: c_totals[:files],
    loc: c_totals[:loc],
    unsafe_blocks: c_totals[:unsafe_blocks],
    unsafe_fns: c_totals[:unsafe_fns],
    unsafe_lines: c_totals[:unsafe_lines],
    density: density
  }
end

grand_density = grand_rust_loc.positive? ? (grand_unsafe_lines.to_f / grand_rust_loc * 100.0).round(1) : 0.0

lines << ""
lines << "## Rust Unsafe Code Density"
lines << ""
lines << "| Crate / Module | Files | Lines of Code (LOC) | Unsafe Blocks | Unsafe Functions | Unsafe Lines | Unsafe Line Density |"
lines << "| :--- | :--- | :--- | :--- | :--- | :--- | :--- |"

rust_metrics.each do |m|
  lines << "| `#{m[:name]}` | #{number_with_delimiter(m[:files])} | #{number_with_delimiter(m[:loc])} | #{number_with_delimiter(m[:unsafe_blocks])} | #{number_with_delimiter(m[:unsafe_fns])} | #{number_with_delimiter(m[:unsafe_lines])} | #{m[:density]}% |"
end

lines << "| **Total** | **#{number_with_delimiter(grand_rust_files)}** | **#{number_with_delimiter(grand_rust_loc)}** | **#{number_with_delimiter(grand_unsafe_blocks)}** | **#{number_with_delimiter(grand_unsafe_fns)}** | **#{number_with_delimiter(grand_unsafe_lines)}** | **#{grand_density}%** |"

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

generated_markdown = lines.join("\n")

if check_mode
  unless File.file?(porting_md_path)
    warn "Error: PORTING.md does not exist."
    exit 1
  end

  existing_markdown = File.read(porting_md_path)

  # Line counts change with every upstream sync, so --check ignores them:
  # it masks the "Lines (LOC)", "Lines of Code (LOC)" and "% of Total LOC"
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
