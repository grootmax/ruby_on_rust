#!/usr/bin/env ruby
# frozen_string_literal: true

module CoreApiDumper
  CATEGORY_MAP = {
    method: "methods",
    module: "modules",
    constant: "constants",
    ancestors: "ancestors"
  }.freeze

  EXCLUDED_PREFIXES = %w[
    CoreApiDumper
    JSON
    Psych
    OptionParser
    Open3
    RbConfig
    YAML
    PP
    PrettyPrint
    Gem
    Bundler
    DidYouMean
    ErrorHighlight
    SyntaxSuggest
    RDoc
  ].freeze

  class Dumper
    def self.dump_core_api
      modules = {}

      ObjectSpace.each_object(Module).each do |mod|
        next if mod.singleton_class?
        name = mod.name
        next if name.nil? || name.empty? || name.start_with?("#<") || name.include?(":#<")
        next if EXCLUDED_PREFIXES.any? { |prefix| name == prefix || name.start_with?("#{prefix}::") }

        anc = mod.ancestors.map(&:name).compact.reject do |n|
          n.empty? || n.start_with?("#<") || EXCLUDED_PREFIXES.any? { |prefix| n == prefix || n.start_with?("#{prefix}::") }
        end.sort

        consts = mod.constants(false).reject do |c|
          c_str = c.to_s
          EXCLUDED_PREFIXES.any? { |prefix| c_str == prefix || c_str.start_with?("#{prefix}::") }
        end.map do |c|
          begin
            old_v = $VERBOSE
            $VERBOSE = nil
            val = mod.const_get(c, false)
            $VERBOSE = old_v
            val_norm = normalize_const(mod, c, val)
            { "name" => c.to_s, "value" => val_norm.to_s }
          rescue Exception
            nil
          end
        end.compact.sort_by { |c| c["name"] }

        inst_m = extract_methods(mod, singleton: false)
        sing_m = extract_methods(mod, singleton: true)

        modules[name] = {
          "ancestors" => anc,
          "constants" => consts,
          "instance_methods" => inst_m,
          "singleton_methods" => sing_m,
          "type" => mod.is_a?(Class) ? "class" : "module"
        }
      end

      sorted_modules = modules.keys.sort.each_with_object({}) do |key, hash|
        hash[key] = modules[key]
      end

      require "json" unless defined?(JSON)
      JSON.pretty_generate(sorted_modules)
    end

    def self.extract_methods(mod, singleton: false)
      target = singleton ? mod.singleton_class : mod
      methods = []

      visibilities = {
        "public" => target.public_instance_methods(false),
        "protected" => target.protected_instance_methods(false),
        "private" => target.private_instance_methods(false)
      }

      visibilities.each do |vis, method_names|
        method_names.each do |m_name|
          begin
            unbound_m = target.instance_method(m_name)
            arity = unbound_m.arity
            params = unbound_m.parameters.map do |kind, p_name|
              p_name ? [kind.to_s, p_name.to_s] : [kind.to_s]
            end
            methods << {
              "arity" => arity,
              "name" => m_name.to_s,
              "parameters" => params,
              "visibility" => vis
            }
          rescue NameError
          end
        end
      end

      methods.sort_by { |m| [m["name"], m["visibility"]] }
    end

    def self.normalize_const(mod, name, val)
      if mod == Object && name == :ENV
        "<ENV>"
      elsif mod == Object && name == :ARGF
        "<ARGF>"
      elsif mod == Object && name == :TOPLEVEL_BINDING
        "<TOPLEVEL_BINDING>"
      elsif mod == Object && name.to_s.start_with?("RUBY_")
        "<#{name}>"
      elsif mod == RbConfig && [:TOPDIR, :DESTDIR].include?(name)
        "<#{name}>"
      elsif (mod == File || mod == File::Constants) && [:ALT_SEPARATOR, :PATH_SEPARATOR, :NULL].include?(name)
        "<#{name}>"
      elsif val.is_a?(Float)
        if val.nan?
          "Float::NAN"
        elsif val.infinite? == 1
          "Float::INFINITY"
        elsif val.infinite? == -1
          "-Float::INFINITY"
        else
          val
        end
      elsif val.is_a?(Module)
        (val.name && !val.name.empty?) ? val.name : "<Anonymous Module>"
      elsif val.is_a?(Encoding)
        val.name
      elsif val.is_a?(Integer) || val.is_a?(String) || val.is_a?(Symbol) || val == true || val == false || val.nil?
        val.is_a?(Symbol) ? val.to_s : val
      else
        "<#{val.class.name}>"
      end
    end
  end

  class Differ
    def self.diff(baseline_data, current_data)
      diffs = []
      all_modules = (baseline_data.keys + current_data.keys).uniq.sort

      all_modules.each do |mod_name|
        base_mod = baseline_data[mod_name]
        curr_mod = current_data[mod_name]

        if base_mod.nil?
          diffs << {
            category: :module,
            action: :added,
            target: mod_name,
            description: "Added module #{mod_name}"
          }
          next
        end

        if curr_mod.nil?
          diffs << {
            category: :module,
            action: :removed,
            target: mod_name,
            description: "Removed module #{mod_name}"
          }
          next
        end

        # Compare ancestors
        if base_mod["ancestors"] != curr_mod["ancestors"]
          diffs << {
            category: :ancestors,
            action: :changed,
            target: mod_name,
            description: "Ancestors changed for #{mod_name}: baseline #{base_mod['ancestors'].inspect} vs current #{curr_mod['ancestors'].inspect}"
          }
        end

        # Compare constants
        base_consts = (base_mod["constants"] || []).each_with_object({}) { |c, h| h[c["name"]] = c["value"] }
        curr_consts = (curr_mod["constants"] || []).each_with_object({}) { |c, h| h[c["name"]] = c["value"] }

        (base_consts.keys + curr_consts.keys).uniq.sort.each do |c_name|
          target = "#{mod_name}::#{c_name}"
          if !base_consts.key?(c_name)
            diffs << {
              category: :constant,
              action: :added,
              target: target,
              description: "Added constant #{target} = #{curr_consts[c_name].inspect}"
            }
          elsif !curr_consts.key?(c_name)
            diffs << {
              category: :constant,
              action: :removed,
              target: target,
              description: "Removed constant #{target}"
            }
          elsif base_consts[c_name] != curr_consts[c_name]
            diffs << {
              category: :constant,
              action: :changed,
              target: target,
              description: "Changed constant #{target}: baseline #{base_consts[c_name].inspect} -> current #{curr_consts[c_name].inspect}"
            }
          end
        end

        # Compare instance methods
        diff_methods(diffs, mod_name, "#", base_mod["instance_methods"] || [], curr_mod["instance_methods"] || [])

        # Compare singleton methods
        diff_methods(diffs, mod_name, ".", base_mod["singleton_methods"] || [], curr_mod["singleton_methods"] || [])
      end

      diffs
    end

    def self.diff_methods(diffs, mod_name, separator, base_methods, curr_methods)
      base_map = base_methods.each_with_object({}) { |m, h| h[m["name"]] = m }
      curr_map = curr_methods.each_with_object({}) { |m, h| h[m["name"]] = m }

      (base_map.keys + curr_map.keys).uniq.sort.each do |m_name|
        target = "#{mod_name}#{separator}#{m_name}"
        m_base = base_map[m_name]
        m_curr = curr_map[m_name]

        if m_base.nil?
          diffs << {
            category: :method,
            action: :added,
            target: target,
            description: "Added method #{target} (visibility: #{m_curr['visibility']}, arity: #{m_curr['arity']}, params: #{m_curr['parameters'].inspect})"
          }
        elsif m_curr.nil?
          diffs << {
            category: :method,
            action: :removed,
            target: target,
            description: "Removed method #{target}"
          }
        else
          changes = []
          if m_base["visibility"] != m_curr["visibility"]
            changes << "visibility: #{m_base['visibility']} -> #{m_curr['visibility']}"
          end
          if m_base["arity"] != m_curr["arity"]
            changes << "arity: #{m_base['arity']} -> #{m_curr['arity']}"
          end
          if m_base["parameters"] != m_curr["parameters"]
            changes << "parameters: #{m_base['parameters'].inspect} -> #{m_curr['parameters'].inspect}"
          end

          unless changes.empty?
            diffs << {
              category: :method,
              action: :changed,
              target: target,
              description: "Changed method #{target}: #{changes.join(', ')}"
            }
          end
        end
      end
    end

    def self.apply_allowlist(diffs, allowlist_path)
      return diffs if allowlist_path.nil? || !File.exist?(allowlist_path)

      allowlist_data = begin
        YAML.safe_load_file(allowlist_path) || {}
      rescue Exception
        {}
      end

      diffs.each do |diff|
        cat_key = CATEGORY_MAP[diff[:category]] || diff[:category].to_s
        action_s = diff[:action].to_s
        allowed_entries = allowlist_data.dig(cat_key, action_s) || []

        is_allowed = allowed_entries.any? do |entry|
          if entry.is_a?(String)
            entry == diff[:target] || File.fnmatch?(entry, diff[:target])
          elsif entry.is_a?(Hash)
            entry["target"] == diff[:target] || entry["method"] == diff[:target] || entry["name"] == diff[:target]
          else
            false
          end
        end

        diff[:allowed] = is_allowed
      end

      diffs
    end
  end

  class Runner
    def self.run(argv = ARGV)
      if argv.include?("--internal-dump")
        puts Dumper.dump_core_api
        exit 0
      end

      require "optparse"
      require "open3"
      require "rbconfig"
      require "json"
      require "yaml"

      options = {
        check: false,
        lockfile: "spec/core_api_lock.json",
        allowlist: "tool/api_allowlist.yml",
        output: nil,
        internal_dump: false
      }

      opt_parser = OptionParser.new do |opts|
        opts.banner = "Usage: ruby tool/dump_api.rb [options]"

        opts.on("--check", "Check runtime reflection against lockfile") do
          options[:check] = true
        end

        opts.on("--lockfile PATH", "Path to API lock JSON file (default: spec/core_api_lock.json)") do |p|
          options[:lockfile] = p
        end

        opts.on("--allowlist PATH", "Path to API allowlist YAML file (default: tool/api_allowlist.yml)") do |p|
          options[:allowlist] = p
        end

        opts.on("-o", "--output PATH", "Output path to write canonical JSON dump") do |p|
          options[:output] = p
        end

        opts.on("--internal-dump", "Internal Stage 1 reflection dump mode") do
          options[:internal_dump] = true
        end

        opts.on("-h", "--help", "Show help") do
          puts opts
          exit 0
        end
      end

      opt_parser.parse!(argv)

      if options[:internal_dump]
        puts Dumper.dump_core_api
        exit 0
      end

      # Spawn isolated Stage 1 reflection subprocess
      ruby_bin = RbConfig.ruby
      script_path = File.expand_path(__FILE__)
      stdout, stderr, status = Open3.capture3(
        ruby_bin,
        "--disable-gems",
        "--disable-did-you-mean",
        "--disable-error_highlight",
        script_path,
        "--internal-dump"
      )

      unless status.success?
        $stderr.puts "Error running Stage 1 reflection: #{stderr}"
        exit status.exitstatus || 1
      end

      current_json = stdout
      current_data = JSON.parse(current_json)

      if options[:check]
        unless File.exist?(options[:lockfile])
          $stderr.puts "Error: Lockfile '#{options[:lockfile]}' not found. Generate it using 'ruby tool/dump_api.rb --output #{options[:lockfile]}'."
          exit 1
        end

        baseline_data = JSON.parse(File.read(options[:lockfile]))
        diffs = Differ.diff(baseline_data, current_data)
        diffs = Differ.apply_allowlist(diffs, options[:allowlist])

        unauthorized_diffs = diffs.reject { |d| d[:allowed] }
        allowed_count = diffs.count { |d| d[:allowed] }

        if unauthorized_diffs.empty?
          if allowed_count > 0
            puts "API Lock Check: OK (0 unauthorized differences, #{allowed_count} allowed differences ignored)"
          else
            puts "API Lock Check: OK (0 differences found)"
          end
          exit 0
        else
          $stderr.puts "======================================================================"
          $stderr.puts "API Lock Divergence Detected!"
          $stderr.puts "The following core API changes differ from #{options[:lockfile]}:"
          $stderr.puts ""

          unauthorized_diffs.group_by { |d| [d[:category], d[:action]] }.each do |(cat, act), items|
            $stderr.puts "[#{act.to_s.upcase} #{CATEGORY_MAP[cat].to_s.upcase}]"
            items.each do |item|
              $stderr.puts "  - #{item[:description]}"
            end
            $stderr.puts ""
          end

          $stderr.puts "Total unauthorized differences: #{unauthorized_diffs.size} (#{allowed_count} allowed differences ignored)"
          $stderr.puts ""
          $stderr.puts "To approve these changes, add them to #{options[:allowlist]} or update"
          $stderr.puts "#{options[:lockfile]} by running:"
          $stderr.puts "  ruby tool/dump_api.rb --output #{options[:lockfile]}"
          $stderr.puts "======================================================================"
          exit 1
        end
      elsif options[:output]
        File.write(options[:output], current_json)
        puts "Core API lock JSON written to #{options[:output]}"
      else
        puts current_json
      end
    end
  end
end

if __FILE__ == $0
  CoreApiDumper::Runner.run(ARGV)
end
