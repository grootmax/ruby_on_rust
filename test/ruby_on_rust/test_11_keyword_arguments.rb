# frozen_string_literal: true
require 'test/unit'

class TestKeywordArguments < Test::Unit::TestCase
  def kw_method(a, b: 10, c: 20)
    [a, b, c]
  end

  def splat_kw_method(a, *args, x: 1, **kwargs)
    [a, args, x, kwargs]
  end

  def test_keyword_argument_defaults
    assert_equal([1, 10, 20], kw_method(1))
    assert_equal([1, 30, 20], kw_method(1, b: 30))
    assert_equal([1, 30, 40], kw_method(1, b: 30, c: 40))
  end

  def test_keyword_argument_separation
    # Passing Hash as positional argument when method accepts kwargs only
    h = { b: 30, c: 40 }
    assert_raise(ArgumentError) { kw_method(1, h) } # 2 positional arguments given, expected 1

    # Explicit double-splat converts Hash to kwargs
    assert_equal([1, 30, 40], kw_method(1, **h))
  end

  def test_splat_and_kwargs
    res = splat_kw_method(10, 20, 30, x: 99, y: 100, z: 200)
    assert_equal(10, res[0])
    assert_equal([20, 30], res[1])
    assert_equal(99, res[2])
    assert_equal({ y: 100, z: 200 }, res[3])
  end

  def test_unknown_keyword_argument_error
    assert_raise(ArgumentError) { kw_method(1, unknown_key: 123) }
  end

  def test_missing_positional_argument_error
    assert_raise(ArgumentError) { kw_method }
  end
end
