#!/usr/bin/env ruby
# frozen_string_literal: true

# Validation script for Rust SAFETY rationale comments.
# Verifies that every `unsafe` construct/keyword in target Rust source files
# is preceded by or accompanied by a `// SAFETY:` rationale comment with non-empty text.

require "fileutils"

def mask_code(content)
  masked = content.dup
  i = 0
  len = masked.length

  in_single_comment = false
  in_multi_comment = false
  in_string = false
  in_raw_string = false
  raw_hashes = 0

  while i < len
    c = masked[i]

    if in_single_comment
      if c == "\n"
        in_single_comment = false
      else
        masked[i] = " "
      end
      i += 1
      next
    end

    if in_multi_comment
      if c == "*" && i + 1 < len && masked[i + 1] == "/"
        masked[i] = " "
        masked[i + 1] = " "
        i += 2
        in_multi_comment = false
      else
        masked[i] = " " unless c == "\n"
        i += 1
      end
      next
    end

    if in_string
      if c == "\\"
        masked[i] = " "
        masked[i + 1] = " " if i + 1 < len && masked[i + 1] != "\n"
        i += 2
      elsif c == "\""
        masked[i] = " "
        in_string = false
        i += 1
      else
        masked[i] = " " unless c == "\n"
        i += 1
      end
      next
    end

    if in_raw_string
      if c == "\""
        match = true
        raw_hashes.times do |h|
          if i + 1 + h >= len || masked[i + 1 + h] != "#"
            match = false
            break
          end
        end
        if match
          masked[i] = " "
          raw_hashes.times { |h| masked[i + 1 + h] = " " }
          i += 1 + raw_hashes
          in_raw_string = false
          next
        end
      end
      masked[i] = " " unless c == "\n"
      i += 1
      next
    end

    # Detect comments and string literals
    if c == "/" && i + 1 < len && masked[i + 1] == "/"
      masked[i] = " "
      masked[i + 1] = " "
      i += 2
      in_single_comment = true
      next
    elsif c == "/" && i + 1 < len && masked[i + 1] == "*"
      masked[i] = " "
      masked[i + 1] = " "
      i += 2
      in_multi_comment = true
      next
    elsif c == "\""
      masked[i] = " "
      i += 1
      in_string = true
      next
    elsif (c == "r" || c == "b" || c == "c") && i + 1 < len && (masked[i + 1] == "\"" || masked[i + 1] == "#")
      j = i + 1
      hashes = 0
      while j < len && masked[j] == "#"
        hashes += 1
        j += 1
      end
      if j < len && masked[j] == "\""
        (i..j).each { |k| masked[k] = " " }
        i = j + 1
        in_raw_string = true
        raw_hashes = hashes
        next
      end
    end

    i += 1
  end

  masked
end

def check_file(file_path)
  unless File.file?(file_path)
    puts "Warning: File not found: #{file_path}"
    return []
  end

  content = File.read(file_path, encoding: "utf-8")
  orig_lines = content.lines
  masked_content = mask_code(content)
  masked_lines = masked_content.lines

  errors = []

  masked_lines.each_with_index do |mline, idx|
    next unless mline =~ /\bunsafe\b/

    has_safety = false

    # Check current line and preceding lines
    lookback_start = [idx - 10, 0].max
    idx.downto(lookback_start) do |i|
      if orig_lines[i] =~ %r{(?://|/\*)\s*SAFETY:\s*(\S+)}
        has_safety = true
        break
      end
      next if i == idx

      l_strip = orig_lines[i].strip
      # Stop searching if we hit non-attribute code before finding a comment
      if !l_strip.empty? && !l_strip.start_with?("//") && !l_strip.start_with?("#[") && !l_strip.start_with?("/*") && !l_strip.start_with?("*")
        break
      end
    end

    unless has_safety
      errors << "#{file_path}:#{idx + 1}: #{orig_lines[idx].strip}"
    end
  end

  errors
end

def collect_files(args)
  if args.empty?
    default_target = File.expand_path("../jit/src/lib.rs", __dir__)
    if File.file?(default_target)
      return [default_target]
    elsif File.file?("jit/src/lib.rs")
      return ["jit/src/lib.rs"]
    else
      return Dir.glob("jit/**/*.rs")
    end
  end

  files = []
  args.each do |arg|
    if File.directory?(arg)
      files.concat(Dir.glob(File.join(arg, "**/*.rs")))
    elsif File.file?(arg)
      files << arg
    else
      globbed = Dir.glob(arg)
      if globbed.empty?
        files << arg
      else
        files.concat(globbed)
      end
    end
  end
  files.uniq
end

files = collect_files(ARGV)
all_errors = []

files.each do |file|
  errors = check_file(file)
  all_errors.concat(errors)
end

if all_errors.empty?
  puts "SAFETY comment check passed for #{files.size} file(s)."
  exit 0
else
  puts "SAFETY comment check failed! Found #{all_errors.size} missing or invalid // SAFETY: comment(s):"
  all_errors.each do |err|
    puts "  #{err}"
  end
  exit 1
end
