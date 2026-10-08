# frozen_string_literal: true

require "test/unit"
require_relative "../../tool/check_method_kinds"

class TestMethodKinds < Test::Unit::TestCase
  CORE_C_METHODS = [
    [Array, :map],
    [Array, :push],
    [Array, :pop],
    [Array, :length],
    [String, :length],
    [String, :upcase],
    [Hash, :keys],
    [Hash, :values],
    [Kernel, :puts],
    [Kernel, :raise],
    [Numeric, :abs],
    [Integer, :+],
    [Float, :to_s],
    [Symbol, :to_s],
    [Object, :nil?],
    [NilClass, :nil?],
    [TrueClass, :to_s],
    [FalseClass, :to_s],
    [Math, :sqrt]
  ].freeze

  def test_core_c_methods_source_location_is_nil
    CORE_C_METHODS.each do |klass, method_name|
      unbound_method = klass.instance_method(method_name)
      loc = unbound_method.source_location
      is_native_or_internal = loc.nil? || loc[0].start_with?("<")
      assert is_native_or_internal,
        "Expected #{klass}##{method_name} source_location to be nil or internal (got #{loc.inspect})"
    end
  end

  def test_core_c_methods_iseq_is_nil
    return unless defined?(RubyVM::InstructionSequence)

    CORE_C_METHODS.each do |klass, method_name|
      unbound_method = klass.instance_method(method_name)
      loc = unbound_method.source_location
      next if loc && loc[0].start_with?("<")

      iseq = begin
        RubyVM::InstructionSequence.of(unbound_method)
      rescue StandardError
        nil
      end
      assert_nil iseq,
        "Expected #{klass}##{method_name} InstructionSequence.of to be nil (C/Rust native method)"
    end
  end

  def test_tracepoint_c_call_and_c_return_events
    events = []
    tp = TracePoint.new(:c_call, :c_return) do |t|
      events << {
        event: t.event,
        method_id: t.method_id,
        defined_class: t.defined_class
      }
    end

    tp.enable do
      [1, 2, 3].map { |x| x * 2 }
      "hello".length
      { a: 1 }.keys
    end

    c_calls = events.select { |e| e[:event] == :c_call }
    c_returns = events.select { |e| e[:event] == :c_return }

    assert_operator c_calls.size, :>, 0, "Expected TracePoint :c_call events to be recorded"
    assert_operator c_returns.size, :>, 0, "Expected TracePoint :c_return events to be recorded"

    map_call = c_calls.find { |e| e[:method_id] == :map && e[:defined_class] == Array }
    assert map_call, "Expected TracePoint :c_call event for Array#map"

    map_return = c_returns.find { |e| e[:method_id] == :map && e[:defined_class] == Array }
    assert map_return, "Expected TracePoint :c_return event for Array#map"

    length_call = c_calls.find { |e| e[:method_id] == :length && e[:defined_class] == String }
    assert length_call, "Expected TracePoint :c_call event for String#length"

    keys_call = c_calls.find { |e| e[:method_id] == :keys && e[:defined_class] == Hash }
    assert keys_call, "Expected TracePoint :c_call event for Hash#keys"
  end

  def test_inspector_runner
    inspector = MethodKindsInspector.new(check: true)
    success = inspector.run
    assert success, "Expected MethodKindsInspector to pass with 0 Rule 7 violations"
    assert_operator inspector.total_methods, :>, 1000, "Expected over 1000 total methods scanned"
    assert_operator inspector.c_methods, :>, 1000, "Expected over 1000 C methods identified"
    assert_equal 0, inspector.violations.size, "Expected 0 Rule 7 violations"
  end
end
