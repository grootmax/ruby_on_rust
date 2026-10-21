# frozen_string_literal: false
require 'test/unit'

# Ruby on Rust: numeric.c helper functions (round, round_half_up,
# round_half_down, round_half_even, double_div_double, rb_dbl_hash,
# int_round_half_even, int_round_half_up, int_round_half_down,
# float_round_overflow, float_round_underflow, ruby_float_step_size)
# ported to core_rs/src/numeric.rs (port unit numeric-A-01).

class TestRustPortsNumeric < Test::Unit::TestCase
  def test_float_round
    assert_equal(2, 1.5.round(half: :up))
    assert_equal(1, 1.5.round(half: :down))
    assert_equal(2, 1.5.round(half: :even))
    assert_equal(2, 2.5.round(half: :even))
    assert_equal(-2, (-1.5).round(half: :up))
    assert_equal(-1, (-1.5).round(half: :down))
    assert_equal(-2, (-1.5).round(half: :even))

    assert_equal(1.23, 1.2345.round(2, half: :up))
    assert_equal(1.24, 1.235.round(2, half: :up))
    assert_equal(1.23, 1.235.round(2, half: :down))
    assert_equal(1.24, 1.235.round(2, half: :even))
    assert_equal(1.25, 1.245.round(2, half: :even))
  end

  def test_float_div
    assert_equal(0.5, 1.0 / 2.0)
    assert_equal(Float::INFINITY, 1.0 / 0.0)
    assert_equal(-Float::INFINITY, 1.0 / -0.0)
    assert( (0.0 / 0.0).nan? )
  end

  def test_float_hash
    assert_kind_of(Integer, 1.5.hash)
    assert_kind_of(Integer, 0.0.hash)
    assert_kind_of(Integer, Float::INFINITY.hash)
    assert_equal(1.5.hash, 1.5.hash)
  end

  def test_integer_round
    assert_equal(20, 15.round(-1, half: :up))
    assert_equal(10, 15.round(-1, half: :down))
    assert_equal(20, 15.round(-1, half: :even))
    assert_equal(20, 25.round(-1, half: :even))
    assert_equal(-20, (-15).round(-1, half: :up))
    assert_equal(-10, (-15).round(-1, half: :down))
    assert_equal(-20, (-15).round(-1, half: :even))
  end

  def test_float_round_limits
    assert_equal(1.23456789, 1.23456789.round(100))
    assert_equal(0, 1.23456789.round(-100))
  end

  def test_float_step_size
    assert_equal(10, (1.0..10.0).step(1.0).size)
    assert_equal(9, (1.0...10.0).step(1.0).size)
    assert_equal(1, (1.0..10.0).step(Float::INFINITY).size)
    assert_equal(10, (10.0..1.0).step(-1.0).size)
  end
end
