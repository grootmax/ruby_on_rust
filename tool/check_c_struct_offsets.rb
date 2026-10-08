#!/usr/bin/env ruby
# frozen_string_literal: true

require "json"
require "optparse"
require "rbconfig"
require "tmpdir"
require "fileutils"

class CStructOffsetChecker
  PUBLIC_STRUCT_FIELDS = {
    "RBasic" => %w[flags klass],
    "RString" => %w[basic len as as.heap.ptr as.heap.aux.capa as.heap.aux.shared as.embed.ary],
    "RArray" => %w[basic as as.heap.len as.heap.aux.capa as.heap.aux.shared_root as.heap.ptr as.ary],
    "RHash" => %w[basic ifnone],
    "RData" => %w[basic dmark dfree data],
    "RTypedData" => %w[basic fields_obj type data],
    "RObject" => %w[basic as as.ary as.extended],
    "RFile" => %w[basic fptr],
    "RRegexp" => %w[basic src usecnt],
    "RMatch" => %w[basic str regexp char_offset_num_allocated char_offset num_regs capa],
    "rmatch_offset" => %w[beg end],
    "rb_io" => %w[self stdio_file fd mode pid lineno pathv finalize wbuf rbuf tied_io_for_writing encs],
    "rb_io_encoding" => %w[enc enc2 ecflags],
    "rb_io_internal_buffer" => %w[ptr off len capa],
    "rb_data_type_struct" => %w[wrap_struct_name function function.dmark function.dfree function.dsize function.dcompact parent data flags],
    "rb_random_struct" => %w[seed],
    "rb_ractor_local_storage_type" => %w[mark free],
    "rb_memory_view_entry" => %w[get_func release_func available_p_func],
    "rb_memory_view" => %w[obj data byte_size readonly format item_size item_desc.components item_desc.length ndim shape strides sub_offsets private_data _memory_view_entry],
    "st_hash_type" => %w[compare hash],
    "st_table" => %w[entry_power bin_power size_ind rebuilds_num entries_start type num_entries entries_bound],
    "rb_internal_thread_event_data" => %w[thread],
    "rb_fiber_scheduler_blocking_operation_state" => %w[result saved_errno],
    "re_registers" => %w[allocated num_regs beg end],
    "re_pattern_buffer" => %w[p used alloc options syntax chain]
  }.freeze

  HEADER_INCLUDES = <<~C
    #include "ruby.h"
    #include "ruby/io.h"
    #include "ruby/re.h"
    #include "ruby/encoding.h"
    #include "ruby/memory_view.h"
    #include "ruby/thread.h"
    #include "ruby/thread_native.h"
    #include "ruby/ractor.h"
    #include "ruby/random.h"
    #include "ruby/st.h"
    #include "ruby/fiber/scheduler.h"
    #include "internal/hash.h"
    #include <stdio.h>
    #include <stddef.h>
  C

  EXCLUDED_STRUCTS = %w[
    stati128 iovec msghdr ifaddrs tms timeval timespec timezone
    rb_thread_cond_struct rb_nativethread_id_t rb_nativethread_lock_t rb_nativethread_cond_t
    foo_tag rbimpl_alignof rbimpl_size_overflow_tag
  ].freeze

  attr_reader :root_dir, :reference_path

  def initialize(root_dir: nil)
    @root_dir = root_dir || File.expand_path("..", __dir__)
    @reference_path = File.join(@root_dir, "tool", "c_struct_offsets_reference.json")
  end

  def scan_public_headers
    header_pattern = File.join(@root_dir, "include", "ruby", "**", "*.h")
    discovered_structs = {}

    Dir.glob(header_pattern).each do |header_path|
      next if header_path.include?("win32.h") || header_path.include?("missing.h") || header_path.include?("backward/")

      content = File.read(header_path)
      content.scan(/(?:typedef\s+)?struct\s+(?:RUBY_ALIGNAS\([^)]+\)\s+)?([A-Za-z0-9_]+)\s*\{([^}]+)\}/m) do |name, body|
        next if name.start_with?("rbimpl_") || name.start_with?("_") || EXCLUDED_STRUCTS.include?(name)

        field_matches = body.scan(/[\w\s\*]+\s+(\*?[\w_]+)(?:\[[^\]]*\])?\s*;/).flatten.map(&:strip)
        field_matches.reject! { |f| f.empty? || f.start_with?("/*") }
        next if field_matches.empty?
        discovered_structs[name] = field_matches
      end
    end

    discovered_structs
  end

  def include_flags
    inc_ruby = File.join(@root_dir, "include")
    flags = ["-I#{inc_ruby}", "-I#{@root_dir}"]

    ext_dirs = Dir.glob(File.join(@root_dir, ".ext", "include", "*")).select { |d| File.directory?(d) }
    ext_dirs.each do |ed|
      flags << "-I#{ed}"
    end
    flags << "-I#{File.join(@root_dir, ".ext", "include")}" if Dir.exist?(File.join(@root_dir, ".ext", "include"))

    flags
  end

  def compiler_command
    cc = ENV["CC"] || RbConfig::CONFIG["CC"] || "gcc"
    cflags = ENV["CFLAGS"] || RbConfig::CONFIG["CFLAGS"] || ""
    "#{cc} #{cflags} #{include_flags.join(" ")}"
  end

  def generate_probe_source(structs_map)
    lines = [HEADER_INCLUDES, "int main(void) {", '    printf("{\\n  \\"structs\\": {\\n");']

    struct_names = structs_map.keys.sort
    struct_names.each_with_index do |sname, sidx|
      fields = structs_map[sname]
      is_last_struct = (sidx == struct_names.size - 1)

      type_expr = sname.start_with?("rb_memory_view") && !sname.end_with?("_entry") ? "rb_memory_view_t" : "struct #{sname}"

      lines << "    /* #{sname} */"
      lines << "    printf(\"    \\\"#{sname}\\\": {\\n\");"
      lines << "    printf(\"      \\\"size\\\": %zu,\\n\", sizeof(#{type_expr}));"
      lines << "    printf(\"      \\\"fields\\\": {\\n\");"

      fields.each_with_index do |field, fidx|
        is_last_field = (fidx == fields.size - 1)
        comma = is_last_field ? "" : ","
        lines << "    printf(\"        \\\"#{field}\\\": %zu#{comma}\\n\", offsetof(#{type_expr}, #{field}));"
      end

      lines << "    printf(\"      }\\n\");"
      lines << "    printf(\"    }#{is_last_struct ? "" : ","}\\n\");"
    end

    lines << "    printf(\"  }\\n}\\n\");"
    lines << "    return 0;"
    lines << "}"
    lines.join("\n")
  end

  def collect_layout_metadata
    scanned = scan_public_headers
    structs_map = PUBLIC_STRUCT_FIELDS.dup

    scanned.each do |sname, sfields|
      next if structs_map.key?(sname)
      next if sname.start_with?("rbimpl_") || sname.start_with?("Onig") || sname.start_with?("driver") || EXCLUDED_STRUCTS.include?(sname)
      structs_map[sname] = sfields
    end

    Dir.mktmpdir("c_struct_check") do |tmpdir|
      src_file = File.join(tmpdir, "probe.c")
      exe_file = File.join(tmpdir, "probe")

      File.write(src_file, generate_probe_source(structs_map))

      cmd = "#{compiler_command} #{src_file} -o #{exe_file}"
      system(cmd, out: File::NULL, err: File::NULL) or raise "Failed to compile C struct layout probe: #{cmd}"

      output = `#{exe_file}`
      raise "C struct layout probe failed to run" unless $?.success?

      JSON.parse(output)
    end
  end

  def generate_static_assertion_source(data)
    lines = [
      HEADER_INCLUDES,
      "#if defined(RBIMPL_STATIC_ASSERT)",
      "#  define ASSERT_LAYOUT(name, cond) RBIMPL_STATIC_ASSERT(name, cond)",
      "#elif defined(__STDC_VERSION__) && __STDC_VERSION__ >= 201112L",
      "#  define ASSERT_LAYOUT(name, cond) _Static_assert(cond, #name)",
      "#else",
      "#  define ASSERT_LAYOUT_CONCAT_IMPL(a, b) a##b",
      "#  define ASSERT_LAYOUT_CONCAT(a, b) ASSERT_LAYOUT_CONCAT_IMPL(a, b)",
      "#  define ASSERT_LAYOUT(name, cond) typedef char ASSERT_LAYOUT_CONCAT(static_assert_failed_, __LINE__)[(cond) ? 1 : -1]",
      "#endif",
      ""
    ]

    structs = data["structs"] || {}
    structs.keys.sort.each do |sname|
      sinfo = structs[sname]
      size = sinfo["size"]
      type_expr = sname.start_with?("rb_memory_view") && !sname.end_with?("_entry") ? "rb_memory_view_t" : "struct #{sname}"
      lines << "ASSERT_LAYOUT(#{sname}_size, sizeof(#{type_expr}) == #{size});"

      fields = sinfo["fields"] || {}
      fields.keys.sort.each do |fname|
        offset = fields[fname]
        safe_fname = fname.gsub(".", "_")
        lines << "ASSERT_LAYOUT(#{sname}_offset_#{safe_fname}, offsetof(#{type_expr}, #{fname}) == #{offset});"
      end
    end

    lines << ""
    lines.join("\n")
  end

  def compile_static_assertions(data)
    Dir.mktmpdir("c_struct_assert") do |tmpdir|
      src_file = File.join(tmpdir, "assertions.c")
      obj_file = File.join(tmpdir, "assertions.o")

      File.write(src_file, generate_static_assertion_source(data))

      cmd = "#{compiler_command} -c #{src_file} -o #{obj_file}"
      out = `#{cmd} 2>&1`
      [ $?.success?, out ]
    end
  end

  def update_reference!
    data = collect_layout_metadata
    formatted = JSON.pretty_generate(data) + "\n"
    File.write(@reference_path, formatted)
    puts "Updated baseline C struct reference: #{@reference_path}"
    data
  end

  def check!
    unless File.exist?(@reference_path)
      $stderr.puts "Error: Reference file #{@reference_path} does not exist."
      $stderr.puts "Run 'ruby tool/check_c_struct_offsets.rb --update' to create it."
      return false
    end

    expected_data = JSON.parse(File.read(@reference_path))
    current_data = collect_layout_metadata

    diffs = []
    expected_structs = expected_data["structs"] || {}
    current_structs = current_data["structs"] || {}

    expected_structs.each do |sname, sinfo|
      curr_sinfo = current_structs[sname]
      if curr_sinfo.nil?
        diffs << "Struct '#{sname}' missing in current headers"
        next
      end

      if sinfo["size"] != curr_sinfo["size"]
        diffs << "Struct '#{sname}' size changed: expected #{sinfo["size"]}, got #{curr_sinfo["size"]}"
      end

      exp_fields = sinfo["fields"] || {}
      curr_fields = curr_sinfo["fields"] || {}

      exp_fields.each do |fname, exp_offset|
        curr_offset = curr_fields[fname]
        if curr_offset.nil?
          diffs << "Field '#{sname}.#{fname}' missing in current headers"
        elsif exp_offset != curr_offset
          diffs << "Field '#{sname}.#{fname}' offset changed: expected #{exp_offset}, got #{curr_offset}"
        end
      end
    end

    assert_success, assert_output = compile_static_assertions(expected_data)
    unless assert_success
      diffs << "C compile-time static assertions failed:\n#{assert_output}"
    end

    if diffs.empty?
      puts "[OK] C struct layout lock verified. All public struct sizes and field offsets match golden baseline."
      true
    else
      $stderr.puts "[ERROR] C struct layout mismatch detected!"
      $stderr.puts ""
      diffs.each { |d| $stderr.puts "  - #{d}" }
      $stderr.puts ""
      $stderr.puts "Run 'ruby tool/check_c_struct_offsets.rb --update' if this ABI change is intentional."
      false
    end
  end
end

if __FILE__ == $0
  options = { mode: :check }
  OptionParser.new do |opts|
    opts.banner = "Usage: ruby tool/check_c_struct_offsets.rb [options]"

    opts.on("--update", "Regenerate golden reference layout file") do
      options[:mode] = :update
    end

    opts.on("--check", "Verify layout against golden reference (default)") do
      options[:mode] = :check
    end

    opts.on("-h", "--help", "Show help") do
      puts opts
      exit 0
    end
  end.parse!

  checker = CStructOffsetChecker.new
  case options[:mode]
  when :update
    checker.update_reference!
    exit 0
  when :check
    success = checker.check!
    exit(success ? 0 : 1)
  end
end
