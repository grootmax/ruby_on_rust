#!/usr/bin/env ruby
# frozen_string_literal: true

require "json"
require "optparse"
require "fileutils"
require "tmpdir"

# Core API Lock Checker for Ruby on Rust
# Dumps and verifies core class/module reflection structures, method arities, and constants.

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

class ApiLockChecker
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
    @dump_file = dump_file || File.join(repo_root, "tool", "ruby_on_rust", "dumps", "api_lock.json")
    @allowlist_file = allowlist_file || File.join(repo_root, "tool", "ruby_on_rust", "allowlists", "api_allowlist.txt")
  end

  def capture_reflection
    data = {}
    CORE_CLASSES.each do |mod|
      next unless mod.is_a?(Module)

      mod_name = mod.name || mod.to_s
      next if mod_name.nil? || mod_name.empty?

      superclass_name = mod.is_a?(Class) && mod.superclass ? (mod.superclass.name || mod.superclass.to_s) : nil

      constants = mod.constants(false).map(&:to_s).sort

      pub_methods = mod.public_instance_methods(false).map(&:to_s).sort
      prot_methods = mod.protected_instance_methods(false).map(&:to_s).sort
      priv_methods = mod.private_instance_methods(false).map(&:to_s).sort
      sing_methods = mod.singleton_methods(false).map(&:to_s).sort

      arities = {}
      (pub_methods + prot_methods + priv_methods).each do |m_name|
        begin
          m_obj = mod.instance_method(m_name.to_sym)
          arities["#{mod_name}##{m_name}"] = m_obj.arity
        rescue StandardError
          # Ignore unbindable methods
        end
      end

      sing_methods.each do |m_name|
        begin
          m_obj = mod.method(m_name.to_sym)
          arities["#{mod_name}.#{m_name}"] = m_obj.arity
        rescue StandardError
          # Ignore unbindable methods
        end
      end

      data[mod_name] = {
        "superclass" => superclass_name,
        "constants" => constants,
        "public_instance_methods" => pub_methods,
        "protected_instance_methods" => prot_methods,
        "private_instance_methods" => priv_methods,
        "singleton_methods" => sing_methods,
        "arities" => arities
      }
    end
    data
  end

  def update_dump
    data = capture_reflection
    FileUtils.mkdir_p(File.dirname(@dump_file))
    File.write(@dump_file, JSON.pretty_generate(data) + "\n")
    puts "API lock dump updated at #{@dump_file}"
  end

  def check
    unless File.exist?(@dump_file)
      warn "Dump file missing at #{@dump_file}. Generating initial lock..."
      update_dump
      return true
    end

    baseline = JSON.parse(File.read(@dump_file))
    allowlist = Allowlist.new(@allowlist_file)

    tmp_dir = Dir.mktmpdir("api_lock")
    tmp_dump = File.join(tmp_dir, "current_api.json")
    current = capture_reflection
    File.write(tmp_dump, JSON.pretty_generate(current) + "\n")

    unapproved_diffs = []
    allowed_diffs = []

    # Diff classes/modules
    baseline_keys = baseline.keys
    current_keys = current.keys

    (baseline_keys - current_keys).each do |missing_cls|
      item_type = "class"
      item_id = missing_cls
      desc = "Missing class/module #{missing_cls}"
      if allowlist.allowed?(item_type, item_id, desc)
        allowed_diffs << "[ALLOWED] #{desc}"
      else
        unapproved_diffs << "[UNAPPROVED] #{desc}"
      end
    end

    (current_keys - baseline_keys).each do |added_cls|
      item_type = "class"
      item_id = added_cls
      desc = "Added class/module #{added_cls}"
      if allowlist.allowed?(item_type, item_id, desc)
        allowed_diffs << "[ALLOWED] #{desc}"
      else
        unapproved_diffs << "[UNAPPROVED] #{desc}"
      end
    end

    (baseline_keys & current_keys).each do |cls_name|
      b_cls = baseline[cls_name]
      c_cls = current[cls_name]

      # Constants diff
      b_consts = b_cls["constants"] || []
      c_consts = c_cls["constants"] || []

      (b_consts - c_consts).each do |const|
        item_type = "constant"
        item_id = "#{cls_name}::#{const}"
        desc = "Removed constant #{item_id}"
        if allowlist.allowed?(item_type, item_id, desc)
          allowed_diffs << "[ALLOWED] #{desc}"
        else
          unapproved_diffs << "[UNAPPROVED] #{desc}"
        end
      end

      (c_consts - b_consts).each do |const|
        item_type = "constant"
        item_id = "#{cls_name}::#{const}"
        desc = "Added constant #{item_id}"
        if allowlist.allowed?(item_type, item_id, desc)
          allowed_diffs << "[ALLOWED] #{desc}"
        else
          unapproved_diffs << "[UNAPPROVED] #{desc}"
        end
      end

      # Methods diff
      %w[public_instance_methods protected_instance_methods private_instance_methods singleton_methods].each do |m_type|
        b_methods = b_cls[m_type] || []
        c_methods = c_cls[m_type] || []
        separator = m_type == "singleton_methods" ? "." : "#"

        (b_methods - c_methods).each do |m_name|
          item_type = m_type == "singleton_methods" ? "singleton_method" : "method"
          item_id = "#{cls_name}#{separator}#{m_name}"
          desc = "Removed #{item_type} #{item_id}"
          if allowlist.allowed?(item_type, item_id, desc)
            allowed_diffs << "[ALLOWED] #{desc}"
          else
            unapproved_diffs << "[UNAPPROVED] #{desc}"
          end
        end

        (c_methods - b_methods).each do |m_name|
          item_type = m_type == "singleton_methods" ? "singleton_method" : "method"
          item_id = "#{cls_name}#{separator}#{m_name}"
          desc = "Added #{item_type} #{item_id}"
          if allowlist.allowed?(item_type, item_id, desc)
            allowed_diffs << "[ALLOWED] #{desc}"
          else
            unapproved_diffs << "[UNAPPROVED] #{desc}"
          end
        end
      end

      # Arity diff
      b_arities = b_cls["arities"] || {}
      c_arities = c_cls["arities"] || {}

      (b_arities.keys & c_arities.keys).each do |m_id|
        old_arity = b_arities[m_id]
        new_arity = c_arities[m_id]
        next if old_arity == new_arity

        item_type = "arity"
        item_id = m_id
        desc = "Arity changed for #{m_id}: baseline=#{old_arity}, current=#{new_arity}"
        if allowlist.allowed?(item_type, item_id, desc)
          allowed_diffs << "[ALLOWED] #{desc}"
        else
          unapproved_diffs << "[UNAPPROVED] #{desc}"
        end
      end
    end

    FileUtils.rm_rf(tmp_dir)

    unless allowed_diffs.empty?
      puts "Allowed API Lock Deviations (#{allowed_diffs.size}):"
      allowed_diffs.each { |d| puts "  #{d}" }
    end

    if unapproved_diffs.empty?
      puts "API Lock Verification PASSED: Core class/module reflection structures match baseline."
      true
    else
      warn "API Lock Verification FAILED: Found #{unapproved_diffs.size} unapproved deviation(s):"
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
    opts.banner = "Usage: ruby tool/ruby_on_rust/check_api_lock.rb [options]"

    opts.on("--update", "-u", "Update baseline API lock file") do
      mode = :update
    end

    opts.on("--check", "-c", "Check current API against baseline lock file") do
      mode = :check
    end

    opts.on("--dump-file PATH", "Specify API lock JSON dump file") do |path|
      dump_file = path
    end

    opts.on("--allowlist PATH", "Specify API allowlist text file") do |path|
      allowlist_file = path
    end

    opts.on("-h", "--help", "Show help message") do
      puts opts
      exit 0
    end
  end.parse!

  repo_root = File.expand_path("../../", __dir__)
  checker = ApiLockChecker.new(repo_root, dump_file: dump_file, allowlist_file: allowlist_file)

  if mode == :update
    checker.update_dump
  else
    success = checker.check
    exit(success ? 0 : 1)
  end
end
