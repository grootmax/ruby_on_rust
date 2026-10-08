#!/usr/bin/env ruby
# Generates jit_offsets.rs from vm_jit_offsets.h and C headers.

require 'tempfile'
require 'fileutils'

output_file = ARGV[0] || 'jit_offsets.rs'

src_dir = File.expand_path('..', __dir__)

cc = ENV['CC'] || 'gcc'
cflags = ENV['CFLAGS'] || ''
cppflags = ENV['CPPFLAGS'] || ''

inc_flags = [
  "-I#{src_dir}",
  "-I#{File.join(src_dir, 'include')}"
]

arch_dirs = Dir.glob(File.join(src_dir, '.ext', 'include', '*'))
arch_dirs.each do |dir|
  inc_flags << "-I#{dir}" if File.directory?(dir)
end

c_code = <<~C
#include "ruby/ruby.h"
#include "internal.h"
#include "internal/class.h"
#include "internal/struct.h"
#include "internal/hash.h"
#include "internal/string.h"
#include "ruby/internal/core/rtypeddata.h"
#include "vm_core.h"
#include "vm_jit_offsets.h"
#include <stdio.h>

int main(void) {
    printf("// Automatically generated from vm_jit_offsets.h by tool/generate_jit_offsets.rb\\n");
    printf("// DO NOT EDIT MANUALLY.\\n\\n");

    printf("pub const ROBJECT_OFFSET_AS_HEAP_FIELDS: i32 = %d;\\n", (int)ROBJECT_OFFSET_AS_HEAP_FIELDS);
    printf("pub const ROBJECT_OFFSET_AS_ARY: i32 = %d;\\n", (int)ROBJECT_OFFSET_AS_ARY);
    printf("pub const RCLASS_OFFSET_PRIME_FIELDS_OBJ: i32 = %d;\\n", (int)RCLASS_OFFSET_PRIME_FIELDS_OBJ);
    printf("pub const TDATA_OFFSET_FIELDS_OBJ: i32 = %d;\\n", (int)TDATA_OFFSET_FIELDS_OBJ);
    printf("pub const RUBY_OFFSET_RHASH_IFNONE: i32 = %d;\\n", (int)RUBY_OFFSET_RHASH_IFNONE);
    printf("pub const RUBY_OFFSET_RHASH_AR_HINT: i32 = %d;\\n", (int)RUBY_OFFSET_RHASH_AR_HINT);
    printf("pub const RUBY_OFFSET_RHASH_AR_PAIRS: i32 = %d;\\n", (int)RUBY_OFFSET_RHASH_AR_PAIRS);
    printf("pub const RUBY_RHASH_AR_TABLE_MAX_SIZE: u32 = %u;\\n", (unsigned)RUBY_RHASH_AR_TABLE_MAX_SIZE);
    printf("pub const RUBY_OFFSET_RSTRING_LEN: i32 = %d;\\n", (int)RUBY_OFFSET_RSTRING_LEN);
    printf("pub const RB_SHAPE_FLAG_SHIFT: u32 = %u;\\n", (unsigned)RB_SHAPE_FLAG_SHIFT);
    printf("pub const RUBY_OFFSET_EC_CFP: i32 = %d;\\n", (int)RUBY_OFFSET_EC_CFP);
    printf("pub const RUBY_OFFSET_EC_INTERRUPT_FLAG: i32 = %d;\\n", (int)RUBY_OFFSET_EC_INTERRUPT_FLAG);
    printf("pub const RUBY_OFFSET_EC_INTERRUPT_MASK: i32 = %d;\\n", (int)RUBY_OFFSET_EC_INTERRUPT_MASK);
    printf("pub const RUBY_OFFSET_EC_THREAD_PTR: i32 = %d;\\n", (int)RUBY_OFFSET_EC_THREAD_PTR);
    printf("pub const RUBY_OFFSET_EC_RACTOR_ID: i32 = %d;\\n", (int)RUBY_OFFSET_EC_RACTOR_ID);

    printf("pub const RUBY_OFFSET_RBASIC_FLAGS: i32 = %d;\\n", (int)RUBY_OFFSET_RBASIC_FLAGS);
    printf("pub const RUBY_OFFSET_RBASIC_KLASS: i32 = %d;\\n", (int)RUBY_OFFSET_RBASIC_KLASS);
    printf("pub const RUBY_OFFSET_RARRAY_AS_HEAP_LEN: i32 = %d;\\n", (int)RUBY_OFFSET_RARRAY_AS_HEAP_LEN);
    printf("pub const RUBY_OFFSET_RARRAY_AS_HEAP_PTR: i32 = %d;\\n", (int)RUBY_OFFSET_RARRAY_AS_HEAP_PTR);
    printf("pub const RUBY_OFFSET_RARRAY_AS_ARY: i32 = %d;\\n", (int)RUBY_OFFSET_RARRAY_AS_ARY);

    printf("pub const RUBY_OFFSET_RSTRUCT_AS_HEAP_PTR: i32 = %d;\\n", (int)RUBY_OFFSET_RSTRUCT_AS_HEAP_PTR);
    printf("pub const RUBY_OFFSET_RSTRUCT_FIELDS_OBJ: i32 = %d;\\n", (int)RUBY_OFFSET_RSTRUCT_FIELDS_OBJ);
    printf("pub const RUBY_OFFSET_RSTRUCT_AS_ARY: i32 = %d;\\n", (int)RUBY_OFFSET_RSTRUCT_AS_ARY);

    printf("pub const RUBY_OFFSET_RSTRING_AS_HEAP_PTR: i32 = %d;\\n", (int)RUBY_OFFSET_RSTRING_AS_HEAP_PTR);
    printf("pub const RUBY_OFFSET_RSTRING_AS_ARY: i32 = %d;\\n", (int)RUBY_OFFSET_RSTRING_AS_ARY);

    printf("pub const RUBY_OFFSET_CFP_PC: i32 = %d;\\n", (int)RUBY_OFFSET_CFP_PC);
    printf("pub const RUBY_OFFSET_CFP_SP: i32 = %d;\\n", (int)RUBY_OFFSET_CFP_SP);
    printf("pub const RUBY_OFFSET_CFP_ISEQ: i32 = %d;\\n", (int)RUBY_OFFSET_CFP_ISEQ);
    printf("pub const RUBY_OFFSET_CFP_SELF: i32 = %d;\\n", (int)RUBY_OFFSET_CFP_SELF);
    printf("pub const RUBY_OFFSET_CFP_EP: i32 = %d;\\n", (int)RUBY_OFFSET_CFP_EP);
    printf("pub const RUBY_OFFSET_CFP_BLOCK_CODE: i32 = %d;\\n", (int)RUBY_OFFSET_CFP_BLOCK_CODE);
    printf("pub const RUBY_OFFSET_CFP_JIT_RETURN: i32 = %d;\\n", (int)RUBY_OFFSET_CFP_JIT_RETURN);
    printf("pub const RUBY_SIZEOF_CONTROL_FRAME: usize = %zu;\\n", (size_t)RUBY_SIZEOF_CONTROL_FRAME);

    printf("pub const RUBY_OFFSET_THREAD_SELF: i32 = %d;\\n", (int)RUBY_OFFSET_THREAD_SELF);

    printf("pub const RUBY_OFFSET_IC_ENTRY: i32 = %d;\\n", (int)RUBY_OFFSET_IC_ENTRY);
    printf("pub const RUBY_OFFSET_ICE_VALUE: i32 = %d;\\n", (int)RUBY_OFFSET_ICE_VALUE);

    return 0;
}
C

rendered_rs = nil

Tempfile.create(['gen_offsets', '.c']) do |c_file|
  c_file.write(c_code)
  c_file.flush

  bin_file = c_file.path + '.bin'
  cmd = "#{cc} #{inc_flags.join(' ')} #{cflags} #{cppflags} -o #{bin_file} #{c_file.path}"

  if system(cmd)
    rendered_rs = `#{bin_file}`
    File.delete(bin_file) if File.exist?(bin_file)
  else
    $stderr.puts "Compilation of offset generator failed. Using fallback static layout."
  end
end

if rendered_rs.nil? || rendered_rs.empty?
  rendered_rs = <<~RUST
    // Fallback constants
    pub const ROBJECT_OFFSET_AS_HEAP_FIELDS: i32 = 16;
    pub const ROBJECT_OFFSET_AS_ARY: i32 = 16;
    pub const RCLASS_OFFSET_PRIME_FIELDS_OBJ: i32 = 40;
    pub const TDATA_OFFSET_FIELDS_OBJ: i32 = 16;
    pub const RUBY_OFFSET_RHASH_IFNONE: i32 = 16;
    pub const RUBY_OFFSET_RHASH_AR_HINT: i32 = 24;
    pub const RUBY_OFFSET_RHASH_AR_PAIRS: i32 = 32;
    pub const RUBY_RHASH_AR_TABLE_MAX_SIZE: u32 = 8;
    pub const RUBY_OFFSET_RSTRING_LEN: i32 = 16;
    pub const RB_SHAPE_FLAG_SHIFT: u32 = 32;
    pub const RUBY_OFFSET_EC_CFP: i32 = 16;
    pub const RUBY_OFFSET_EC_INTERRUPT_FLAG: i32 = 32;
    pub const RUBY_OFFSET_EC_INTERRUPT_MASK: i32 = 36;
    pub const RUBY_OFFSET_EC_THREAD_PTR: i32 = 48;
    pub const RUBY_OFFSET_EC_RACTOR_ID: i32 = 64;
    pub const RUBY_OFFSET_RBASIC_FLAGS: i32 = 0;
    pub const RUBY_OFFSET_RBASIC_KLASS: i32 = 8;
    pub const RUBY_OFFSET_RARRAY_AS_HEAP_LEN: i32 = 16;
    pub const RUBY_OFFSET_RARRAY_AS_HEAP_PTR: i32 = 32;
    pub const RUBY_OFFSET_RARRAY_AS_ARY: i32 = 16;
    pub const RUBY_OFFSET_RSTRUCT_AS_HEAP_PTR: i32 = 32;
    pub const RUBY_OFFSET_RSTRUCT_FIELDS_OBJ: i32 = 16;
    pub const RUBY_OFFSET_RSTRUCT_AS_ARY: i32 = 24;
    pub const RUBY_OFFSET_RSTRING_AS_HEAP_PTR: i32 = 24;
    pub const RUBY_OFFSET_RSTRING_AS_ARY: i32 = 24;
    pub const RUBY_OFFSET_CFP_PC: i32 = 0;
    pub const RUBY_OFFSET_CFP_SP: i32 = 8;
    pub const RUBY_OFFSET_CFP_ISEQ: i32 = 16;
    pub const RUBY_OFFSET_CFP_SELF: i32 = 24;
    pub const RUBY_OFFSET_CFP_EP: i32 = 32;
    pub const RUBY_OFFSET_CFP_BLOCK_CODE: i32 = 40;
    pub const RUBY_OFFSET_CFP_JIT_RETURN: i32 = 48;
    pub const RUBY_SIZEOF_CONTROL_FRAME: usize = 56;
    pub const RUBY_OFFSET_THREAD_SELF: i32 = 16;
    pub const RUBY_OFFSET_IC_ENTRY: i32 = 0;
    pub const RUBY_OFFSET_ICE_VALUE: i32 = 8;
  RUST
end

FileUtils.mkdir_p(File.dirname(output_file))
File.write(output_file, rendered_rs)
puts "Wrote offset constants to #{output_file}"
