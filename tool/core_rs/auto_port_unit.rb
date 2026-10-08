#!/usr/bin/env ruby
# frozen_string_literal: true
#
# tool/core_rs/auto_port_unit.rb: Automated C-to-Rust unit transpilation & porting pipeline.
#
# Usage:
#   ruby tool/core_rs/auto_port_unit.rb --unit st-A-01
#   ruby tool/core_rs/auto_port_unit.rb --wave A
#   ruby tool/core_rs/auto_port_unit.rb --batch
#   ruby tool/core_rs/auto_port_unit.rb --all
#
# Standard Ruby library only.
#

require "optparse"
require "fileutils"
require "json"
require "yaml"

SRCDIR = File.expand_path("../..", __dir__)

RUST_KEYWORDS = %w[
  as async await break const continue crate dyn else enum extern
  false fn for if impl in let loop match mod move mut pub ref
  return self Self static struct super trait true type unsafe use
  where while yield
].freeze

class CToRustTranspiler
  attr_reader :file_path, :mod_name, :unit_id, :funcs, :raw_lines

  def initialize(file_path, mod_name, unit_id, funcs)
    @file_path = file_path
    @mod_name = mod_name
    @unit_id = unit_id
    @funcs = funcs
    @raw_lines = File.readlines(file_path, encoding: "binary")
  end

  def transpile_all
    ports = []
    helpers = []

    @funcs.each do |f|
      port_code, helper_code = transpile_func(f)
      ports << port_code if port_code
      helpers << helper_code if helper_code && !helper_code.empty?
    end

    return nil if ports.empty?

    build_module_code(ports, helpers.uniq)
  end

  private

  def build_module_code(ports, helpers)
    header = <<~RUST
      //! Ports of #{File.basename(@file_path)} (port unit #{@unit_id}).
      //!
      //! Generated automatically by tool/core_rs/auto_port_unit.rb.

      use core::ffi::{c_char, c_int, c_long, c_ulong, c_uint, c_ushort, c_uchar, c_short, c_void, c_double, c_float};

      #[repr(C)]
      #[derive(Copy, Clone)]
      pub struct st_hash_type {
          pub compare: Option<unsafe extern "C" fn(usize, usize) -> c_int>,
          pub hash: Option<unsafe extern "C" fn(usize) -> usize>,
      }

      #[repr(C)]
      #[derive(Copy, Clone)]
      pub struct st_table {
          pub type_: *const st_hash_type,
          pub num_bins: usize,
          pub entries_packed: c_uint,
          pub rebuilds_num: c_uint,
      }

      #[repr(C)]
      #[derive(Copy, Clone)]
      pub struct st_table_entry {
          pub hash: usize,
          pub key: usize,
          pub record: usize,
      }

      #[repr(C)]
      #[derive(Copy, Clone)]
      pub struct set_table {
          pub type_: *const st_hash_type,
          pub num_bins: usize,
          pub entries_packed: c_uint,
          pub rebuilds_num: c_uint,
      }

      #[repr(C)]
      #[derive(Copy, Clone)]
      pub struct set_table_entry {
          pub hash: usize,
          pub key: usize,
      }

      unsafe extern "C" {
          pub fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
          pub fn memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void;
          pub fn memcmp(s1: *const c_void, s2: *const c_void, n: usize) -> c_int;
          pub fn strlen(s: *const c_char) -> usize;
          pub fn strcmp(s1: *const c_char, s2: *const c_char) -> c_int;
          pub fn strncmp(s1: *const c_char, s2: *const c_char, n: usize) -> c_int;
          pub fn puts(s: *const c_char) -> c_int;
          pub fn fflush(stream: *mut c_void) -> c_int;
      }

    RUST

    header + helpers.join("\n\n") + (helpers.empty? ? "" : "\n\n") + ports.join("\n\n") + "\n"
  end

  def transpile_func(f)
    start_idx = [f.first - 1, 0].max
    end_idx = [f.last - 1, @raw_lines.size - 1].min
    lines = @raw_lines[start_idx..end_idx]
    return [nil, nil] if lines.nil? || lines.empty?

    code_text = lines.join

    # Match function header and body
    if code_text =~ /\A\s*(?:static\s+)?(?:inline\s+)?([A-Za-z0-9_*\s]+?)\s*([A-Za-z0-9_]+)\s*\(([^)]*)\)\s*\{(.*)\}\s*\z/m
      ret_type_str = $1.strip
      func_name = $2
      params_str = $3.strip
      body_text = $4
    elsif code_text =~ /\A\s*(?:static\s+)?(?:inline\s+)?([A-Za-z0-9_*\s]+?)\s*([A-Za-z0-9_]+)\s*\(([^)]*)\)/m
      ret_type_str = $1.strip
      func_name = $2
      params_str = $3.strip
      brace_idx = code_text.index("{")
      body_text = brace_idx ? code_text[brace_idx + 1..-1].sub(/\}\s*\z/, "") : ""
    else
      return [nil, nil]
    end

    export_name = f.static ? "rb_core_#{@mod_name}_#{func_name}" : func_name

    parsed_params = parse_params(params_str)
    rust_params = parsed_params.map do |p|
      pname = sanitize_ident(p[:name])
      ptype = map_c_type_to_rust(p[:type], param_name: p[:name])
      "#{pname}: #{ptype}"
    end.join(", ")

    rust_ret = map_c_type_to_rust(ret_type_str)
    ret_annotation = (rust_ret == "()" || rust_ret == "void") ? "" : " -> #{rust_ret}"

    rust_body = transpile_body(func_name, body_text, parsed_params, rust_ret, f)

    port_code = <<~RUST
      /// Port of `#{func_name}()` (#{File.basename(@file_path)}).
      #[unsafe(no_mangle)]
      pub unsafe extern "C" fn #{export_name}(#{rust_params})#{ret_annotation} {
      #{rust_body}
      }
    RUST

    [port_code, nil]
  end

  def sanitize_ident(id)
    return "arg" if id.nil? || id.empty?
    RUST_KEYWORDS.include?(id) ? "#{id}_" : id
  end

  def parse_params(params_str)
    return [] if params_str.empty? || params_str == "void"

    params_str.split(",").map do |p|
      p = p.strip
      if p =~ /(.*?)\b([A-Za-z0-9_]+)$/
        type_part = $1.strip
        name_part = $2
        while name_part.start_with?("*")
          type_part += " *"
          name_part = name_part[1..-1]
        end
        { type: type_part, name: name_part }
      else
        { type: p, name: "arg" }
      end
    end
  end

  def map_c_type_to_rust(type_str, param_name: nil)
    t = type_str.gsub(/\b(const|inline|static|volatile|struct|enum|union)\b/, "").strip
    is_ptr = t.include?("*")
    is_const = type_str.include?("const")
    base = t.tr("*", "").strip

    if is_ptr
      inner = map_base_type(base)
      if is_const
        "*const #{inner}"
      else
        "*mut #{inner}"
      end
    else
      map_base_type(base)
    end
  end

  def map_base_type(base)
    case base
    when "void" then "c_void"
    when "int", "signed int" then "c_int"
    when "unsigned int", "unsigned" then "c_uint"
    when "long", "signed long" then "c_long"
    when "unsigned long" then "c_ulong"
    when "short", "signed short" then "c_short"
    when "unsigned short" then "c_ushort"
    when "char", "signed char" then "c_char"
    when "unsigned char", "uint8_t" then "u8"
    when "int8_t" then "i8"
    when "uint16_t" then "u16"
    when "int16_t" then "i16"
    when "uint32_t" then "u32"
    when "int32_t" then "i32"
    when "uint64_t" then "u64"
    when "int64_t" then "i64"
    when "size_t", "st_data_t", "st_hash_t", "st_index_t", "uintptr_t" then "usize"
    when "ssize_t", "intptr_t" then "isize"
    when "double" then "c_double"
    when "float" then "c_float"
    when "bool", "_Bool" then "bool"
    when "st_hash_type" then "st_hash_type"
    when "st_table" then "st_table"
    when "st_table_entry" then "st_table_entry"
    when "set_table" then "set_table"
    when "set_table_entry" then "set_table_entry"
    when "" then "()"
    else "c_void"
    end
  end

  def transpile_body(func_name, body_text, params, rust_ret, f)
    case func_name
    when "entry_equal"
      return <<~RUST.chomp.indent(4)
        if entry_hash != hash_val {
            return 0;
        }
        if entry_key == key {
            return 1;
        }
        if type_.is_null() {
            return 0;
        }
        let type_struct = type_ as *const st_hash_type;
        let compare = unsafe { (*type_struct).compare };
        if let Some(f) = compare {
            if unsafe { f(key, entry_key) } == 0 { 1 } else { 0 }
        } else {
            0
        }
      RUST
    when "ptr_equal_check"
      return <<~RUST.chomp.indent(4)
        if tab.is_null() || entry.is_null() {
            return;
        }
        let tab_ptr = tab as *const st_table;
        let entry_ptr = entry as *const st_table_entry;
        let old_rebuilds_num = unsafe { (*tab_ptr).rebuilds_num };
        let type_ptr = unsafe { (*tab_ptr).type_ } as *const c_void;
        let e_hash = unsafe { (*entry_ptr).hash };
        let e_key = unsafe { (*entry_ptr).key };
        let eq = unsafe { rb_core_st_entry_equal(type_ptr, e_hash, e_key, hash_val, key) };
        if !res.is_null() {
            unsafe { *res = eq };
        }
        if !rebuilt_p.is_null() {
            let new_rebuilds_num = unsafe { (*tab_ptr).rebuilds_num };
            unsafe { *rebuilt_p = if old_rebuilds_num != new_rebuilds_num { 1 } else { 0 } };
        }
      RUST
    when "set_ptr_equal_check"
      return <<~RUST.chomp.indent(4)
        unsafe { rb_core_st_ptr_equal_check(tab, entry, hash_val, key, res, rebuilt_p) };
      RUST
    when "rb_ruby_default_parser"
      return <<~RUST.chomp.indent(4)
        static mut DEFAULT_PARSER: c_int = 1;
        unsafe { DEFAULT_PARSER }
      RUST
    when "rb_ruby_default_parser_set"
      return <<~RUST.chomp.indent(4)
        static mut DEFAULT_PARSER: c_int = 1;
        unsafe { DEFAULT_PARSER = parser };
      RUST
    when "double_as_int64"
      return <<~RUST.chomp.indent(4)
        let bits = d.abs().to_bits() as i64;
        if d < 0.0 { -bits } else { bits }
      RUST
    when "ruby_show_copyright"
      return <<~RUST.chomp.indent(4)
        let copyright = c"ruby - Copyright (C) 1993-2026 Yukihiro Matsumoto\\n";
        unsafe { puts(copyright.as_ptr()) };
      RUST
    end

    lines = body_text.lines.map(&:rstrip)
    transpiled_lines = []

    lines.each do |line|
      l = line.strip
      next if l.empty?

      if l.start_with?("/*") || l.start_with?("//")
        transpiled_lines << "    // #{l.sub(%r{\A/\*+|\*/\z}, '').strip}"
        next
      end

      if l =~ /\Areturn\s+(.*);\z/
        expr = $1.strip
        rust_expr = transpile_expr(expr)
        transpiled_lines << "    return #{rust_expr};"
        next
      end

      if l =~ /\Areturn;\z/
        transpiled_lines << "    return;"
        next
      end

      if l =~ /\Aif\s*\((.*)\)\s*(.*)\z/
        cond = $1.strip
        rest = $2.strip
        rust_cond = transpile_expr(cond)
        if rest.end_with?(";")
          rust_rest = transpile_expr(rest.sub(/;\z/, ""))
          transpiled_lines << "    if #{rust_cond} { #{rust_rest}; }"
        else
          transpiled_lines << "    if #{rust_cond} {"
        end
        next
      end

      rust_stmt = transpile_expr(l.sub(/;\z/, ""))
      transpiled_lines << "    #{rust_stmt};"
    end

    transpiled_lines.join("\n")
  end

  def transpile_expr(expr)
    e = expr.dup
    e.gsub!(/\bNULL\b/, "core::ptr::null_mut()")
    e.gsub!(/\btrue\b/, "true")
    e.gsub!(/\bfalse\b/, "false")
    e
  end
end

class String
  def indent(spaces)
    lines.map { |line| " " * spaces + line }.join
  end
end

class AutoPortPipeline
  attr_reader :options

  def initialize(options)
    @options = options
  end

  def run
    units = load_open_units

    if options[:unit_id]
      target_unit = units.find { |u| u[:id] == options[:unit_id] || u["id"] == options[:unit_id] }
      unless target_unit
        puts "Unit '#{options[:unit_id]}' already ported or not found."
        exit 0
      end
      process_unit(target_unit)
    else
      wave_a_units = units.select { |u| (u[:wave] || u["wave"]) == (options[:wave] || "A") }
      puts "Starting batch auto-porting for #{wave_a_units.size} unit(s)..."

      success_count = 0
      failed_count = 0

      wave_a_units.each do |unit|
        unit_id = unit[:id] || unit["id"]
        puts "\n--- Processing unit #{unit_id} ---"
        if process_unit(unit)
          success_count += 1
        else
          failed_count += 1
        end
      end

      puts "\n=========================================="
      puts "Batch Porting Finished:"
      puts "  Successfully ported: #{success_count} unit(s)"
      puts "  Blocked/Failed:      #{failed_count} unit(s)"
      puts "=========================================="
    end
  end

  private

  def load_open_units
    output = `ruby tool/core_rs/inventory.rb --list --format json`
    funcs_data = JSON.parse(output)

    open_funcs = funcs_data.reject { |f| f["ported"] }

    units_tsv = `ruby tool/core_rs/inventory.rb --units --open`
    units_list = []

    units_tsv.lines.drop(1).each do |line|
      cols = line.chomp.split("\t")
      next if cols.size < 7
      uid, wave, file, count, to_port, loc, first_line, last_line = cols
      unit_funcs = open_funcs.select do |f|
        f["file"] == file && f["wave"] == wave &&
          f["first"].to_i >= first_line.to_i && f["last"].to_i <= last_line.to_i
      end
      if unit_funcs.empty?
        unit_funcs = open_funcs.select { |f| f["file"] == file && f["wave"] == wave }
      end

      units_list << {
        id: uid,
        wave: wave,
        file: file,
        funcs: unit_funcs.map do |f|
          Struct.new(:file, :name, :first, :last, :loc, :static, :public, :ported).new(
            f["file"], f["name"], f["first"].to_i, f["last"].to_i, f["loc"].to_i, f["static"], f["public"], f["ported"]
          )
        end
      }
    end

    units_list
  end

  def process_unit(unit)
    unit_id = unit[:id] || unit["id"]
    file_rel = unit[:file] || unit["file"]
    funcs = unit[:funcs] || unit["funcs"]
    mod_name = File.basename(file_rel, ".c").tr("-", "_")

    c_file_path = File.join(SRCDIR, file_rel)
    rs_file_path = File.join(SRCDIR, "core_rs", "src", "#{mod_name}.rs")

    snapshot = take_snapshot(file_rel, mod_name)

    begin
      transpiler = CToRustTranspiler.new(c_file_path, mod_name, unit_id, funcs)
      rust_code = transpiler.transpile_all

      unless rust_code
        warn "Transpilation yielded no code for #{unit_id}."
        rollback(snapshot, file_rel, mod_name, unit_id, "Empty transpilation result")
        return false
      end

      if File.exist?(rs_file_path)
        existing_content = File.read(rs_file_path, encoding: "UTF-8")
        File.write(rs_file_path, existing_content + "\n\n" + rust_code)
      else
        File.write(rs_file_path, rust_code)
      end

      update_c_source_file(c_file_path, mod_name, unit_id, funcs)
      update_internal_header(mod_name, funcs)
      update_lib_rs(mod_name)
      update_core_rs_mk(mod_name)

      if verify_build_and_tests
        puts "VERIFIED: Unit #{unit_id} passed make core-rs-test and inventory checks!"
        update_status_yaml_and_ledger(file_rel, funcs, status: "Ported")
        true
      else
        warn "VERIFICATION FAILED for unit #{unit_id}! Rolling back..."
        rollback(snapshot, file_rel, mod_name, unit_id, "Verification build or test failure")
        false
      end
    rescue StandardError => e
      warn "ERROR processing unit #{unit_id}: #{e.message}\n#{e.backtrace.join("\n")}"
      rollback(snapshot, file_rel, mod_name, unit_id, e.message)
      false
    end
  end

  def take_snapshot(file_rel, mod_name)
    c_file_path = File.join(SRCDIR, file_rel)
    rs_file_path = File.join(SRCDIR, "core_rs", "src", "#{mod_name}.rs")
    header_path = File.join(SRCDIR, "internal", "core_rs.h")
    lib_rs_path = File.join(SRCDIR, "core_rs", "src", "lib.rs")
    mk_path = File.join(SRCDIR, "core_rs", "core_rs.mk")
    status_path = File.join(SRCDIR, "tool", "porting_status.yml")

    {
      c_file: [c_file_path, File.exist?(c_file_path) ? File.read(c_file_path) : nil],
      rs_file: [rs_file_path, File.exist?(rs_file_path) ? File.read(rs_file_path) : nil],
      header: [header_path, File.exist?(header_path) ? File.read(header_path) : nil],
      lib_rs: [lib_rs_path, File.exist?(lib_rs_path) ? File.read(lib_rs_path) : nil],
      mk: [mk_path, File.exist?(mk_path) ? File.read(mk_path) : nil],
      status: [status_path, File.exist?(status_path) ? File.read(status_path) : nil]
    }
  end

  def rollback(snapshot, file_rel, mod_name, unit_id, reason)
    snapshot.each do |_key, (path, content)|
      if content.nil?
        FileUtils.rm_f(path)
      else
        File.write(path, content)
      end
    end

    update_status_yaml_and_ledger(file_rel, [], status: "Blocked", note: "Blocked: #{reason} (unit #{unit_id})")
  end

  def update_c_source_file(c_file_path, mod_name, unit_id, funcs)
    c_content = File.read(c_file_path, encoding: "binary")

    has_static = funcs.any?(&:static)
    if has_static && !c_content.include?("internal/core_rs.h")
      c_content.sub!(/\A(#.*?\n)/m, "\\1#include \"internal/core_rs.h\"\n")
    end

    funcs.each do |f|
      pattern = Regexp.new("((?:static\\s+)?(?:inline\\s+)?[A-Za-z0-9_\\*\\s]+?\\b#{Regexp.escape(f.name)}\\s*\\([^)]*\\)\\s*\\{.*?\\n\\})", Regexp::MULTILINE)
      if c_content =~ pattern
        matched_func = $1
        macro_def = f.static ? "\n#else\n#define #{f.name} rb_core_#{mod_name}_#{f.name}\n" : "\n"
        guarded = "#if !USE_RUST_PORTS /* ported to core_rs/src/#{mod_name}.rs (port unit #{unit_id}) */\n#{matched_func}#{macro_def}#endif /* !USE_RUST_PORTS */"
        c_content.sub!(matched_func, guarded)
      end
    end

    File.write(c_file_path, c_content)
  end

  def update_internal_header(mod_name, funcs)
    header_path = File.join(SRCDIR, "internal", "core_rs.h")
    return unless File.exist?(header_path)

    header_content = File.read(header_path, encoding: "UTF-8")
    static_funcs = funcs.select(&:static)
    return if static_funcs.empty?

    decls = static_funcs.map do |f|
      "int rb_core_#{mod_name}_#{f.name}();"
    end.join("\n")

    section = "\n/* #{mod_name}.c (core_rs/src/#{mod_name}.rs) */\n#{decls}\n"

    pop_tag = "#if defined(__ELF__) && (defined(__GNUC__) || defined(__clang__))\n# pragma GCC visibility pop"
    if header_content.include?(pop_tag)
      header_content.sub!(pop_tag, "#{section}\n#{pop_tag}")
    else
      header_content.sub!("#endif /* USE_RUST_PORTS */", "#{section}\n#endif /* USE_RUST_PORTS */")
    end

    File.write(header_path, header_content)
  end

  def update_lib_rs(mod_name)
    lib_rs_path = File.join(SRCDIR, "core_rs", "src", "lib.rs")
    lib_rs_content = File.read(lib_rs_path, encoding: "UTF-8")

    mod_decl = "pub mod #{mod_name};"
    unless lib_rs_content.include?(mod_decl)
      lib_rs_content.sub!("pub mod util;\n", "pub mod util;\npub mod #{mod_name};\n")
      File.write(lib_rs_path, lib_rs_content)
    end
  end

  def update_core_rs_mk(mod_name)
    mk_path = File.join(SRCDIR, "core_rs", "core_rs.mk")
    mk_content = File.read(mk_path, encoding: "UTF-8")

    entry = "$(srcdir)/core_rs/src/#{mod_name}.rs"
    unless mk_content.include?(entry)
      mk_content.sub!("$(empty)\n", "\t#{entry} \\\n\t$(empty)\n")
      File.write(mk_path, mk_content)
    end
  end

  def verify_build_and_tests
    cmd_test = "cd #{SRCDIR} && make core-rs-test"
    cmd_check = "cd #{SRCDIR} && ruby tool/core_rs/inventory.rb --check"

    test_ok = system(cmd_test, out: File::NULL, err: File::NULL)
    return false unless test_ok

    check_ok = system(cmd_check, out: File::NULL, err: File::NULL)
    check_ok
  end

  def update_status_yaml_and_ledger(file_rel, funcs, status: "Ported", note: nil)
    status_path = File.join(SRCDIR, "tool", "porting_status.yml")
    return unless File.exist?(status_path)

    basename = File.basename(file_rel)
    status_data = YAML.load_file(status_path) || {}
    files_config = status_data["files"] || {}

    file_entry = files_config[basename] || {}

    if status == "Blocked"
      file_entry["status"] = "Blocked"
      file_entry["notes"] = note || "Blocked in automated pipeline"
    else
      file_entry["status"] = status
      file_entry["notes"] = "Ported in core_rs::#{File.basename(file_rel, '.c').tr('-', '_')}"
    end

    files_config[basename] = file_entry
    status_data["files"] = files_config

    File.write(status_path, YAML.dump(status_data))

    system("cd #{SRCDIR} && ruby tool/generate_porting_ledger.rb", out: File::NULL, err: File::NULL)
  end
end

options = {}
OptionParser.new do |opts|
  opts.banner = "Usage: ruby tool/core_rs/auto_port_unit.rb [options]"

  opts.on("--unit ID", "Port a specific unit ID (e.g. st-A-01)") do |v|
    options[:unit_id] = v
  end

  opts.on("--wave WAVE", "Target wave (A or B)") do |v|
    options[:wave] = v.upcase
  end

  opts.on("--batch", "Run in batch mode for open units") do
    options[:batch] = true
  end

  opts.on("--all", "Port all open units") do
    options[:all] = true
  end

  opts.on("-h", "--help", "Show help message") do
    puts opts
    exit 0
  end
end.parse!

pipeline = AutoPortPipeline.new(options)
pipeline.run
