# frozen_string_literal: true
require 'test/unit'

class TestVmExecCore < Test::Unit::TestCase
  def test_vm_exec_basic_evaluation
    result = 1 + 2 + 3
    assert_eql 6, result
  end

  def test_vm_exec_loop_performance
    sum = 0
    10_000.times do |i|
      sum += i
    end
    assert_eql 49_995_000, sum
  end

  def test_insns_address_table_accessible
    if defined?(RubyVM::INSTRUCTION_NAMES)
      assert_operator RubyVM::INSTRUCTION_NAMES.size, :>, 200
    end
  end
end
