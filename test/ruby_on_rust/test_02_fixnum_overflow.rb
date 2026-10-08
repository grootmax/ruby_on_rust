# frozen_string_literal: true
require 'test/unit'

class TestFixnumOverflow < Test::Unit::TestCase
  # 64-bit Fixnum max in CRuby is 2**62 - 1
  FIXNUM_MAX = (1 << 62) - 1
  FIXNUM_MIN = -(1 << 62)

  def test_fixnum_max_type
    assert_kind_of(Integer, FIXNUM_MAX)
    assert_kind_of(Integer, FIXNUM_MIN)
  end

  def test_addition_promotion
    promoted = FIXNUM_MAX + 1
    assert_kind_of(Integer, promoted)
    assert_equal(2**62, promoted)

    # Demotion back when subtracting
    demoted = promoted - 1
    assert_equal(FIXNUM_MAX, demoted)
  end

  def test_subtraction_underflow
    underflowed = FIXNUM_MIN - 1
    assert_kind_of(Integer, underflowed)
    assert_equal(-(2**62) - 1, underflowed)

    # Demotion
    demoted = underflowed + 1
    assert_equal(FIXNUM_MIN, demoted)
  end

  def test_multiplication_promotion
    promoted = FIXNUM_MAX * 2
    assert_kind_of(Integer, promoted)
    assert_equal((2**62 - 1) * 2, promoted)
  end

  def test_exponentiation_promotion
    promoted = 2**63
    assert_kind_of(Integer, promoted)
    assert_equal(9223372036854775808, promoted)
  end

  def test_bitwise_shift_promotion
    promoted = FIXNUM_MAX << 2
    assert_kind_of(Integer, promoted)
    assert_equal((2**62 - 1) * 4, promoted)

    # Shift right demotes back
    demoted = promoted >> 2
    assert_equal(FIXNUM_MAX, demoted)
  end

  def test_bitwise_operations
    big1 = (1 << 65) | 0b1010
    big2 = (1 << 65) | 0b1100
    assert_equal((1 << 65) | 0b1000, big1 & big2)
    assert_equal((1 << 65) | 0b1110, big1 | big2)
    assert_equal(0b0110, big1 ^ big2)
  end
end
