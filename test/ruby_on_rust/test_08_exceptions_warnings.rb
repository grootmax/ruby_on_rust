# frozen_string_literal: true
require 'test/unit'

class TestExceptionsWarnings < Test::Unit::TestCase
  def test_exception_class_hierarchy
    assert_operator(ArgumentError, :<, StandardError)
    assert_operator(TypeError, :<, StandardError)
    assert_operator(KeyError, :<, IndexError)
    assert_operator(IndexError, :<, StandardError)
    assert_operator(FrozenError, :<, RuntimeError)
    assert_operator(RuntimeError, :<, StandardError)
    assert_operator(StandardError, :<, Exception)
  end

  def test_exception_message_and_backtrace_shape
    err = nil
    begin
      raise ArgumentError, "custom argument error"
    rescue => e
      err = e
    end

    assert_not_nil(err)
    assert_equal(ArgumentError, err.class)
    assert_equal("custom argument error", err.message)

    bt = err.backtrace
    assert_kind_of(Array, bt)
    assert_operator(bt.size, :>, 0)
    # Validate backtrace frame format "file:line:in 'method'"
    first_frame = bt.first
    assert_match(/:\d+:in /, first_frame, "Backtrace frame must match standard file:line:in format")
  end

  def test_key_error_details
    h = { a: 1 }
    err = nil
    begin
      h.fetch(:missing)
    rescue KeyError => e
      err = e
    end

    assert_not_nil(err)
    assert_equal(:missing, err.key)
    assert_equal(h, err.receiver)
  end

  def test_warning_output_capture
    warnings = []
    original_warn = Warning.method(:warn)

    begin
      Warning.define_singleton_method(:warn) do |msg, **kw|
        warnings << msg
      end

      Kernel.warn("test warning message")
      assert_equal(1, warnings.size)
      assert_match(/test warning message/, warnings.first)
    ensure
      Warning.define_singleton_method(:warn, original_warn)
    end
  end
end
