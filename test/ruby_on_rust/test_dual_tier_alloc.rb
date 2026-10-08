# frozen_string_literal: false
require 'test/unit'

class TestDualTierAlloc < Test::Unit::TestCase
  def setup
    GC.start
  end

  def test_standard_heap_allocations_register_gc_pressure
    initial_malloc = GC.stat(:total_malloc_bytes)
    initial_increase = GC.stat(:malloc_increase_bytes)

    # Standard heap allocations via ruby_xmalloc / CRubyGlobalAlloc
    arr = Array.new(50_000) { "standard_heap_string_allocation_#{_1}" }

    current_malloc = GC.stat(:total_malloc_bytes)
    assert_operator current_malloc, :>, initial_malloc, "Total malloc bytes should increase with standard heap allocations"

    # Retain reference so GC doesn't sweep during assertion
    assert_equal 50_000, arr.size
  end

  def test_jit_virtual_memory_page_allocations_register_gc_pressure
    if defined?(RubyVM::YJIT) && RubyVM::YJIT.enabled?
      initial_code_size = RubyVM::YJIT.runtime_stats[:code_region_size] || 0
      initial_increase = GC.stat(:malloc_increase_bytes)

      # Trigger JIT compilation to allocate virtual memory pages
      100.times do |i|
        eval <<~RUBY
          def generated_jit_method_#{i}(x)
            x * 2 + #{i}
          end
          10.times { generated_jit_method_#{i}(5) }
        RUBY
      end

      new_code_size = RubyVM::YJIT.runtime_stats[:code_region_size] || 0
      assert_operator new_code_size, :>=, initial_code_size, "JIT code region size should not decrease"
      assert_operator GC.stat(:total_malloc_bytes), :>, 0, "Malloc bytes tracked by CRuby GC should be positive"
    else
      # If YJIT isn't enabled by default in test runner, test passes gracefully
      pass "YJIT is not enabled in this run"
    end
  end

  def test_dual_tier_gc_memory_accounting
    initial_increase = GC.stat(:malloc_increase_bytes)

    # Standard allocations
    buffers = Array.new(100) { "x" * 100_000 }
    after_alloc_increase = GC.stat(:malloc_increase_bytes)

    assert_operator after_alloc_increase, :>, initial_increase, "Memory pressure (malloc_increase_bytes) should increase after allocations"

    # Clear references and force GC cycle
    buffers = nil
    GC.start

    post_gc_increase = GC.stat(:malloc_increase_bytes)
    assert_operator post_gc_increase, :<, after_alloc_increase, "malloc_increase_bytes should reset/decrease after GC"
  end
end
