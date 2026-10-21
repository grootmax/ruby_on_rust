# frozen_string_literal: false
#
# Ruby-level tests for array.c's rotate_count (port unit array-A-01,
# core_rs/src/array.rs), reached through Array#rotate (rb_ary_rotate_m) and
# Array#rotate! (rb_ary_rotate).  They pass with and without
# --with-rust-ports.
require 'test/unit'

class TestRustPortsArray < Test::Unit::TestCase
  def expected_rotation(a, n)
    a.each_index.map { |i| a[(i + n) % a.size] }
  end

  def test_rotate_matches_modular_index
    a = (1..7).to_a
    (-30..30).each do |n|
      exp = expected_rotation(a, n)
      assert_equal(exp, a.rotate(n), "rotate(#{n})")
      assert_equal(exp, a.dup.rotate!(n), "rotate!(#{n})")
    end
  end

  def test_rotate_extreme_counts
    a = (1..5).to_a
    [2**62, -2**62, 2**63 - 1, -2**63, -2**63 + 1].each do |n|
      exp = expected_rotation(a, n)
      assert_equal(exp, a.rotate(n), "rotate(#{n})")
      assert_equal(exp, a.dup.rotate!(n), "rotate!(#{n})")
    end
  end

  def test_rotate_single_and_empty
    assert_equal([1], [1].rotate(-7))
    assert_equal([1], [1].rotate!(2**63 - 1))
    assert_equal([], [].rotate(3))
    assert_equal([], [].rotate!(-3))
  end

  def test_rotate_multiples_of_size
    a = %w[a b c d]
    [-8, -4, 0, 4, 8, 4 * 2**60].each do |n|
      assert_equal(a, a.rotate(n), "rotate(#{n})")
      assert_equal(a, a.dup.rotate!(n), "rotate!(#{n})")
    end
  end
end
