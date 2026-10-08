#!/usr/bin/env ruby
# frozen_string_literal: true

require "optparse"

class SymbolFilterGenerator
  attr_reader :patterns, :prefix, :format

  def initialize(manifest_path, prefix: "", format: "objcopy")
    @prefix = prefix || ""
    @format = (format || "objcopy").to_s.tr("-", "_")
    @patterns = load_manifest(manifest_path)
  end

  def load_manifest(path)
    return [] unless path && File.exist?(path)

    File.readlines(path, encoding: "UTF-8").filter_map do |line|
      line = line.strip
      next if line.empty? || line.start_with?("#")
      # rust_eh_personality is required on Darwin (where symbol prefix is '_'),
      # but must stay hidden on Linux to prevent symbol leaking in libruby.so.
      next if line == "rust_eh_personality" && @prefix.empty?

      "#{@prefix}#{line}"
    end
  end

  def pattern_regexps
    @pattern_regexps ||= @patterns.map do |pat|
      /\A#{Regexp.escape(pat).gsub('\*', '.*')}\z/
    end
  end

  def match_symbol?(sym)
    pattern_regexps.any? { |re| re.match?(sym) }
  end

  def parse_nm_input(input)
    matched = []
    lines = input.respond_to?(:each_line) ? input.each_line : input.to_s.lines
    lines.each do |line|
      line = line.strip
      next if line.empty?

      words = line.split
      sym = words.last
      next if sym.nil? || sym.end_with?(":")

      if words.size >= 2
        type_char = words[-2]
        next if type_char.length == 1 && type_char =~ /[a-zU]/
      end

      if match_symbol?(sym)
        matched << sym unless matched.include?(sym)
      end
    end
    matched
  end

  def generate(input = nil)
    case @format
    when "objcopy"
      @patterns.map { |pat| "--keep-global-symbol=#{pat}" }.join(" ")
    when "ld_u", "ld_u_flags", "u"
      syms = parse_nm_input(input || $stdin)
      syms.map { |s| "-u #{s}" }.join(" ")
    when "symbols", "exported_symbols_list", "list"
      syms = parse_nm_input(input || $stdin)
      syms.join("\n")
    else
      raise ArgumentError, "Unknown format: #{@format}"
    end
  end
end

if __FILE__ == $0
  options = {
    prefix: "",
    format: "objcopy",
    manifest: nil
  }

  parser = OptionParser.new do |opts|
    opts.banner = "Usage: gen_symbol_filter.rb [options] [manifest_file]"

    opts.on("-f", "--format FORMAT", "Output format: objcopy, ld-u, symbols") do |f|
      options[:format] = f
    end

    opts.on("-p", "--prefix PREFIX", "Symbol prefix (e.g. '_' or '')") do |p|
      options[:prefix] = p
    end

    opts.on("-m", "--manifest FILE", "Manifest file path") do |m|
      options[:manifest] = m
    end
  end

  parser.parse!

  manifest_file = options[:manifest] || ARGV[0] || File.expand_path("../defs/rust_exported_symbols.sym", __dir__)
  generator = SymbolFilterGenerator.new(manifest_file, prefix: options[:prefix], format: options[:format])
  output = generator.generate
  puts output unless output.empty?
end
