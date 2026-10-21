# frozen_string_literal: false
#
# Ruby-level tests for imemo.c's rb_imemo_name (port unit imemo-A-01,
# core_rs/src/imemo.rs), reached through ObjectSpace.dump (objspace_dump.c
# writes "imemo_type" with rb_imemo_name) and ObjectSpace.count_imemo_objects.
# They pass with and without --with-rust-ports.
require 'test/unit'
require 'objspace'
require 'json'

class TestRustPortsImemo < Test::Unit::TestCase
  IMEMO_NAMES = %w[env cref svar throw_data ifunc memo ment iseq tmpbuf
                   cvar_entry callinfo callcache constcache fields subclasses cdhash]

  def test_dump_all_imemo_types_are_known_names
    seen = []
    ObjectSpace.dump_all(output: :string).each_line do |line|
      next unless line.include?('"type":"IMEMO"')
      seen << JSON.parse(line)["imemo_type"]
    end
    assert_not_empty(seen)
    assert_empty(seen.uniq - IMEMO_NAMES, "unexpected imemo_type names")
  end

  def test_dump_shows_common_imemo_types
    names = ObjectSpace.dump_all(output: :string).each_line.filter_map do |line|
      JSON.parse(line)["imemo_type"] if line.include?('"type":"IMEMO"')
    end.uniq
    %w[iseq ment cref].each do |name|
      assert_include(names, name)
    end
  end
end
