#!/usr/bin/env ruby
# frozen_string_literal: true

require "optparse"
require "set"
require "rbconfig"

class MethodKindsInspector
  CORE_CLASSES = [
    Object, Module, Class, Kernel, NilClass, TrueClass, FalseClass,
    Numeric, Integer, Float, String, Symbol, Array, Hash, Range, Regexp,
    Proc, Thread, Fiber, IO, File, Dir, Process, Exception, Time, Struct, Math,
    Enumerable, Encoding, MatchData, Method, UnboundMethod, Binding, Enumerator,
    Rational, Complex, GC, ObjectSpace
  ].freeze

  attr_reader :options, :violations, :inspected_modules, :total_methods, :c_methods, :ruby_methods

  def initialize(options = {})
    @options = options
    @violations = []
    @inspected_modules = Set.new
    @total_methods = 0
    @c_methods = 0
    @ruby_methods = 0

    repo_root = File.expand_path("..", __dir__)
    raw_dirs = [
      RbConfig::CONFIG["prefix"],
      RbConfig::CONFIG["rubylibdir"],
      RbConfig::CONFIG["archlibdir"],
      RbConfig::CONFIG["vendordir"],
      RbConfig::CONFIG["sitedir"],
      RbConfig::CONFIG["sitelibdir"],
      RbConfig::CONFIG["vendorlibdir"],
      RbConfig::CONFIG["sitearchlibdir"],
      RbConfig::CONFIG["vendorarchlibdir"],
      RbConfig::CONFIG["rubyhdrdir"],
      RbConfig::CONFIG["rubyarchhdrdir"],
      RbConfig::CONFIG["libdir"],
      File.join(repo_root, "lib"),
      File.join(repo_root, "prelude"),
      File.join(repo_root, "ast")
    ]
    if defined?(Gem)
      raw_dirs << Gem.dir if Gem.respond_to?(:dir)
      raw_dirs << Gem.default_dir if Gem.respond_to?(:default_dir)
      raw_dirs.concat(Gem.path) if Gem.respond_to?(:path) && Gem.path.is_a?(Array)
    end

    dirs = Set.new
    raw_dirs.compact.reject(&:empty?).each do |d|
      dirs.add(d)
      begin
        dirs.add(File.expand_path(d))
      rescue StandardError
      end
      begin
        dirs.add(File.realpath(d))
      rescue StandardError
      end
    end
    @stdlib_dirs = dirs.to_a
  end

  def system_library?(path)
    return true if path.start_with?("<")
    return true if path.include?("rubygems") || path.include?("bundler")

    expanded_path = begin
      File.expand_path(path)
    rescue StandardError
      path
    end

    real_path = begin
      File.realpath(path)
    rescue StandardError
      expanded_path
    end

    @stdlib_dirs.any? do |dir|
      path.start_with?(dir) ||
        expanded_path.start_with?(dir) ||
        real_path.start_with?(dir)
    end
  end

  def discover_targets
    targets = []

    if options[:target_class]
      target_name = options[:target_class]
      begin
        klass = Object.const_get(target_name)
        targets << klass
        targets << klass.singleton_class if klass.respond_to?(:singleton_class)
      rescue NameError
        warn "Warning: Target class '#{target_name}' not found."
      end
      return targets
    end

    # Discover built-in modules and classes
    ObjectSpace.each_object(Module) do |mod|
      next if mod.name&.start_with?("MethodKindsInspector")
      targets << mod
      begin
        targets << mod.singleton_class
      rescue StandardError
        # Skip if singleton class cannot be retrieved
      end
    end

    # Discover singletons of built-in objects
    [ENV, ARGF, TOPLEVEL_BINDING].each do |obj|
      if obj.respond_to?(:singleton_class)
        targets << obj.singleton_class
      end
    end

    targets.uniq
  end

  def inspect_method(target, method_name, is_singleton: false)
    begin
      unbound_method = target.instance_method(method_name)
    rescue NameError
      return
    end

    loc = unbound_method.source_location
    iseq = nil
    if defined?(RubyVM::InstructionSequence)
      begin
        iseq = RubyVM::InstructionSequence.of(unbound_method)
      rescue StandardError
        iseq = nil
      end
    end

    @total_methods += 1

    is_c_method = loc.nil? && iseq.nil?

    if is_c_method
      @c_methods += 1
    else
      @ruby_methods += 1
    end

    # Determine if this target is a core class/module or its singleton
    target_mod = begin
      is_singleton ? target.attached_object : target
    rescue StandardError
      target
    end
    is_core_target = CORE_CLASSES.include?(target) || CORE_CLASSES.include?(target_mod)

    # Rule 7 Check:
    # If a core C method is implemented in non-internal/non-system Ruby code, flag violation.
    if is_core_target && !is_c_method
      loc_str = loc ? "#{loc[0]}:#{loc[1]}" : "iseq present"
      if loc && !system_library?(loc[0])
        @violations << {
          target: target.to_s,
          method: method_name,
          location: loc_str,
          type: "Rule 7 violation: C/Rust core method shifted to Ruby code"
        }
      end
    end

    if options[:verbose]
      kind_str = is_c_method ? "C/Rust" : "Ruby (#{loc ? "#{loc[0]}:#{loc[1]}" : 'iseq'})"
      puts "  #{target}##{method_name} => #{kind_str}"
    end
  end

  def run
    start_time = Process.clock_gettime(Process::CLOCK_MONOTONIC)
    targets = discover_targets

    targets.each do |target|
      @inspected_modules.add(target)
      is_singleton = begin
        target.singleton_class?
      rescue StandardError
        false
      end

      methods = []
      methods.concat(target.instance_methods(false)) rescue nil
      methods.concat(target.protected_instance_methods(false)) rescue nil
      methods.concat(target.private_instance_methods(false)) rescue nil

      methods.uniq.compact.each do |m|
        inspect_method(target, m, is_singleton: is_singleton)
      end
    end

    elapsed = Process.clock_gettime(Process::CLOCK_MONOTONIC) - start_time

    puts "Method Kinds Reflection Summary:"
    puts "  Modules/Classes Inspected : #{@inspected_modules.size}"
    puts "  Total Methods Scanned     : #{@total_methods}"
    puts "  C/Rust Native Methods     : #{@c_methods}"
    puts "  Ruby-Defined Methods      : #{@ruby_methods}"
    puts "  Rule 7 Violations         : #{@violations.size}"
    puts "  Elapsed Time              : #{format('%.3f', elapsed)}s"

    unless @violations.empty?
      puts "\nVIOLATIONS DETECTED:"
      @violations.each do |v|
        puts "  - #{v[:target]}##{v[:method]} (#{v[:location]}): #{v[:type]}"
      end
      return false
    end

    true
  end
end

if __FILE__ == $0
  options = {
    verbose: false,
    check: true,
    target_class: nil
  }

  OptionParser.new do |opts|
    opts.banner = "Usage: ruby tool/check_method_kinds.rb [options]"

    opts.on("-v", "--verbose", "Output detailed method registration kinds") do
      options[:verbose] = true
    end

    opts.on("-c", "--class CLASS", "Inspect specific class or module") do |klass|
      options[:target_class] = klass
    end

    opts.on("--check", "Check for Rule 7 method kind violations (default)") do
      options[:check] = true
    end

    opts.on("-h", "--help", "Show help") do
      puts opts
      exit 0
    end
  end.parse!

  inspector = MethodKindsInspector.new(options)
  success = inspector.run
  exit(success ? 0 : 1)
end
