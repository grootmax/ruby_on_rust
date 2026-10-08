#!/usr/bin/env ruby
# frozen_string_literal: true

require "optparse"
require "json"
require "open3"
require "tempfile"
require "shellwords"
require "rbconfig"

class ApiLock
  DEFAULT_LOCKFILE = File.expand_path("api_lock.json", __dir__)

  TARGET_CLASSES = %w[
    Object
    Kernel
    String
    Array
    Hash
    Integer
    Float
    Range
    Struct
    Time
    Comparable
    Enumerable
    Symbol
    Numeric
    Regexp
    MatchData
    File
    IO
    Dir
    Process
    Thread
    Fiber
    Ractor
    Encoding
    Exception
  ].freeze

  def initialize(options)
    @options = options
    @lockfile = options[:lockfile] || DEFAULT_LOCKFILE
    @ruby = options[:ruby] || ENV["RUBY"] || RbConfig.ruby
    @verbose = options[:verbose]
  end

  def run
    if @options[:generate]
      generate_lockfile
    elsif @options[:check]
      check_lockfile
    else
      warn "Please specify --check or --generate"
      false
    end
  end

  private

  def dump_current_api
    script = <<~RUBY
      require "json"
      classes = #{TARGET_CLASSES.inspect}
      api_data = {}

      classes.each do |c_name|
        next unless Object.const_defined?(c_name)
        klass = Object.const_get(c_name)
        next unless klass.is_a?(Module)

        public_methods = klass.public_instance_methods(false).sort.map do |m|
          arity = begin; klass.instance_method(m).arity; rescue; nil; end
          { name: m.to_s, arity: arity }
        end

        singleton_methods = klass.singleton_class.public_instance_methods(false).sort.map do |m|
          arity = begin; klass.singleton_class.instance_method(m).arity; rescue; nil; end
          { name: m.to_s, arity: arity }
        end

        constants = klass.constants(false).map(&:to_s).sort

        api_data[c_name] = {
          public_instance_methods: public_methods,
          singleton_methods: singleton_methods,
          constants: constants
        }
      end

      puts JSON.generate(api_data)
    RUBY

    temp = Tempfile.new(["dump_api", ".rb"])
    temp.write(script)
    temp.close

    stdout, stderr, status = Open3.capture3("#{@ruby} #{Shellwords.escape(temp.path)}")
    unless status.success?
      raise "Failed to dump current API: #{stderr}"
    end

    JSON.parse(stdout)
  ensure
    temp&.unlink
  end

  def generate_lockfile
    api_data = dump_current_api
    File.write(@lockfile, JSON.pretty_generate(api_data))
    puts "API lockfile successfully generated at: #{@lockfile}"
    true
  end

  def check_lockfile
    unless File.file?(@lockfile)
      warn "Lockfile not found at: #{@lockfile}. Run with --generate first."
      return false
    end

    locked_api = JSON.parse(File.read(@lockfile))
    current_api = dump_current_api

    diffs = []

    locked_api.each do |c_name, locked_data|
      current_data = current_api[c_name]
      unless current_data
        diffs << "Missing class/module: #{c_name}"
        next
      end

      # Compare public instance methods
      locked_inst = locked_data["public_instance_methods"].map { |m| [m["name"], m["arity"]] }.to_h
      curr_inst   = current_data["public_instance_methods"].map { |m| [m["name"], m["arity"]] }.to_h

      missing_inst = locked_inst.keys - curr_inst.keys
      unless missing_inst.empty?
        diffs << "Class #{c_name} missing instance methods: #{missing_inst.join(', ')}"
      end

      # Compare singleton methods
      locked_sing = locked_data["singleton_methods"].map { |m| [m["name"], m["arity"]] }.to_h
      curr_sing   = current_data["singleton_methods"].map { |m| [m["name"], m["arity"]] }.to_h

      missing_sing = locked_sing.keys - curr_sing.keys
      unless missing_sing.empty?
        diffs << "Class #{c_name} missing singleton methods: #{missing_sing.join(', ')}"
      end
    end

    if diffs.empty?
      puts "API lock verification passed cleanly. Zero API drift detected against lockfile."
      true
    else
      puts "--- API Lock Verification Failed (#{diffs.size} divergences) ---"
      diffs.each { |d| puts " - #{d}" }
      false
    end
  end
end

if __FILE__ == $0
  options = {}
  OptionParser.new do |opts|
    opts.banner = "Usage: ruby tool/ruby_on_rust/api_lock.rb [options]"

    opts.on("--generate", "Generate or update the API lockfile") { options[:generate] = true }
    opts.on("--check", "Check current API against the lockfile") { options[:check] = true }
    opts.on("--lockfile FILE", "Path to API lockfile") { |v| options[:lockfile] = v }
    opts.on("--ruby PATH", "Ruby target executable") { |v| options[:ruby] = v }
    opts.on("-v", "--verbose", "Enable verbose log output") { options[:verbose] = true }
    opts.on("-h", "--help", "Show help message") do
      puts opts
      exit 0
    end
  end.parse!

  lock = ApiLock.new(options)
  success = lock.run
  exit(success ? 0 : 1)
end
