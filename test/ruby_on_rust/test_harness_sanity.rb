# frozen_string_literal: true

require_relative "helper"

module RubyOnRust
  class TestHarnessSanity < RubyOnRust::TestCase
    def test_harness_sanity
      assert_equal true, true
    end
  end
end
