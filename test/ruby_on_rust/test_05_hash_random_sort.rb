# frozen_string_literal: true
require 'test/unit'

class TestHashRandomSort < Test::Unit::TestCase
  SortItem = Struct.new(:val, :seq)

  class CustomKey
    attr_reader :id
    def initialize(id)
      @id = id
    end

    def hash
      42 # Constant hash code to force collision
    end

    def eql?(other)
      other.is_a?(CustomKey) && @id == other.id
    end
  end

  def test_hash_insertion_order
    h = {}
    h[:first] = 1
    h[:second] = 2
    h[:third] = 3
    assert_equal([:first, :second, :third], h.keys)
    assert_equal([1, 2, 3], h.values)

    # Deleting and re-inserting moves key to the end
    h.delete(:first)
    h[:first] = 10
    assert_equal([:second, :third, :first], h.keys)
    assert_equal([2, 3, 10], h.values)
  end

  def test_seeded_random_determinism
    r1 = Random.new(42)
    r2 = Random.new(42)

    seq1 = Array.new(10) { r1.rand(1000000) }
    seq2 = Array.new(10) { r2.rand(1000000) }

    assert_equal(seq1, seq2, "Random.new(42) must produce deterministic sequence")
  end

  def test_sort_stability
    # Testing stable sorting of pairs where keys are identical
    items = [
      SortItem.new(2, 1),
      SortItem.new(1, 2),
      SortItem.new(2, 3),
      SortItem.new(1, 4),
      SortItem.new(2, 5)
    ]

    sorted = items.sort_by(&:val)
    vals = sorted.map(&:val)
    seqs = sorted.map(&:seq)

    assert_equal([1, 1, 2, 2, 2], vals)
    assert_equal([2, 4, 1, 3, 5], seqs, "Enumerable#sort_by must be stable")
  end

  def test_hash_key_equality_and_collision
    k1 = CustomKey.new(1)
    k2 = CustomKey.new(2)
    k3 = CustomKey.new(1)

    h = {}
    h[k1] = "value1"
    h[k2] = "value2"

    assert_equal(2, h.size)
    assert_equal("value1", h[k1])
    assert_equal("value2", h[k2])
    assert_equal("value1", h[k3]) # k3.eql?(k1) is true
  end
end
