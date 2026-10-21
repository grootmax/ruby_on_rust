# frozen_string_literal: false
require 'test/unit'

# Ruby on Rust: siphash.c (u8to64_le, u64to8_le, rotl64_to, rotl64_swap, add64_to,
# xor64_to, int_sip_dump, int_sip_init, sip_hash_dump) is ported to
# core_rs/src/siphash.rs (port unit siphash-A-01).

class TestRustPortsSiphash < Test::Unit::TestCase
  def test_string_hash
    # Hashing in Ruby uses siphash internally for strings, symbols, and objects.
    assert_kind_of Integer, "hello".hash
    assert_kind_of Integer, "world".hash
    assert_kind_of Integer, "".hash
    assert_equal "hello".hash, "hello".hash
    refute_equal "hello".hash, "world".hash
  end

  def test_symbol_hash
    assert_kind_of Integer, :hello.hash
    assert_equal :hello.hash, :hello.hash
    refute_equal :hello.hash, :world.hash
  end

  def test_integer_hash
    assert_kind_of Integer, 12345.hash
    assert_equal 12345.hash, 12345.hash
  end

  def test_array_and_hash_hash
    arr = [1, "two", :three]
    h = { a: 1, b: "two" }
    assert_kind_of Integer, arr.hash
    assert_kind_of Integer, h.hash
    assert_equal arr.hash, [1, "two", :three].hash
    assert_equal h.hash, { a: 1, b: "two" }.hash
  end
end
