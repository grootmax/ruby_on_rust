# frozen_string_literal: true
require 'test/unit'

class TestCoercionProtocols < Test::Unit::TestCase
  class CustomNum
    attr_reader :val
    def initialize(val)
      @val = val
    end

    def coerce(other)
      [CustomNum.new(other), self]
    end

    def +(other)
      CustomNum.new(@val + other.val)
    end

    def ==(other)
      other.is_a?(CustomNum) && @val == other.val
    end
  end

  class CustomString
    def initialize(str)
      @str = str
    end

    def to_str
      @str
    end
  end

  class CustomArray
    def initialize(arr)
      @arr = arr
    end

    def to_ary
      @arr
    end
  end

  class CustomInt
    def initialize(int)
      @int = int
    end

    def to_int
      @int
    end
  end

  class ComparableItem
    include Comparable
    attr_reader :score

    def initialize(score)
      @score = score
    end

    def <=>(other)
      return nil unless other.is_a?(ComparableItem)
      @score <=> other.score
    end
  end

  def test_numeric_coerce
    num = CustomNum.new(10)
    result = 5 + num
    assert_kind_of(CustomNum, result)
    assert_equal(CustomNum.new(15), result)
  end

  def test_to_str_implicit_coercion
    cs = CustomString.new("world")
    assert_equal("hello world", "hello " + cs)
    assert_equal("world", String.try_convert(cs))
  end

  def test_to_ary_implicit_coercion
    ca = CustomArray.new([3, 4])
    assert_equal([1, 2, 3, 4], [1, 2] + ca)
    assert_equal([3, 4], Array.try_convert(ca))

    x, y = ca
    assert_equal(3, x)
    assert_equal(4, y)
  end

  def test_to_int_implicit_coercion
    ci = CustomInt.new(2)
    arr = %w[a b c d]
    assert_equal("c", arr[ci])
  end

  def test_comparable_protocol
    item1 = ComparableItem.new(10)
    item2 = ComparableItem.new(20)
    item3 = ComparableItem.new(10)

    assert_true(item1 < item2)
    assert_true(item2 > item1)
    assert_true(item1 == item3)
    assert_true(item1 <= item3)
    assert_true(item1.between?(ComparableItem.new(5), ComparableItem.new(15)))
    assert_equal(item1, ComparableItem.new(5).clamp(item1, item2))
  end
end
