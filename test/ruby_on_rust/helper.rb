# frozen_string_literal: true

require "test/unit"
require "core_assertions"

module RubyOnRust
  class TestCase < ::Test::Unit::TestCase
    include Test::Unit::CoreAssertions
  end
end
