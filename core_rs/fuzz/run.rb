#!/usr/bin/env ruby
# frozen_string_literal: true
#
# core_rs/fuzz/run.rb: build and run differential fuzz units with coverage and inventory gates.
#
#   ruby core_rs/fuzz/run.rb --inventory-check
#   ruby core_rs/fuzz/run.rb [--coverage] --with-ports BUILD_DIR --without-ports BUILD_DIR \
#        [-n CASES] [--seed N] [--mutation-cases N] [--work DIR] UNIT.c...
#
# For each unit harness (core_rs/fuzz/units/<unit>.c) this
#
# 1. validates harness coverage against ported inventory functions (--inventory-check);
# 2. extracts the C reference: every function or static table named on a
#    `FUZZ-EXTRACT: <file.c> name...` line of the harness is copied verbatim
#    from the CRuby source into <work>/<unit>.ref.c, renamed to ref_<name>.
#    `name:FIRST-LAST` copies that line range instead (use it when the
#    definition has #if variants and the range must keep the #if lines);
# 3. copies core_rs's staticlib from the --with-ports build and renames its
#    defined rb_* / ruby_* symbols to rs_*;
# 4. compiles the harness against both and links the remaining C
#    dependencies from the --without-ports build's libruby.so, so nothing
#    the C reference calls can be a Rust port;
# 5. runs the differential fuzz (-n cases, must report 0 mismatches) and
#    then the mutation check (must report > 0 mismatches);
# 6. when --coverage is supplied, builds with profiling instruments, merges
#    LLVM profile data, generates an LCOV report, and enforces 100% branch/statement
#    coverage across all active ported units.
#
# Requirements: Linux (GNU nm/objcopy or their llvm- equivalents), both
# builds configured with --enable-shared. Standard Ruby library only.
# See core_rs/fuzz/README.md.

require "optparse"
require "fileutils"
require "open3"
require "shellwords"
require "json"

SRCDIR = File.expand_path("../..", __dir__)

opts = { n: 1_000_000, seed: 1, mutation_cases: 100_000, work: nil, coverage: false, inventory_check: false }
parser = OptionParser.new do |o|
  o.banner = "usage: #{$0} [--inventory-check] [--coverage] --with-ports DIR --without-ports DIR [options] UNIT.c..."
  o.on("--inventory-check", "verify that all ported functions in inventory.rb have active fuzz harnesses") { opts[:inventory_check] = true }
  o.on("--coverage", "enable LLVM/gcov branch coverage collection and enforce 100% coverage gate") { opts[:coverage] = true }
  o.on("--with-ports DIR", "build tree configured with the Rust ports") { |v| opts[:with] = File.expand_path(v) }
  o.on("--without-ports DIR", "build tree configured --without-rust-ports") { |v| opts[:without] = File.expand_path(v) }
  o.on("-n CASES", Integer, "differential cases (default 1000000)") { |v| opts[:n] = v }
  o.on("--seed N", Integer, "PRNG seed (default 1)") { |v| opts[:seed] = v }
  o.on("--mutation-cases N", Integer, "cases for the mutation check (default 100000)") { |v| opts[:mutation_cases] = v }
  o.on("--work DIR", "where to build (default <with-ports>/target/core_rs/fuzz)") { |v| opts[:work] = File.expand_path(v) }
end
units = parser.parse(ARGV)

def die(msg)
  warn "core_rs/fuzz: #{msg}"
  exit 2
end

def run!(*cmd)
  out, status = Open3.capture2e(*cmd)
  die("command failed: #{cmd.shelljoin}\n#{out}") unless status.success?
  out
end

def check_inventory(srcdir)
  inventory_script = File.join(srcdir, "tool/core_rs/inventory.rb")
  out, status = Open3.capture2("ruby", inventory_script, "--list", "--format", "json")
  die("failed to run inventory script: #{inventory_script}") unless status.success?

  funcs = JSON.parse(out)
  ported_funcs = funcs.select { |f| f["ported"] }

  harness_dir = File.join(srcdir, "core_rs/fuzz/units")
  harness_files = Dir[File.join(harness_dir, "*.c")]

  extracted = {}
  harness_files.each do |harness|
    File.foreach(harness) do |line|
      if line =~ /^\s*\*?\s*FUZZ-EXTRACT:\s*(\S+)\s+(.+?)\s*$/
        c_file = $1.strip
        func_list = $2.strip.split
        func_list.each do |token|
          name = token.split(":").first
          extracted[[File.basename(c_file), name]] ||= []
          extracted[[File.basename(c_file), name]] << File.basename(harness)
        end
      end
    end
  end

  unmapped = []
  mapped_count = 0
  ported_funcs.each do |f|
    file_base = File.basename(f["file"])
    func_name = f["name"]
    if extracted[[file_base, func_name]]
      mapped_count += 1
    else
      unmapped << "#{f["file"]}: #{func_name}"
    end
  end

  if unmapped.empty?
    puts "core_rs/fuzz inventory check: OK (#{mapped_count} ported function(s) mapped to fuzz harnesses)"
    return true
  else
    warn "core_rs/fuzz inventory check: ERROR (#{unmapped.size} ported function(s) lack fuzz harnesses):"
    unmapped.each { |m| warn "  #{m}" }
    return false
  end
end

if opts[:inventory_check] && (units.empty? || !opts[:with] || !opts[:without])
  ok = check_inventory(SRCDIR)
  exit(ok ? 0 : 1)
end

abort parser.help if units.empty? || !opts[:with] || !opts[:without]

if opts[:inventory_check] || opts[:coverage]
  ok = check_inventory(SRCDIR)
  die("inventory check failed") unless ok
end

# A make variable from a configured build tree's Makefile.
def make_var(build, name)
  File.foreach(File.join(build, "Makefile")) do |line|
    return $1.strip if line =~ /\A#{Regexp.escape(name)}\s*=(.*)/
  end
  nil
end

def find_definition(lines, name, file)
  word = /(?<![\w])#{Regexp.escape(name)}\s*(\(|\[|=)/
  lines.each_with_index do |line, i|
    next if line.start_with?(" ", "\t", "#", "/", "*") || line !~ word
    text = +""
    j = i
    while j < lines.size
      text << lines[j]
      break if text.include?("{") || text.include?(";")
      j += 1
    end
    brace = text.index("{")
    semi = text.index(";")
    next unless brace && (!semi || brace < semi)
    first = i
    while first > 0
      prev = lines[first - 1]
      break if prev.strip.empty? || prev.start_with?("#") ||
               prev.rstrip.end_with?(";", "}", "*/", "{") || prev.start_with?(" ", "\t")
      first -= 1
    end
    last = (i...lines.size).find { |k| lines[k].start_with?("}") }
    die("#{file}: no closing brace at column 0 for #{name}") unless last
    proto = nil
    if text =~ word && $1 == "("
      head = (lines[first...i].join + text)
      proto = head[0...head.index("{")].strip.gsub(/\s+/, " ") + ";"
    end
    return [first, last, proto]
  end
  die("#{file}: definition of #{name} not found")
end

def extract_reference(harness, out)
  specs = File.read(harness).scan(/^\s*\*?\s*FUZZ-EXTRACT:\s*(\S+)\s+(.+?)\s*$/)
  die("#{harness}: no FUZZ-EXTRACT line") if specs.empty?
  names = []
  protos = []
  body = +""
  specs.each do |file, list|
    path = File.join(SRCDIR, file)
    lines = File.readlines(path)
    defs = list.split.map do |token|
      if token =~ /\A(\w+):(\d+)-(\d+)\z/
        [$1, $2.to_i - 1, $3.to_i - 1, nil]
      else
        [token, *find_definition(lines, token, file)]
      end
    end
    defs.sort_by { |_, first, _, _| first }.each do |name, first, last, proto|
      names << name
      protos << proto if proto
      body << %(#line #{first + 1} "#{path}"\n)
      body << lines[first..last].join
      body << "\n"
    end
  end
  File.write(out, <<~C)
    /* Generated by core_rs/fuzz/run.rb from the CRuby sources. Do not edit. */
    #{names.map { |n| "#define #{n} ref_#{n}" }.join("\n")}
    #{protos.join("\n")}
    #{body}
    #{names.map { |n| "#undef #{n}" }.join("\n")}
  C
  names
end

def tool(build, var, fallback)
  v = make_var(build, var)
  v = nil if v.nil? || v.empty? || v.include?("$")
  v || fallback
end

def find_llvm_tool(name)
  which_out, st = Open3.capture2("which", name)
  return which_out.strip if st.success? && !which_out.strip.empty?

  sysroot, st = Open3.capture2("rustc", "--print", "sysroot")
  sysroot = sysroot.strip if st.success?
  if sysroot
    tool_path = Dir[File.join(sysroot, "lib/rustlib/*/bin", name)].first
    return tool_path if tool_path && File.exist?(tool_path)
  end

  versioned = Dir["/usr/bin/#{name}-*", "/usr/lib/llvm-*/bin/#{name}"].sort.last
  return versioned if versioned && File.exist?(versioned)

  if system("which rustup >/dev/null 2>&1")
    Open3.capture2("rustup", "component", "add", "llvm-tools")
    Open3.capture2("rustup", "component", "add", "llvm-tools-preview")
    if sysroot
      tool_path = Dir[File.join(sysroot, "lib/rustlib/*/bin", name)].first
      return tool_path if tool_path && File.exist?(tool_path)
    end
  end

  nil
end

def core_rs_cfg_flags(with_build, arch_hdr)
  cc = make_var(with_build, "CC") || "cc"
  cfg_c = File.join(SRCDIR, "core_rs/cfg.c")
  cmd = [*cc.shellsplit, "-E", "-I#{arch_hdr}", "-I#{SRCDIR}/include", "-I#{SRCDIR}", cfg_c]
  out, status = Open3.capture2(*cmd)
  flags = []
  if status.success?
    out.each_line do |line|
      flags << "--cfg" << "core_rs_flonum" if line =~ /^core_rs_cfg_use_flonum 1$/
      flags << "--cfg" << "core_rs_no_flonum" if line =~ /^core_rs_cfg_use_flonum 0$/
    end
  end
  flags = ["--cfg", "core_rs_flonum"] if flags.empty?
  flags
end

def build_rust_lib(with_build, work, arch_hdr, coverage)
  if coverage
    cfg_flags = core_rs_cfg_flags(with_build, arch_hdr)
    cov_lib = File.join(work, "libcore_rs_cov.a")
    src = File.join(SRCDIR, "core_rs/src/lib.rs")
    cmd = ["rustc", "--crate-name=core_rs", "--crate-type=staticlib", "--edition=2024",
           "-g", "-C", "opt-level=0", "-C", "codegen-units=1", "-C", "panic=abort",
           "-C", "overflow-checks=on", "-C", "instrument-coverage",
           *cfg_flags, "-o", cov_lib, src]
    run!(*cmd)
    cov_lib
  else
    lib = File.join(with_build, "target/core_rs/libcore_rs.a")
    die("#{lib} not found: build the --with-ports tree first (make ruby)") unless File.exist?(lib)
    lib
  end
end

def rename_rust(with_build, work, arch_hdr, coverage)
  raw_lib = build_rust_lib(with_build, work, arch_hdr, coverage)
  nm = tool(with_build, "NM", "nm")
  objcopy = tool(with_build, "OBJCOPY", "objcopy")
  syms = run!(nm, "--defined-only", "--extern-only", raw_lib).lines.filter_map do |l|
    f = l.split
    f[2] if f.size == 3 && f[1] =~ /\A[TDRB]\z/ && f[2] =~ /\A(rb_|ruby_)/
  end.uniq
  die("no rb_/ruby_ symbols in #{raw_lib}") if syms.empty?
  map = File.join(work, "rs.syms")
  File.write(map, syms.map { |s| "#{s} rs_#{s}\n" }.join)
  out = File.join(work, "libcore_rs_renamed.a")
  FileUtils.cp(raw_lib, out)
  run!(objcopy, "--redefine-syms=#{map}", out)
  out
end

def libruby_dir(without)
  so = Dir[File.join(without, "libruby.so*")].first
  die("#{without}: libruby.so not found; configure it with --enable-shared") unless so
  die("#{without} is not a --without-rust-ports build") unless make_var(without, "USE_RUST_PORTS") == "0"
  without
end

def arch_hdrdir(build)
  dir = Dir[File.join(build, ".ext/include/*")].find { |d| File.exist?(File.join(d, "ruby/config.h")) }
  die("#{build}: .ext/include/<arch>/ruby/config.h not found") unless dir
  dir
end

def process_coverage(work, with_build, exes, profraws, llvm_cov, llvm_profdata)
  merged_profdata = File.join(work, "merged.profdata")
  run!(llvm_profdata, "merge", "-sparse", *profraws, "-o", merged_profdata)

  lcov_path = File.join(work, "coverage.lcov")
  lcov_out = run!(llvm_cov, "export", exes.first, *(exes[1..].flat_map { |e| ["-object", e] }),
                  "-instr-profile=#{merged_profdata}", "-format=lcov")
  File.write(lcov_path, lcov_out)

  target_lcov = File.join(with_build, "target/core_rs/fuzz/coverage.lcov")
  if lcov_path != target_lcov
    FileUtils.mkdir_p(File.dirname(target_lcov))
    FileUtils.cp(lcov_path, target_lcov)
  end

  json_out = run!(llvm_cov, "export", exes.first, *(exes[1..].flat_map { |e| ["-object", e] }),
                  "-instr-profile=#{merged_profdata}")
  data = JSON.parse(json_out)

  files = data["data"].first["files"]
  ported_files = files.select { |f| f["filename"] =~ %r{core_rs/src/(complex|re|util)\.rs\z} }

  puts
  puts "=" * 80
  puts "core_rs Coverage Report (LLVM-cov)"
  puts "=" * 80
  printf("%-20s %-10s %-10s %-10s %-10s %-10s %-10s\n",
         "Filename", "Lines", "Covered", "Line %", "Regions", "Covered", "Branch %")
  puts "-" * 80

  total_lines = 0
  total_lines_cov = 0
  total_regions = 0
  total_regions_cov = 0
  coverage_failed = false

  summary_rows = []

  ported_files.each do |f|
    fname = File.basename(f["filename"])
    l_sum = f["summary"]["lines"]
    r_sum = f["summary"]["regions"]

    l_cnt, l_cov, l_pct = l_sum["count"], l_sum["covered"], l_sum["percent"].round(2)
    r_cnt, r_cov, r_pct = r_sum["count"], r_sum["covered"], r_sum["percent"].round(2)

    total_lines += l_cnt
    total_lines_cov += l_cov
    total_regions += r_cnt
    total_regions_cov += r_cov

    if l_cov < l_cnt || r_cov < r_cnt
      coverage_failed = true
    end

    row = sprintf("%-20s %-10d %-10d %-10.2f %-10d %-10d %-10.2f",
                  fname, l_cnt, l_cov, l_pct, r_cnt, r_cov, r_pct)
    puts row
    summary_rows << "| `#{fname}` | #{l_cov}/#{l_cnt} (#{l_pct}%) | #{r_cov}/#{r_cnt} (#{r_pct}%) |"
  end

  tot_l_pct = total_lines > 0 ? (total_lines_cov.to_f / total_lines * 100).round(2) : 100.0
  tot_r_pct = total_regions > 0 ? (total_regions_cov.to_f / total_regions * 100).round(2) : 100.0

  puts "-" * 80
  printf("%-20s %-10d %-10d %-10.2f %-10d %-10d %-10.2f\n",
         "TOTAL", total_lines, total_lines_cov, tot_l_pct, total_regions, total_regions_cov, tot_r_pct)
  puts "=" * 80
  puts "LCOV report written to #{target_lcov}"
  puts

  if ENV["GITHUB_STEP_SUMMARY"] && File.exist?(ENV["GITHUB_STEP_SUMMARY"])
    markdown = <<~MD
      ### core_rs Branch & Statement Coverage Report
      | File | Statement Coverage | Branch / Region Coverage |
      |---|---|---|
      #{summary_rows.join("\n")}
      | **Total** | **#{total_lines_cov}/#{total_lines} (#{tot_l_pct}%)** | **#{total_regions_cov}/#{total_regions} (#{tot_r_pct}%)** |

      **LCOV Report:** Artifact saved at `#{target_lcov}`.
    MD
    File.open(ENV["GITHUB_STEP_SUMMARY"], "a") { |f| f.puts markdown }
  end

  if coverage_failed
    warn "core_rs/fuzz: COVERAGE GATE FAILED: 100% branch and statement coverage required on all ported Rust functions."
    exit 1
  end
end

die("#{opts[:with]} does not build the Rust ports") unless make_var(opts[:with], "USE_RUST_PORTS") == "1"
work = opts[:work] || File.join(opts[:with], "target/core_rs/fuzz")
FileUtils.mkdir_p(work)

arch_hdr = arch_hdrdir(opts[:without])
rust_lib = rename_rust(opts[:with], work, arch_hdr, opts[:coverage])
libdir = libruby_dir(opts[:without])
cc = make_var(opts[:without], "CC") || "cc"
failed = false

llvm_cov = nil
llvm_profdata = nil
if opts[:coverage]
  llvm_cov = find_llvm_tool("llvm-cov")
  llvm_profdata = find_llvm_tool("llvm-profdata")
  die("llvm-cov not found") unless llvm_cov
  die("llvm-profdata not found") unless llvm_profdata
end

eh_stub = nil
if opts[:coverage]
  eh_stub = File.join(work, "eh_stub.c")
  File.write(eh_stub, "void rust_eh_personality(void) {}\n")
end

exes = []
profraws = []

units.each do |harness|
  harness = File.expand_path(harness)
  unit = File.basename(harness, ".c")
  ref = File.join(work, "#{unit}.ref.c")
  extract_reference(harness, ref)
  exe = File.join(work, "fuzz_#{unit}")
  exes << exe

  extra_args = []
  extra_args << eh_stub << "-Wl,-u,__llvm_profile_runtime" if opts[:coverage]

  cmd = [*cc.shellsplit, "-std=gnu99", "-O2", "-g", "-Werror=implicit-function-declaration",
         "-I#{work}", "-I#{arch_hdr}", "-I#{SRCDIR}/include", "-I#{SRCDIR}",
         "-I#{File.join(SRCDIR, 'core_rs/fuzz')}", %(-DFUZZ_REF="#{unit}.ref.c"),
         "-o", exe, harness, *extra_args, rust_lib,
         "-L#{libdir}", "-Wl,-rpath,#{libdir}", "-lruby", "-lpthread", "-ldl", "-lm"]
  run!(*cmd)

  diff_env = {}
  mut_env = {}
  if opts[:coverage]
    diff_profraw = File.join(work, "fuzz_#{unit}_diff.profraw")
    mut_profraw = File.join(work, "fuzz_#{unit}_mut.profraw")
    profraws << diff_profraw << mut_profraw
    diff_env["LLVM_PROFILE_FILE"] = diff_profraw
    mut_env["LLVM_PROFILE_FILE"] = mut_profraw
  end

  diff_out, diff_st = Open3.capture2e(diff_env, exe, "-n", opts[:n].to_s, "-s", opts[:seed].to_s)
  puts diff_out
  mut_out, mut_st = Open3.capture2e(mut_env, exe, "-n", opts[:mutation_cases].to_s, "-s", opts[:seed].to_s, "--mutate")
  puts mut_out
  unless diff_st.success?
    warn "core_rs/fuzz: #{unit}: differential run FAILED (mismatches between C and Rust)"
    failed = true
  end
  unless mut_st.success?
    warn "core_rs/fuzz: #{unit}: mutation check FAILED (the harness cannot detect a changed input)"
    failed = true
  end
end

if opts[:coverage] && !failed
  process_coverage(work, opts[:with], exes, profraws, llvm_cov, llvm_profdata)
end

exit(failed ? 1 : 0)
