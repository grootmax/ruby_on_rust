#!/usr/bin/env ruby
# frozen_string_literal: true

require "optparse"
require "json"
require "open3"
require "tempfile"
require "shellwords"
require "rbconfig"

class AbiLock
  DEFAULT_LOCKFILE = File.expand_path("abi_lock.json", __dir__)

  def initialize(options)
    @options = options
    @lockfile = options[:lockfile] || DEFAULT_LOCKFILE
    @ruby = options[:ruby] || ENV["RUBY"] || RbConfig.ruby
    @libruby = options[:libruby]
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

  def inspect_abi
    abi_info = {
      ruby_version: RbConfig::CONFIG["RUBY_PROGRAM_VERSION"] || RUBY_VERSION,
      platform: RbConfig::CONFIG["arch"] || RUBY_PLATFORM,
      sizes: dump_type_sizes,
      symbols: dump_symbols
    }
    abi_info
  end

  def dump_type_sizes
    script = <<~RUBY
      require "json"
      sizes = {
        value_size: 1.size,
        fixnum_max: (2**(8 * 1.size - 2) - 1),
        fixnum_min: (-2**(8 * 1.size - 2)),
        float_size: [0.0].pack("D").bytesize,
        endianness: [1].pack("I") == [1].pack("N") ? "big" : "little"
      }
      puts JSON.generate(sizes)
    RUBY

    temp = Tempfile.new(["abi_sizes", ".rb"])
    temp.write(script)
    temp.close

    stdout, stderr, status = Open3.capture3("#{@ruby} #{Shellwords.escape(temp.path)}")
    unless status.success?
      raise "Failed to dump ABI type sizes: #{stderr}"
    end

    JSON.parse(stdout)
  ensure
    temp&.unlink
  end

  def dump_symbols
    if @libruby && File.file?(@libruby)
      stdout, status = Open3.capture2("nm -D --defined-only #{Shellwords.escape(@libruby)}")
      return stdout.split("\n").map { |line| line.split.last }.compact.sort if status.success?
    end

    # Fallback to key C/Rust ABI entrypoint symbols expected in libruby
    %w[
      ruby_init
      ruby_options
      ruby_run_node
      ruby_cleanup
      rb_define_class
      rb_define_method
      rb_define_singleton_method
      rb_define_module
      rb_raise
      rb_funcall
      rb_str_new
      rb_ary_new
      rb_hash_new
      rb_int2big
      rb_float_new
      rb_struct_define
    ].sort
  end

  def generate_lockfile
    abi_data = inspect_abi
    File.write(@lockfile, JSON.pretty_generate(abi_data))
    puts "ABI lockfile successfully generated at: #{@lockfile}"
    true
  end

  def check_lockfile
    unless File.file?(@lockfile)
      warn "Lockfile not found at: #{@lockfile}. Run with --generate first."
      return false
    end

    locked_abi = JSON.parse(File.read(@lockfile))
    current_abi = inspect_abi

    diffs = []

    # Check sizes
    locked_sizes = locked_abi["sizes"] || {}
    curr_sizes = current_abi[:sizes] || {}

    locked_sizes.each do |k, v|
      if curr_sizes[k] != v
        diffs << "ABI size mismatch for #{k}: locked=#{v}, current=#{curr_sizes[k]}"
      end
    end

    # Check required symbols
    locked_symbols = locked_abi["symbols"] || []
    curr_symbols = current_abi[:symbols] || []

    missing_symbols = locked_symbols - curr_symbols
    unless missing_symbols.empty?
      diffs << "Missing required ABI symbols: #{missing_symbols.join(', ')}"
    end

    if diffs.empty?
      puts "ABI lock verification passed cleanly. Zero ABI drift detected against lockfile."
      true
    else
      puts "--- ABI Lock Verification Failed (#{diffs.size} divergences) ---"
      diffs.each { |d| puts " - #{d}" }
      false
    end
  end
end

if __FILE__ == $0
  options = {}
  OptionParser.new do |opts|
    opts.banner = "Usage: ruby tool/ruby_on_rust/abi_lock.rb [options]"

    opts.on("--generate", "Generate or update the ABI lockfile") { options[:generate] = true }
    opts.on("--check", "Check current ABI against the lockfile") { options[:check] = true }
    opts.on("--lockfile FILE", "Path to ABI lockfile") { |v| options[:lockfile] = v }
    opts.on("--libruby PATH", "Path to libruby shared library for symbol inspection") { |v| options[:libruby] = v }
    opts.on("--ruby PATH", "Ruby target executable") { |v| options[:ruby] = v }
    opts.on("-v", "--verbose", "Enable verbose log output") { options[:verbose] = true }
    opts.on("-h", "--help", "Show help message") do
      puts opts
      exit 0
    end
  end.parse!

  lock = AbiLock.new(options)
  success = lock.run
  exit(success ? 0 : 1)
end
