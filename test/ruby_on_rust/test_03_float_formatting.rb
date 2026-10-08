# frozen_string_literal: true
require 'test/unit'

class TestFloatFormatting < Test::Unit::TestCase
  def test_float_to_s
    assert_equal("100.0", 100.0.to_s)
    assert_equal("1.0e+20", 1e20.to_s)
    assert_equal("-100.0", (-100.0).to_s)
    assert_equal("0.0", 0.0.to_s)
    assert_equal("-0.0", (-0.0).to_s)
  end

  def test_sprintf_formatting
    assert_equal("100", sprintf("%g", 100.0))
    assert_equal("100.00", sprintf("%.2f", 100.0))
    assert_equal("1.00e+02", sprintf("%.2e", 100.0))
    assert_equal("1e+20", sprintf("%g", 1e20))
  end

  def test_float_parsing
    assert_equal(100.0, Float("100.0"))
    assert_equal(1e20, Float("1e20"))
    assert_equal(100.0, "100.0".to_f)
    assert_equal(1e20, "1e20".to_f)
    assert_equal(-0.5, Float("-0.5"))
    assert_equal(1e-308, Float("1e-308"))
    assert_equal(1.7976931348623157e+308, Float("1.7976931348623157e+308"))
  end

  def test_special_floats
    assert_equal("NaN", Float::NAN.to_s)
    assert_equal("Infinity", Float::INFINITY.to_s)
    assert_equal("-Infinity", (-Float::INFINITY).to_s)

    assert_true(Float::NAN.nan?)
    assert_true(Float::INFINITY.infinite? > 0)
    assert_true((-Float::INFINITY).infinite? < 0)
  end

  def test_nan_comparisons
    assert_false(Float::NAN == Float::NAN)
    assert_true(Float::NAN != Float::NAN)
    assert_nil(Float::NAN <=> 1.0)
    assert_nil(1.0 <=> Float::NAN)
    assert_nil(Float::NAN <=> Float::NAN)
  end
end
