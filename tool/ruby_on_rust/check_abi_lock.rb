#!/usr/bin/env ruby
# frozen_string_literal: true

require "json"
require "optparse"
require "fileutils"
require "tmpdir"
require "digest"

# C ABI Symbol, Header Macro, and Struct Offset Checker for Ruby on Rust
# Audits include/ruby/*.h headers, exported C symbols, and core struct field layouts.

class Allowlist
  Entry = Struct.new(:type, :identifier, :reason, :raw_pattern)

  def initialize(file_path)
    @entries = []
    return unless file_path && File.exist?(file_path)

    File.foreach(file_path) do |line|
      line = line.strip
      next if line.empty? || line.start_with?("#")

      if line.include?(":")
        type, rest = line.split(":", 2)
        type = type.strip
        identifier, reason = rest.split(/\s+#\s*/, 2)
        identifier = identifier ? identifier.strip : ""
        reason = reason ? reason.strip : ""
        @entries << Entry.new(type, identifier, reason, line) unless identifier.empty?
      else
        pattern, reason = line.split(/\s+#\s*/, 2)
        pattern = pattern ? pattern.strip : ""
        reason = reason ? reason.strip : ""
        @entries << Entry.new(nil, pattern, reason, line) unless pattern.empty?
      end
    end
  end

  def allowed?(item_type, item_identifier, item_description = nil)
    @entries.any? do |entry|
      if entry.type
        type_match = (entry.type == "*" || entry.type == item_type.to_s)
        id_match = (entry.identifier == "*" || entry.identifier == item_identifier.to_s ||
                    File.fnmatch?(entry.identifier, item_identifier.to_s) ||
                    (entry.identifier.start_with?("/") && entry.identifier.end_with?("/") && Regexp.new(entry.identifier[1..-2]).match?(item_identifier.to_s)))
        type_match && id_match
      else
        pattern = entry.identifier
        item_identifier.to_s == pattern ||
          (item_description && item_description.include?(pattern)) ||
          File.fnmatch?(pattern, item_identifier.to_s) ||
          (pattern.start_with?("/") && pattern.end_with?("/") && Regexp.new(pattern[1..-2]).match?(item_identifier.to_s))
      end
    end
  end
end

class AbiLockChecker
  CORE_STRUCTS = %w[
    RBasic RObject RClass RFloat RString RArray RRegexp
    RHash RFile RData RTypedData RStruct RBignum
  ].freeze

  def initialize(repo_root, dump_file: nil, allowlist_file: nil)
    @repo_root = repo_root
    @dump_file = dump_file || File.join(repo_root, "tool", "ruby_on_rust", "dumps", "abi_lock.json")
    @allowlist_file = allowlist_file || File.join(repo_root, "tool", "ruby_on_rust", "allowlists", "abi_allowlist.txt")
  end

  def capture_abi_state
    headers_data = scan_headers
    symbols_data = scan_symbols
    structs_data = scan_structs

    {
      "headers" => headers_data,
      "symbols" => symbols_data,
      "structs" => structs_data
    }
  end

  def scan_headers
    headers = {}
    include_dir = File.join(@repo_root, "include")
    return headers unless Dir.exist?(include_dir)

    header_files = Dir.glob(File.join(include_dir, "**", "*.h")).sort
    header_files.each do |h_path|
      rel_path = h_path.sub("#{@repo_root}/", "")
      content = File.read(h_path)
      checksum = Digest::SHA256.hexdigest(content)

      # Extract macro definitions
      macros = []
      content.scan(/^\s*#\s*define\s+([A-Za-z0-9_]+)/) do |match|
        macros << match[0]
      end

      headers[rel_path] = {
        "checksum" => checksum,
        "macros" => macros.uniq.sort
      }
    end
    headers
  end

  def scan_symbols
    symbols = []
    
    # Check if libruby shared object exists and nm can run
    so_paths = Dir.glob(File.join(@repo_root, "libruby.so*")) + Dir.glob(File.join(@repo_root, "libruby.dylib*"))
    if !so_paths.empty? && system("which nm > /dev/null 2>&1")
      so_file = so_paths.first
      begin
        out = `nm -D --defined-only "#{so_file}" 2>/dev/null`
        out.each_line do |line|
          if line =~ /[TDB]\s+(r[b|uby]_[A-Za-z0-9_]+)$/
            symbols << $1
          end
        end
      rescue StandardError
      end
    end

    # Fallback/complement: extract function declarations starting with rb_ or ruby_ in include/
    include_dir = File.join(@repo_root, "include")
    if Dir.exist?(include_dir)
      Dir.glob(File.join(include_dir, "**", "*.h")).each do |h_path|
        content = File.read(h_path)
        content.scan(/\b(rb_|ruby_)[A-Za-z0-9_]+\b/) do |match|
          sym = match[0]
          # Exclude macro names or simple preprocessor definitions
          symbols << sym unless sym.upcase == sym
        end
      end
    end

    symbols.uniq.sort
  end

  def scan_structs
    struct_layouts = {}
    include_dir = File.join(@repo_root, "include")

    all_header_contents = +""
    if Dir.exist?(include_dir)
      Dir.glob(File.join(include_dir, "**", "*.h")).each do |h_path|
        all_header_contents << "\n" << File.read(h_path)
      end
    end

    CORE_STRUCTS.each do |st_name|
      fields = []
      # Match struct R... { ... } definition
      if all_header_contents =~ /struct\s+#{st_name}\s*\{([^}]+)\}/m
        body = $1
        body.each_line do |line|
          line = line.strip
          next if line.empty? || line.start_with?("//") || line.start_with?("/*") || line.start_with?("#")
          if line =~ /([A-Za-z0-9_]+)\s*;\s*$/
            fields << $1
          end
        end
      end

      struct_layouts[st_name] = fields
    end

    struct_layouts
  end

  def update_dump
    data = capture_abi_state
    FileUtils.mkdir_p(File.dirname(@dump_file))
    File.write(@dump_file, JSON.pretty_generate(data) + "\n")
    puts "ABI lock dump updated at #{@dump_file}"
  end

  def check
    unless File.exist?(@dump_file)
      warn "Dump file missing at #{@dump_file}. Generating initial lock..."
      update_dump
      return true
    end

    baseline = JSON.parse(File.read(@dump_file))
    allowlist = Allowlist.new(@allowlist_file)

    current = capture_abi_state

    unapproved_diffs = []
    allowed_diffs = []

    # 1. Header & Macro Diffs
    b_headers = baseline["headers"] || {}
    c_headers = current["headers"] || {}

    (b_headers.keys - c_headers.keys).each do |missing_h|
      item_type = "header"
      item_id = missing_h
      desc = "Missing C header file #{missing_h}"
      if allowlist.allowed?(item_type, item_id, desc)
        allowed_diffs << "[ALLOWED] #{desc}"
      else
        unapproved_diffs << "[UNAPPROVED] #{desc}"
      end
    end

    (b_headers.keys & c_headers.keys).each do |h_path|
      b_macros = b_headers[h_path]["macros"] || []
      c_macros = c_headers[h_path]["macros"] || []

      (b_macros - c_macros).each do |missing_macro|
        item_type = "macro"
        item_id = "#{h_path}:#{missing_macro}"
        desc = "Removed header macro #{missing_macro} in #{h_path}"
        if allowlist.allowed?(item_type, item_id, desc)
          allowed_diffs << "[ALLOWED] #{desc}"
        else
          unapproved_diffs << "[UNAPPROVED] #{desc}"
        end
      end
    end

    # 2. Exported Symbol Diffs
    b_symbols = baseline["symbols"] || []
    c_symbols = current["symbols"] || []

    (b_symbols - c_symbols).each do |missing_sym|
      item_type = "symbol"
      item_id = missing_sym
      desc = "Missing exported C ABI symbol #{missing_sym}"
      if allowlist.allowed?(item_type, item_id, desc)
        allowed_diffs << "[ALLOWED] #{desc}"
      else
        unapproved_diffs << "[UNAPPROVED] #{desc}"
      end
    end

    # 3. Struct Layout & Offset Diffs
    b_structs = baseline["structs"] || {}
    c_structs = current["structs"] || {}

    b_structs.each do |st_name, b_fields|
      c_fields = c_structs[st_name] || []
      if b_fields != c_fields
        item_type = "struct"
        item_id = st_name
        desc = "Struct layout or field offset mismatch for #{st_name}: baseline=#{b_fields.inspect}, current=#{c_fields.inspect}"
        if allowlist.allowed?(item_type, item_id, desc)
          allowed_diffs << "[ALLOWED] #{desc}"
        else
          unapproved_diffs << "[UNAPPROVED] #{desc}"
        end
      end
    end

    unless allowed_diffs.empty?
      puts "Allowed ABI Lock Deviations (#{allowed_diffs.size}):"
      allowed_diffs.each { |d| puts "  #{d}" }
    end

    if unapproved_diffs.empty?
      puts "ABI Lock Verification PASSED: C ABI headers, macros, symbols, and struct offsets match baseline."
      true
    else
      warn "ABI Lock Verification FAILED: Found #{unapproved_diffs.size} unapproved deviation(s):"
      unapproved_diffs.each { |d| warn "  #{d}" }
      false
    end
  end
end

if __FILE__ == $0
  mode = :check
  dump_file = nil
  allowlist_file = nil

  OptionParser.new do |opts|
    opts.banner = "Usage: ruby tool/ruby_on_rust/check_abi_lock.rb [options]"

    opts.on("--update", "-u", "Update baseline ABI lock file") do
      mode = :update
    end

    opts.on("--check", "-c", "Check current ABI against baseline lock file") do
      mode = :check
    end

    opts.on("--dump-file PATH", "Specify ABI lock JSON dump file") do |path|
      dump_file = path
    end

    opts.on("--allowlist PATH", "Specify ABI allowlist text file") do |path|
      allowlist_file = path
    end

    opts.on("-h", "--help", "Show help message") do
      puts opts
      exit 0
    end
  end.parse!

  repo_root = File.expand_path("../../", __dir__)
  checker = AbiLockChecker.new(repo_root, dump_file: dump_file, allowlist_file: allowlist_file)

  if mode == :update
    checker.update_dump
  else
    success = checker.check
    exit(success ? 0 : 1)
  end
end
