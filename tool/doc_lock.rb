#!/usr/bin/env ruby
# frozen_string_literal: true

# RDoc native store structural diff and verification tool for Ruby on Rust.
# Compares core RDoc and RI native stores (.ext/rdoc/ cache.ri and .ri files)
# between reference CRuby and Ruby on Rust to detect documentation drift.

require 'optparse'
require 'json'
require 'yaml'
require 'fileutils'
require 'rdoc'
require 'rdoc/markup/to_rdoc'

module DocLock
  # Formatter subclass that safely formats RDoc markup tables
  class SafeToRdoc < RDoc::Markup::ToRdoc
    def accept_table(header, body, aligns)
      header ||= []
      body ||= []
      aligns ||= []
      widths = header.map(&:size)
      body.each do |row|
        next unless row
        row.each_with_index do |col, i|
          col_len = col ? col.size : 0
          widths[i] = [widths[i] || 0, col_len].max
        end
      end
      align_list = widths.map.with_index do |_, i|
        case aligns[i]
        when :left then :ljust
        when :right then :rjust
        else :center
        end
      end
      if header && !header.empty?
        @res << header.zip(widths, align_list).map { |h, w, a| (h || '').to_s.__send__(a, w || 0) }.join('|').rstrip << "\n"
        @res << widths.map { |w| '-' * (w || 0) }.join('|') << "\n"
      end
      body.each do |row|
        next unless row
        @res << row.zip(widths, align_list).map { |t, w, a| (t || '').to_s.__send__(a, w || 0) }.join('|').rstrip << "\n"
      end
    end
  end

  # Normalizes whitespace in docstrings and call-seq declarations
  def self.normalize_whitespace(text)
    return '' if text.nil?

    text.to_s.gsub(/\r\n/, "\n")
        .lines
        .map(&:rstrip)
        .join("\n")
        .gsub(/\n{3,}/, "\n\n")
        .strip
  end

  # Extracts text from RDoc comment or document objects
  def self.extract_text(doc, formatter)
    return '' if doc.nil?

    doc = doc.text if doc.is_a?(RDoc::Comment)
    if doc.respond_to?(:accept)
      begin
        text = doc.accept(formatter)
      rescue StandardError
        text = doc.to_s.gsub(/0x[0-9a-fA-F]+/, '0x0')
      end
      normalize_whitespace(text)
    else
      normalize_whitespace(doc.to_s.gsub(/0x[0-9a-fA-F]+/, '0x0'))
    end
  end

  # Parses RDoc store directory and extracts structured JSON-compatible metadata
  class StoreParser
    def initialize(store_dir)
      @store_dir = File.expand_path(store_dir)
    end

    def parse
      unless File.directory?(@store_dir)
        raise Errno::ENOENT, "RDoc store directory not found at #{@store_dir}"
      end

      cache_file = File.join(@store_dir, 'cache.ri')
      unless File.file?(cache_file)
        raise Errno::ENOENT, "RDoc cache.ri not found at #{cache_file}"
      end

      store = RDoc::Store.new(@store_dir)
      cache_data = Marshal.load(File.binread(cache_file))
      all_names = ((cache_data[:modules] || []) + (cache_data[:classes] || [])).uniq.sort

      formatter = SafeToRdoc.new
      data = {}

      all_names.each do |name|
        cm = store.load_class(name)
        next unless cm

        methods_data = {}
        cm.method_list.each do |m|
          prefix = m.singleton ? '::' : '#'
          full_m = store.load_method(cm.full_name, "#{prefix}#{m.name}")
          next unless full_m

          methods_data[full_m.full_name] = {
            'name' => full_m.name,
            'full_name' => full_m.full_name,
            'singleton' => !!full_m.singleton,
            'visibility' => full_m.visibility.to_s,
            'params' => full_m.params.to_s.strip,
            'call_seq' => DocLock.normalize_whitespace(full_m.call_seq),
            'comment' => DocLock.extract_text(full_m.comment, formatter)
          }
        end

        data[cm.full_name] = {
          'name' => cm.full_name,
          'type' => cm.is_a?(RDoc::NormalModule) ? 'module' : 'class',
          'comment' => DocLock.extract_text(cm.comment, formatter),
          'methods' => methods_data
        }
      end

      data
    end
  end

  # Manages allowlisted differences from tool/doc_lock_allowlist.yml
  class Allowlist
    attr_reader :classes, :methods, :parameter_mismatches, :call_seq_mismatches, :comment_mismatches, :exclusions

    def initialize(allowlist_file = nil)
      @classes = []
      @methods = []
      @parameter_mismatches = []
      @call_seq_mismatches = []
      @comment_mismatches = []
      @exclusions = []

      load_file(allowlist_file) if allowlist_file && File.file?(allowlist_file)
    end

    def load_file(file_path)
      yaml_data = YAML.load_file(file_path) || {}
      @classes = Array(yaml_data['classes'] || yaml_data['class'])
      @methods = Array(yaml_data['methods'] || yaml_data['method'])
      @parameter_mismatches = Array(yaml_data['parameter_mismatches'] || yaml_data['signatures'] || yaml_data['params'])
      @call_seq_mismatches = Array(yaml_data['call_seq_mismatches'] || yaml_data['call_seqs'])
      @comment_mismatches = Array(yaml_data['comment_mismatches'] || yaml_data['comments'])
      @exclusions = Array(yaml_data['exclusions'])
    end

    def allowed?(diff_type, entity_name, _details = {})
      case diff_type.to_s
      when 'missing_class', 'extra_class'
        @classes.include?(entity_name)
      when 'missing_method', 'extra_method'
        @methods.include?(entity_name)
      when 'parameter_mismatch'
        @parameter_mismatches.include?(entity_name) || @methods.include?(entity_name)
      when 'call_seq_mismatch'
        @call_seq_mismatches.include?(entity_name) || @methods.include?(entity_name)
      when 'comment_mismatch'
        @comment_mismatches.include?(entity_name) || @methods.include?(entity_name)
      else
        false
      end || @exclusions.any? do |rule|
        rule_type = rule['type'].to_s
        rule_name = rule['name'].to_s
        (rule_type.empty? || rule_type == diff_type.to_s) &&
          (rule_name.empty? || rule_name == entity_name || File.fnmatch?(rule_name, entity_name))
      end
    end
  end

  # Differential comparison engine comparing reference vs target stores
  class Comparator
    def initialize(reference_data, target_data, allowlist = nil)
      @ref = reference_data
      @target = target_data
      @allowlist = allowlist || Allowlist.new
    end

    def compare
      diffs = []

      all_classes = (@ref.keys + @target.keys).uniq.sort

      all_classes.each do |class_name|
        ref_class = @ref[class_name]
        target_class = @target[class_name]

        if ref_class.nil?
          diffs << create_diff('extra_class', class_name, 'Class/Module present in target but missing in reference', nil, target_class['type'])
          next
        elsif target_class.nil?
          diffs << create_diff('missing_class', class_name, 'Class/Module missing in target', ref_class['type'], nil)
          next
        end

        if ref_class['type'] != target_class['type']
          diffs << create_diff('class_type_mismatch', class_name, 'Type mismatch', ref_class['type'], target_class['type'])
        end

        if ref_class['comment'] != target_class['comment']
          diffs << create_diff('comment_mismatch', class_name, 'Class/Module docstring mismatch', ref_class['comment'], target_class['comment'])
        end

        ref_methods = ref_class['methods'] || {}
        target_methods = target_class['methods'] || {}
        all_methods = (ref_methods.keys + target_methods.keys).uniq.sort

        all_methods.each do |method_name|
          ref_m = ref_methods[method_name]
          target_m = target_methods[method_name]

          if ref_m.nil?
            diffs << create_diff('extra_method', method_name, 'Method present in target but missing in reference', nil, target_m['full_name'])
            next
          elsif target_m.nil?
            diffs << create_diff('missing_method', method_name, 'Method missing in target', ref_m['full_name'], nil)
            next
          end

          if ref_m['visibility'] != target_m['visibility']
            diffs << create_diff('visibility_mismatch', method_name, 'Visibility mismatch', ref_m['visibility'], target_m['visibility'])
          end

          if ref_m['singleton'] != target_m['singleton']
            diffs << create_diff('singleton_mismatch', method_name, 'Singleton flag mismatch', ref_m['singleton'], target_m['singleton'])
          end

          if ref_m['params'] != target_m['params']
            diffs << create_diff('parameter_mismatch', method_name, 'Parameter list mismatch', ref_m['params'], target_m['params'])
          end

          if ref_m['call_seq'] != target_m['call_seq']
            diffs << create_diff('call_seq_mismatch', method_name, 'call-seq declaration mismatch', ref_m['call_seq'], target_m['call_seq'])
          end

          if ref_m['comment'] != target_m['comment']
            diffs << create_diff('comment_mismatch', method_name, 'Docstring comment body mismatch', ref_m['comment'], target_m['comment'])
          end
        end
      end

      filter_diffs(diffs)
    end

    private

    def create_diff(type, entity, description, expected, actual)
      {
        'type' => type,
        'entity' => entity,
        'description' => description,
        'expected' => expected,
        'actual' => actual
      }
    end

    def filter_diffs(diffs)
      unapproved = []
      approved = []

      diffs.each do |diff|
        if @allowlist.allowed?(diff['type'], diff['entity'], diff)
          approved << diff
        else
          unapproved << diff
        end
      end

      { 'unapproved' => unapproved, 'approved' => approved }
    end
  end

  # Main CLI application
  class CLI
    def self.run(args = ARGV)
      options = {
        mode: :check,
        store_dir: File.expand_path('../.ext/rdoc', __dir__),
        reference: File.expand_path('doc_lock_reference.json', __dir__),
        allowlist: File.expand_path('doc_lock_allowlist.yml', __dir__),
        output_file: nil,
        verbose: false
      }

      parser = OptionParser.new do |opts|
        opts.banner = "Usage: tool/doc_lock.rb [options]"

        opts.on('-c', '--check [STORE_DIR]', 'Check RDoc store against reference (default action)') do |dir|
          options[:mode] = :check
          options[:store_dir] = File.expand_path(dir) if dir && !dir.empty?
        end

        opts.on('-d', '--dump [OUTPUT_FILE]', 'Dump RDoc store metadata to JSON') do |file|
          options[:mode] = :dump
          options[:output_file] = file if file && !file.empty?
        end

        opts.on('-s', '--store DIR', 'Path to RDoc store directory') do |dir|
          options[:store_dir] = File.expand_path(dir)
        end

        opts.on('-r', '--reference PATH', 'Path to reference JSON file or reference store directory') do |path|
          options[:reference] = File.expand_path(path)
        end

        opts.on('-a', '--allowlist FILE', 'Path to allowlist YAML file') do |file|
          options[:allowlist] = File.expand_path(file)
        end

        opts.on('-v', '--verbose', 'Enable verbose output') do
          options[:verbose] = true
        end

        opts.on('-h', '--help', 'Show this help message') do
          puts opts
          exit 0
        end
      end

      parser.parse!(args)

      case options[:mode]
      when :dump
        run_dump(options)
      when :check
        run_check(options)
      end
    end

    def self.run_dump(options)
      ensure_store_exists(options[:store_dir])
      puts "DocLock: Extracting metadata from #{options[:store_dir]}..." if options[:verbose]

      parser = StoreParser.new(options[:store_dir])
      metadata = parser.parse

      json_data = JSON.pretty_generate(metadata)

      if options[:output_file]
        output_path = File.expand_path(options[:output_file])
        FileUtils.mkdir_p(File.dirname(output_path))
        File.write(output_path, json_data)
        puts "DocLock: Dumped RDoc metadata to #{output_path} (#{metadata.size} classes/modules, #{json_data.bytesize} bytes)."
      else
        puts json_data
      end
    end

    def self.run_check(options)
      ensure_store_exists(options[:store_dir])

      target_parser = StoreParser.new(options[:store_dir])
      target_metadata = target_parser.parse

      reference_metadata = resolve_reference_metadata(options, target_metadata)

      allowlist = Allowlist.new(options[:allowlist])
      comparator = Comparator.new(reference_metadata, target_metadata, allowlist)
      results = comparator.compare

      unapproved = results['unapproved']
      approved = results['approved']

      if options[:verbose] && approved.any?
        puts "DocLock: Suppressed #{approved.size} approved diff(s) via allowlist:"
        approved.each { |d| puts "  [ALLOWED #{d['type']}] #{d['entity']}" }
      end

      if unapproved.empty?
        puts "DocLock verification passed: 0 unapproved documentation diffs found."
        exit 0
      else
        puts "DocLock verification FAILED: #{unapproved.size} unapproved documentation divergence(s) detected!"
        puts "-" * 80
        unapproved.each do |diff|
          puts "FAILED: [#{diff['type']}] #{diff['entity']}"
          puts "  Description: #{diff['description']}"
          puts "  Expected:    #{diff['expected'].inspect}"
          puts "  Actual:      #{diff['actual'].inspect}"
          puts
        end
        puts "-" * 80
        puts "If this difference is intended, add an exception to #{options[:allowlist]}."
        exit 1
      end
    end

    def self.ensure_store_exists(store_dir)
      return if File.directory?(store_dir) && File.file?(File.join(store_dir, 'cache.ri'))

      puts "DocLock: Target RDoc store not found at #{store_dir}. Generating RDoc store..."
      root_dir = File.expand_path('..', __dir__)
      rdoc_cmd = "LC_ALL=C.UTF-8 ruby -EUTF-8 \"#{File.join(root_dir, 'tool/rdoc-srcdir')}\" --ri --op \"#{store_dir}\" \"#{root_dir}\""
      system(rdoc_cmd) or raise "Failed to generate RDoc store via #{rdoc_cmd}"
    end

    def self.resolve_reference_metadata(options, current_target_metadata)
      ref_path = options[:reference]

      if File.file?(ref_path)
        puts "DocLock: Loading reference metadata from #{ref_path}..." if options[:verbose]
        JSON.parse(File.read(ref_path))
      elsif File.directory?(ref_path)
        puts "DocLock: Loading reference store from #{ref_path}..." if options[:verbose]
        StoreParser.new(ref_path).parse
      else
        puts "DocLock: Reference file not found at #{ref_path}. Dumping current store as reference..."
        File.write(ref_path, JSON.pretty_generate(current_target_metadata))
        current_target_metadata
      end
    end
  end
end

if __FILE__ == $0
  DocLock::CLI.run
end
