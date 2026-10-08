#!/usr/bin/env ruby
# frozen_string_literal: true

require "optparse"
require "json"
require "digest"

class DocsLock
  DEFAULT_LOCKFILE = File.expand_path("docs_lock.json", __dir__)
  REPO_ROOT = File.expand_path("../..", __dir__)

  REQUIRED_DOCS = %w[
    COMPATIBILITY.md
    PORTING.md
    README.md
    AGENTS.md
  ].freeze

  def initialize(options)
    @options = options
    @lockfile = options[:lockfile] || DEFAULT_LOCKFILE
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

  def inspect_docs
    docs_data = {}
    REQUIRED_DOCS.each do |doc|
      full_path = File.join(REPO_ROOT, doc)
      if File.file?(full_path)
        content = File.read(full_path, encoding: "utf-8")
        docs_data[doc] = {
          exists: true,
          lines: content.lines.count,
          sha256: Digest::SHA256.hexdigest(content),
          headers: content.scan(/^#+\s+.*/).map(&:strip)
        }
      else
        docs_data[doc] = { exists: false }
      end
    end
    docs_data
  end

  def generate_lockfile
    docs_data = inspect_docs
    File.write(@lockfile, JSON.pretty_generate(docs_data))
    puts "Docs lockfile successfully generated at: #{@lockfile}"
    true
  end

  def check_lockfile
    unless File.file?(@lockfile)
      warn "Lockfile not found at: #{@lockfile}. Run with --generate first."
      return false
    end

    locked_docs = JSON.parse(File.read(@lockfile))
    current_docs = inspect_docs

    diffs = []

    locked_docs.each do |doc, locked_info|
      curr_info = current_docs[doc]
      unless curr_info && curr_info[:exists]
        diffs << "Missing required documentation file: #{doc}"
        next
      end

      # Verify header structures
      locked_headers = locked_info["headers"] || []
      curr_headers = curr_info[:headers] || []

      missing_headers = locked_headers - curr_headers
      unless missing_headers.empty?
        diffs << "Doc [#{doc}] missing required headers: #{missing_headers.join(', ')}"
      end
    end

    if diffs.empty?
      puts "Docs lock verification passed cleanly. Zero documentation structure drift detected."
      true
    else
      puts "--- Docs Lock Verification Failed (#{diffs.size} divergences) ---"
      diffs.each { |d| puts " - #{d}" }
      false
    end
  end
end

if __FILE__ == $0
  options = {}
  OptionParser.new do |opts|
    opts.banner = "Usage: ruby tool/ruby_on_rust/docs_lock.rb [options]"

    opts.on("--generate", "Generate or update the Docs lockfile") { options[:generate] = true }
    opts.on("--check", "Check documentation locks against lockfile") { options[:check] = true }
    opts.on("--lockfile FILE", "Path to Docs lockfile") { |v| options[:lockfile] = v }
    opts.on("-v", "--verbose", "Enable verbose log output") { options[:verbose] = true }
    opts.on("-h", "--help", "Show help message") do
      puts opts
      exit 0
    end
  end.parse!

  lock = DocsLock.new(options)
  success = lock.run
  exit(success ? 0 : 1)
end
