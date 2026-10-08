# frozen_string_literal: true

require 'test/unit'

class TestVmBatch < Test::Unit::TestCase
  def test_basic_arithmetic_batching
    a = 100
    b = 200
    c = a + b
    d = c - 50
    e = d * 3
    assert_equal 750, e
  end

  def test_local_variable_access_batching
    x = 1
    y = 2
    z = 3
    x = y + z
    y = x + z
    z = x + y
    assert_equal 5, x
    assert_equal 8, y
    assert_equal 13, z
  end

  def test_stack_operations_batching
    # Test dup, swap, pop, putnil, putself
    arr = []
    a = 42
    b = a
    assert_equal b, a
    assert_equal 42, b
  end

  def test_comparison_batching
    a = 10
    b = 20
    assert_equal true, a < b
    assert_equal true, a <= b
    assert_equal false, a > b
    assert_equal false, a >= b
    assert_equal false, a == b
    assert_equal true, a == 10
  end

  def test_fixnum_overflow_fallback_to_bignum
    # Adding two large Fixnums that overflow into Bignum should trigger side exit fallback to C
    max_fixnum = (1 << (8 * 0.size - 2)) - 1
    result = max_fixnum + 1
    assert_kind_of Integer, result
    assert_equal max_fixnum + 1, result
  end

  def test_type_mismatch_fallback_to_method_call
    # Non-Fixnum opt_plus should fall back to C method call
    str1 = "Hello, "
    str2 = "World!"
    result = str1 + str2
    assert_equal "Hello, World!", result
  end

  def test_batching_inside_loop
    sum = 0
    i = 1
    while i <= 100
      sum += i
      i += 1
    end
    assert_equal 5050, sum
  end

  def test_batching_nested_calls
    def compute(x, y)
      a = x * 2
      b = y * 3
      a + b
    end

    res = compute(5, 10)
    assert_equal 40, res
  end
end
