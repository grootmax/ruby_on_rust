#!/usr/bin/env ruby
# frozen_string_literal: true

require "json"
require "optparse"
require "fileutils"
require "tmpdir"

# Core Documentation Coverage and Lock Checker for Ruby on Rust
# Audits C and Ruby core source files for RDoc documentation coverage and flags core documentation gaps.

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

class DocsLockChecker
  CORE_CLASSES = [
    Object, Kernel, Module, Class, NilClass, TrueClass, FalseClass,
    Array, Hash, String, Symbol, Numeric, Integer, Float, Rational, Complex,
    Regexp, Range, Proc, Method, UnboundMethod, Binding, Thread, Fiber, Ractor,
    Queue, SizedQueue, ConditionVariable, Mutex, Exception, StandardError,
    SystemExit, Signal, Process, GC, IO, File, Dir, Struct, Enumerable,
    Comparable, FileTest, Math, ObjectSpace
  ].freeze

  def initialize(repo_root, dump_file: nil, allowlist_file: nil)
    @repo_root = repo_root
    @dump_file = dump_file || File.join(repo_root, "tool", "ruby_on_rust", "dumps", "docs_lock.json")
    @allowlist_file = allowlist_file || File.join(repo_root, "tool", "ruby_on_rust", "allowlists", "docs_allowlist.txt")
  end

  def scan_doc_coverage
    # Scan root .c files for RDoc comments
    c_files = Dir.glob(File.join(@repo_root, "*.c")).sort
    doc_blocks = {}

    c_files.each do |c_path|
      content = File.read(c_path, encoding: "UTF-8", invalid: :replace, undef: :replace)
      # Find C RDoc blocks: /* ... */ comments before rb_define_method
      content.scan(%r{/\*([^*]|\*[^/])*\*/\s*(?:static\s+VALUE\s+|void\s+|VALUE\s+)?[A-Za-z0-9_]+\s*\([^)]*\)\s*\{}) do |match|
        block = match[0]
        if block =~ /call-seq:|Document-method:|Document-class:/i
          # Found documented block
          if block =~ /Document-method:\s*([A-Za-z0-9_:#\.-]+)/i || block =~ /call-seq:\s*([A-Za-z0-9_:#\.-]+)/i
            doc_blocks[$1.strip] = true
          end
        end
      end
    end

    total_methods = 0
    documented_methods = 0
    undocumented_list = []

    CORE_CLASSES.each do |mod|
      next unless mod.is_a?(Module)

      mod_name = mod.name || mod.to_s
      next if mod_name.nil? || mod_name.empty?

      pub_methods = mod.public_instance_methods(false).map(&:to_s)
      sing_methods = mod.singleton_methods(false).map(&:to_s)

      pub_methods.each do |m_name|
        total_methods += 1
        identifier = "#{mod_name}##{m_name}"
        # Check if documented via C RDoc scanning or reflection
        documented = doc_blocks.key?(identifier) || doc_blocks.key?(m_name)
        if documented
          documented_methods += 1
        else
          undocumented_list << identifier
        end
      end

      sing_methods.each do |m_name|
        total_methods += 1
        identifier = "#{mod_name}.#{m_name}"
        documented = doc_blocks.key?(identifier) || doc_blocks.key?(m_name)
        if documented
          documented_methods += 1
        else
          undocumented_list << identifier
        end
      end
    end

    coverage_pct = total_methods.positive? ? ((documented_methods.to_f / total_methods) * 100.0).round(2) : 100.0

    {
      "total_methods" => total_methods,
      "documented_methods" => documented_methods,
      "coverage_pct" => coverage_pct,
      "undocumented_methods" => undocumented_list.sort
    }
  end

  def update_dump
    data = scan_doc_coverage
    FileUtils.mkdir_p(File.dirname(@dump_file))
    File.write(@dump_file, JSON.pretty_generate(data) + "\n")
    puts "Docs lock dump updated at #{@dump_file}"
  end

  def check
    unless File.exist?(@dump_file)
      warn "Dump file missing at #{@dump_file}. Generating initial lock..."
      update_dump
      return true
    end

    baseline = JSON.parse(File.read(@dump_file))
    allowlist = Allowlist.new(@allowlist_file)

    current = scan_doc_coverage

    unapproved_diffs = []
    allowed_diffs = []

    # Coverage percentage regression check
    b_cov = baseline["coverage_pct"] || 0.0
    c_cov = current["coverage_pct"] || 0.0

    if c_cov < b_cov
      desc = "Core doc coverage percentage dropped: baseline=#{b_cov}%, current=#{c_cov}%"
      if allowlist.allowed?("coverage", "pct", desc)
        allowed_diffs << "[ALLOWED] #{desc}"
      else
        unapproved_diffs << "[UNAPPROVED] #{desc}"
      end
    end

    # New undocumented methods check
    b_undoc = baseline["undocumented_methods"] || []
    c_undoc = current["undocumented_methods"] || []

    new_undoc = c_undoc - b_undoc
    new_undoc.each do |m_id|
      item_type = "undocumented"
      item_id = m_id
      desc = "New undocumented core method #{m_id}"
      if allowlist.allowed?(item_type, item_id, desc)
        allowed_diffs << "[ALLOWED] #{desc}"
      else
        unapproved_diffs << "[UNAPPROVED] #{desc}"
      end
    end

    unless allowed_diffs.empty?
      puts "Allowed Documentation Lock Deviations (#{allowed_diffs.size}):"
      allowed_diffs.each { |d| puts "  #{d}" }
    end

    if unapproved_diffs.empty?
      puts "Docs Lock Verification PASSED: Documentation coverage (#{c_cov}%) meets baseline."
      true
    else
      warn "Docs Lock Verification FAILED: Found #{unapproved_diffs.size} unapproved deviation(s):"
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
    opts.banner = "Usage: ruby tool/ruby_on_rust/check_docs_lock.rb [options]"

    opts.on("--update", "-u", "Update baseline docs lock file") do
      mode = :update
    end

    opts.on("--check", "-c", "Check current docs against baseline lock file") do
      mode = :check
    end

    opts.on("--dump-file PATH", "Specify docs lock JSON dump file") do |path|
      dump_file = path
    end

    opts.on("--allowlist PATH", "Specify docs allowlist text file") do |path|
      allowlist_file = path
    end

    opts.on("-h", "--help", "Show help message") do
      puts opts
      exit 0
    end
  end.parse!

  repo_root = File.expand_path("../../", __dir__)
  checker = DocsLockChecker.new(repo_root, dump_file: dump_file, allowlist_file: allowlist_file)

  if mode == :update
    checker.update_dump
  else
    success = checker.check
    exit(success ? 0 : 1)
  end
end
