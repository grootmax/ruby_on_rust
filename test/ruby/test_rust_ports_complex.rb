# frozen_string_literal: false
require 'test/unit'

# Ruby on Rust: complex.c's literal scanner (issign, read_sign, isdecimal,
# read_digits, islettere, read_num, read_den, read_rat_nos, read_rat,
# isimagunit, skip_ws) is ported to core_rs/src/complex.rs (port unit
# complex-A-01).
#
# These tests must pass unchanged in both build modes:
#   ./configure                         (Rust ports, USE_RUST_PORTS=1)
#   ./configure --without-rust-ports    (original C, USE_RUST_PORTS=0)
# Expected values are what the original C implementation produces,
# including its quirks: "1__000".to_c is 1, "1e+".to_c is a Float, and
# "1@2.".to_c is accepted although Complex("1@2.") is not.
class TestRustPortsComplex < Test::Unit::TestCase
  # [input, String#to_c, Complex(input), Complex(input, exception: false)]
  CASES = [
    ["", Complex(0, 0), ArgumentError, nil],
    [" ", Complex(0, 0), ArgumentError, nil],
    ["1", Complex(1, 0), Complex(1, 0), Complex(1, 0)],
    ["-1", Complex(-1, 0), Complex(-1, 0), Complex(-1, 0)],
    ["+1", Complex(1, 0), Complex(1, 0), Complex(1, 0)],
    ["1i", Complex(0, 1), Complex(0, 1), Complex(0, 1)],
    ["-i", Complex(0, -1), Complex(0, -1), Complex(0, -1)],
    ["+i", Complex(0, 1), Complex(0, 1), Complex(0, 1)],
    ["i", Complex(0, 1), Complex(0, 1), Complex(0, 1)],
    ["j", Complex(0, 1), Complex(0, 1), Complex(0, 1)],
    ["3j", Complex(0, 3), Complex(0, 3), Complex(0, 3)],
    ["1+2i", Complex(1, 2), Complex(1, 2), Complex(1, 2)],
    ["1-2i", Complex(1, -2), Complex(1, -2), Complex(1, -2)],
    ["1+i", Complex(1, 1), Complex(1, 1), Complex(1, 1)],
    ["1-I", Complex(1, -1), Complex(1, -1), Complex(1, -1)],
    ["1.5", Complex(1.5, 0), Complex(1.5, 0), Complex(1.5, 0)],
    ["1.5e3", Complex(1500.0, 0), Complex(1500.0, 0), Complex(1500.0, 0)],
    ["1.5E-3", Complex(0.0015, 0), Complex(0.0015, 0), Complex(0.0015, 0)],
    ["1e", Complex(1, 0), ArgumentError, nil],
    ["1e+", Complex(1.0, 0), ArgumentError, nil],
    ["1.", Complex(1, 0), ArgumentError, nil],
    [".5", Complex(0.5, 0), Complex(0.5, 0), Complex(0.5, 0)],
    ["1/3", Complex(Rational(1, 3), 0), Complex(Rational(1, 3), 0), Complex(Rational(1, 3), 0)],
    ["1/3+2/5i", Complex(Rational(1, 3), Rational(2, 5)), Complex(Rational(1, 3), Rational(2, 5)), Complex(Rational(1, 3), Rational(2, 5))],
    ["1/", Complex(1, 0), ArgumentError, nil],
    ["1@2", Complex.polar(1, 2), Complex.polar(1, 2), Complex.polar(1, 2)],
    ["1@-2.5", Complex.polar(1, -2.5), Complex.polar(1, -2.5), Complex.polar(1, -2.5)],
    ["1@", Complex(1, 0), ArgumentError, nil],
    ["1@2.", Complex.polar(1, 2), ArgumentError, nil],
    ["1_000", Complex(1000, 0), Complex(1000, 0), Complex(1000, 0)],
    ["1__000", Complex(1, 0), ArgumentError, nil],
    ["1_", Complex(1, 0), ArgumentError, nil],
    ["1_000+2_000i", Complex(1000, 2000), Complex(1000, 2000), Complex(1000, 2000)],
    ["1_e3", Complex(1, 0), ArgumentError, nil],
    ["1e3_0", Complex(1.0e+30, 0), Complex(1.0e+30, 0), Complex(1.0e+30, 0)],
    [" 1 ", Complex(1, 0), Complex(1, 0), Complex(1, 0)],
    ["\t3+4i\n", Complex(3, 4), Complex(3, 4), Complex(3, 4)],
    ["1+2ix", Complex(1, 2), ArgumentError, nil],
    ["1+xi", Complex(1, 0), ArgumentError, nil],
    ["1+3x", Complex(1, 0), ArgumentError, nil],
    ["abc", Complex(0, 0), ArgumentError, nil],
    ["--1", Complex(0, 0), ArgumentError, nil],
    ["1+-2i", Complex(1, 0), ArgumentError, nil],
    ["1.0/2", Complex(Rational(1, 2), 0), Complex(Rational(1, 2), 0), Complex(Rational(1, 2), 0)],
    ["2/0", ZeroDivisionError, ZeroDivisionError, ZeroDivisionError],
    ["0x10", Complex(0, 0), ArgumentError, nil],
    ["1e1000", Complex(Float::INFINITY, 0), Complex(Float::INFINITY, 0), Complex(Float::INFINITY, 0)],
    ["-1e-1000i", Complex(0, -0.0), Complex(0, -0.0), Complex(0, -0.0)],
    ["1.5e3/2", Complex(Rational(750, 1), 0), Complex(Rational(750, 1), 0), Complex(Rational(750, 1), 0)],
    ["\uFF11", Complex(0, 0), ArgumentError, nil],
    ["1 + 2i", Complex(1, 0), ArgumentError, nil],
    ["_1", Complex(0, 0), ArgumentError, nil],
    ["1._5", Complex(1, 0), ArgumentError, nil],
    ["1e_5", Complex(1, 0), ArgumentError, nil],
    ["+_1", Complex(0, 0), ArgumentError, nil],
    ["3@0.5@", Complex.polar(3, 0.5), ArgumentError, nil],
    ["i5", Complex(0, 1), ArgumentError, nil],
    ["1i2", Complex(0, 1), ArgumentError, nil],
    ["\u0661\u0662", Complex(0, 0), ArgumentError, nil],
    ["+.5i", Complex(0, 0.5), Complex(0, 0.5), Complex(0, 0.5)],
    ["-.e5", Complex(0, 0), ArgumentError, nil],
  ]

  def check(expected, actual, msg)
    if expected.is_a?(Complex) && actual.is_a?(Complex)
      # eql? tells 1 from 1.0 and Rational(1, 1) from 1.
      assert(expected.real.eql?(actual.real) && expected.imaginary.eql?(actual.imaginary),
             "#{msg}: expected #{expected.inspect}, got #{actual.inspect}")
    else
      assert_equal(expected, actual, msg)
    end
  end

  def result
    yield
  rescue ArgumentError, ZeroDivisionError => e
    e.class
  end

  def test_string_to_c
    CASES.each { |s, lax, _, _| check(lax, result { s.to_c }, "#{s.dump}.to_c") }
  end

  def test_complex_strict
    CASES.each { |s, _, strict, _| check(strict, result { Complex(s) }, "Complex(#{s.dump})") }
  end

  def test_complex_no_exception
    CASES.each { |s, _, _, nx| check(nx, result { Complex(s, exception: false) }, "Complex(#{s.dump}, exception: false)") }
  end

  # Underscores between digits, at every position of every fragment.
  def test_underscores
    assert_equal(Complex(1000, 2000), "1_000+2_000i".to_c)
    assert_equal(Complex(Rational(1000, 3), 0), "1_000/3".to_c)
    assert_equal(Complex(1.0e30, 0), "1e3_0".to_c)
    assert_raise(ArgumentError) { Complex("1__0") }
    assert_raise(ArgumentError) { Complex("1_") }
  end
end
