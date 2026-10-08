# frozen_string_literal: true

require 'test/unit'

module TestRubyOnRust
  class TestVersion < Test::Unit::TestCase
    def test_ruby_description_includes_rust
      assert_include RUBY_DESCRIPTION, '+RUST'
    end

    def test_ruby_engine_is_ruby
      assert_equal 'ruby', RUBY_ENGINE
    end
  end
end
