#!/usr/bin/env ruby
# frozen_string_literal: true
#
# core_rs/fuzz/run.rb: build and run one differential fuzz unit.
#
#   ruby core_rs/fuzz/run.rb --with-ports BUILD_DIR --without-ports BUILD_DIR \
#        [-n CASES] [--seed N] [--mutation-cases N] [--work DIR] UNIT.c...
#
# For each unit harness (core_rs/fuzz/units/<unit>.c) this
#
# 1. extracts the C reference: every function or static table named on a
#    `FUZZ-EXTRACT: <file.c> name...` line of the harness is copied verbatim
#    from the CRuby source into <work>/<unit>.ref.c, renamed to ref_<name>.
#    `name:FIRST-LAST` copies that line range instead (use it when the
#    definition has #if variants and the range must keep the #if lines);
# 2. copies core_rs's staticlib from the --with-ports build and renames its
#    defined rb_* / ruby_* symbols to rs_*;
# 3. compiles the harness against both and links the remaining C
#    dependencies from the --without-ports build's libruby.so, so nothing
#    the C reference calls can be a Rust port;
# 4. runs the differential fuzz (-n cases, must report 0 mismatches) and
#    then the mutation check (must report > 0 mismatches).
#
# Requirements: Linux (GNU nm/objcopy or their llvm- equivalents), both
# builds configured with --enable-shared.  Standard Ruby library only.
# See core_rs/fuzz/README.md.

require "optparse"
require "fileutils"
require "open3"
require "shellwords"

SRCDIR = File.expand_path("../..", __dir__)

opts = { n: 1_000_000, seed: 1, mutation_cases: 100_000, work: nil }
parser = OptionParser.new do |o|
  o.banner = "usage: #{$0} --with-ports DIR --without-ports DIR [options] UNIT.c..."
  o.on("--with-ports DIR", "build tree configured with the Rust ports") { |v| opts[:with] = File.expand_path(v) }
  o.on("--without-ports DIR", "build tree configured --without-rust-ports") { |v| opts[:without] = File.expand_path(v) }
  o.on("-n CASES", Integer, "differential cases (default 1000000)") { |v| opts[:n] = v }
  o.on("--seed N", Integer, "PRNG seed (default 1)") { |v| opts[:seed] = v }
  o.on("--mutation-cases N", Integer, "cases for the mutation check (default 100000)") { |v| opts[:mutation_cases] = v }
  o.on("--work DIR", "where to build (default <with-ports>/target/core_rs/fuzz)") { |v| opts[:work] = File.expand_path(v) }
end
units = parser.parse(ARGV)
abort parser.help if units.empty? || !opts[:with] || !opts[:without]

def die(msg)
  warn "core_rs/fuzz: #{msg}"
  exit 2
end

def run!(*cmd)
  out, status = Open3.capture2e(*cmd)
  die("command failed: #{cmd.shelljoin}\n#{out}") unless status.success?
  out
end

# A make variable from a configured build tree's Makefile.
def make_var(build, name)
  File.foreach(File.join(build, "Makefile")) do |line|
    return $1.strip if line =~ /\A#{Regexp.escape(name)}\s*=(.*)/
  end
  nil
end

# The definition of `name` (a function or a file-scope table) in `lines`,
# as [first_line_index, last_line_index, prototype_or_nil].  CRuby style is
# assumed: the name starts a line (return type on the line before) or
# follows the type on the same line, and the closing brace of the
# definition is at column 0.
def find_definition(lines, name, file)
  word = /(?<![\w])#{Regexp.escape(name)}\s*(\(|\[|=)/
  lines.each_with_index do |line, i|
    next if line.start_with?(" ", "\t", "#", "/", "*") || line !~ word
    # Join until the first `{` or `;` to tell a definition from a prototype.
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
    # Walk up over the return type / storage class lines.
    first = i
    while first > 0
      prev = lines[first - 1]
      break if prev.strip.empty? || prev.start_with?("#") ||
               prev.rstrip.end_with?(";", "}", "*/", "{") || prev.start_with?(" ", "\t")
      first -= 1
    end
    last = (i...lines.size).find { |k| lines[k].start_with?("}") }
    die("#{file}: no closing brace at column 0 for #{name}") unless last
    # A prototype lets callers precede callees (e.g. ruby_scan_oct calls
    # ruby_scan_digits, which util.c defines later).
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
        # Explicit line range, e.g. for a definition with #if variants.
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

def rename_rust(with, work)
  lib = File.join(with, "target/core_rs/libcore_rs.a")
  die("#{lib} not found: build the --with-ports tree first (make ruby)") unless File.exist?(lib)
  nm = tool(with, "NM", "nm")
  objcopy = tool(with, "OBJCOPY", "objcopy")
  syms = run!(nm, "--defined-only", "--extern-only", lib).lines.filter_map do |l|
    f = l.split
    f[2] if f.size == 3 && f[1] =~ /\A[TDRB]\z/ && f[2] =~ /\A(rb_|ruby_)/
  end.uniq
  die("no rb_/ruby_ symbols in #{lib}") if syms.empty?
  map = File.join(work, "rs.syms")
  File.write(map, syms.map { |s| "#{s} rs_#{s}\n" }.join)
  out = File.join(work, "libcore_rs_renamed.a")
  FileUtils.cp(lib, out)
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

die("#{opts[:with]} does not build the Rust ports") unless make_var(opts[:with], "USE_RUST_PORTS") == "1"
work = opts[:work] || File.join(opts[:with], "target/core_rs/fuzz")
FileUtils.mkdir_p(work)
rust_lib = rename_rust(opts[:with], work)
libdir = libruby_dir(opts[:without])
cc = make_var(opts[:without], "CC") || "cc"
failed = false

units.each do |harness|
  harness = File.expand_path(harness)
  unit = File.basename(harness, ".c")
  ref = File.join(work, "#{unit}.ref.c")
  extract_reference(harness, ref)
  exe = File.join(work, "fuzz_#{unit}")
  cmd = [*cc.shellsplit, "-std=gnu99", "-O2", "-g", "-Werror=implicit-function-declaration",
         "-I#{work}", "-I#{arch_hdrdir(opts[:without])}", "-I#{SRCDIR}/include", "-I#{SRCDIR}",
         "-I#{File.join(SRCDIR, 'core_rs/fuzz')}", %(-DFUZZ_REF="#{unit}.ref.c"),
         "-o", exe, harness, rust_lib,
         "-L#{libdir}", "-Wl,-rpath,#{libdir}", "-lruby", "-lpthread", "-ldl", "-lm"]
  run!(*cmd)

  diff_out, diff_st = Open3.capture2e(exe, "-n", opts[:n].to_s, "-s", opts[:seed].to_s)
  puts diff_out
  mut_out, mut_st = Open3.capture2e(exe, "-n", opts[:mutation_cases].to_s, "-s", opts[:seed].to_s, "--mutate")
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
exit(failed ? 1 : 0)
