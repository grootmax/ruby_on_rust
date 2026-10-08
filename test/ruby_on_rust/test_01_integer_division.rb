# frozen_string_literal: true
require 'test/unit'

class TestIntegerDivision < Test::Unit::TestCase
  def test_floor_division
    assert_equal(-4, -7 / 2, "Floor division of -7 / 2 must equal -4")
    assert_equal(-4, 7 / -2, "Floor division of 7 / -2 must equal -4")
    assert_equal(3, -7 / -2, "Floor division of -7 / -2 must equal 3")
    assert_equal(3, 7 / 2, "Floor division of 7 / 2 must equal 3")
    assert_equal(-3, -7 / 3, "Floor division of -7 / 3 must equal -3")
  end

  def test_modulo
    assert_equal(2, -7 % 3, "Modulo -7 % 3 must equal 2 (matching divisor sign)")
    assert_equal(-2, 7 % -3, "Modulo 7 % -3 must equal -2 (matching divisor sign)")
    assert_equal(-1, -7 % -3, "Modulo -7 % -3 must equal -1")
    assert_equal(1, 7 % 3, "Modulo 7 % 3 must equal 1")
  end

  def test_divmod
    assert_equal([-3, 2], -7.divmod(3), "divmod -7 by 3")
    assert_equal([-3, -2], 7.divmod(-3), "divmod 7 by -3")
    assert_equal([2, -1], (-7).divmod(-3), "divmod -7 by -3")
    assert_equal([2, 1], 7.divmod(3), "divmod 7 by 3")
  end

  def test_zero_division
    assert_raise(ZeroDivisionError) { 7 / 0 }
    assert_raise(ZeroDivisionError) { -7 / 0 }
    assert_raise(ZeroDivisionError) { 7 % 0 }
    assert_raise(ZeroDivisionError) { 7.divmod(0) }
  end

  def test_bignum_division_modulo
    big = 2**70
    assert_equal(-(big / 3) - 1, (-big) / 3)
    mod = (-big) % 3
    assert_operator(mod, :>=, 0)
    assert_operator(mod, :<, 3)
  end
end
