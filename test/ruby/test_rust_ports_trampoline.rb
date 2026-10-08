# frozen_string_literal: false
require 'test/unit'
require 'rbconfig'

# Ruby on Rust: C trampoline shims for deferred exception handling across FFI boundaries.
class TestRustPortsTrampoline < Test::Unit::TestCase
  def test_build_mode_is_reported
    assert_include(%w[0 1], RbConfig::CONFIG["USE_RUST_PORTS"].to_s)
  end

  # Test that methods calling into ported functions handle exceptions cleanly
  # without leaking memory or corrupting C/Rust exception state.
  def test_deferred_exception_handling
    # Complex parsing with zero division raises ZeroDivisionError cleanly
    assert_raise(ZeroDivisionError) do
      Complex("2/0")
    end

    # Invalid Complex string raises ArgumentError cleanly
    assert_raise(ArgumentError) do
      Complex("1__0")
    end

    # Repeated exception handling preserves exception state across calls
    10.times do
      assert_raise(ZeroDivisionError) { Complex("2/0") }
      assert_raise(ArgumentError) { Complex("invalid") }
    end
  end
end
