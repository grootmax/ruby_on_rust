# frozen_string_literal: false
require 'test/unit'
require 'rbconfig'

class TestRubyOnRustSanity < Test::Unit::TestCase
  def test_sanity
    assert_equal "ruby", RUBY_ENGINE
    assert_not_nil RUBY_VERSION
    assert_operator RUBY_VERSION, :>=, "3.0.0"
  end

  def test_build_mode_configuration
    assert_include [true, false], RbConfig::CONFIG.key?("USE_RUST_PORTS")
  end
end
