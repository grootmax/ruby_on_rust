# frozen_string_literal: true
require 'test/unit'

class TestAsymptoticComplexity < Test::Unit::TestCase
  def test_hash_lookup_complexity
    n = 10_000
    h = {}
    n.times { |i| h["key_#{i}"] = i }

    # Measure time for 1,000 lookups in 10,000 element hash
    t0 = Process.clock_gettime(Process::CLOCK_MONOTONIC)
    1000.times do |i|
      k = "key_#{i * 10}"
      v = h[k]
      assert_equal(i * 10, v)
      assert_true(h.key?(k))
    end
    t1 = Process.clock_gettime(Process::CLOCK_MONOTONIC)

    elapsed = t1 - t0
    # 1,000 O(1) hash lookups should comfortably complete in under 0.1s
    assert_operator(elapsed, :<, 0.1, "Hash lookup O(1) bound exceeded: took #{elapsed}s")
  end

  def test_array_indexing_and_push_pop_complexity
    n = 10_000
    arr = []

    # O(1) amortized push
    t0 = Process.clock_gettime(Process::CLOCK_MONOTONIC)
    n.times { |i| arr.push(i) }
    t1 = Process.clock_gettime(Process::CLOCK_MONOTONIC)

    assert_operator(t1 - t0, :<, 0.1, "Array push bound exceeded")

    # O(1) indexing
    t2 = Process.clock_gettime(Process::CLOCK_MONOTONIC)
    1000.times { |i| assert_equal(i * 5, arr[i * 5]) }
    t3 = Process.clock_gettime(Process::CLOCK_MONOTONIC)

    assert_operator(t3 - t2, :<, 0.1, "Array indexing bound exceeded")

    # O(1) pop
    t4 = Process.clock_gettime(Process::CLOCK_MONOTONIC)
    n.times { arr.pop }
    t5 = Process.clock_gettime(Process::CLOCK_MONOTONIC)

    assert_operator(t5 - t4, :<, 0.1, "Array pop bound exceeded")
    assert_empty(arr)
  end
end
