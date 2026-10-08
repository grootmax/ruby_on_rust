# frozen_string_literal: true

require_relative "helper"

module RubyOnRust
  class TestOptionFilter < RubyOnRust::TestCase
    def test_pattern_matching
      pattern = Test::Unit::RubyOnRustOption::RUBY_ON_RUST_TEST_PATTERN

      assert pattern.match?("/app/ruby_on_rust/test/ruby_on_rust/test_harness_sanity.rb")
      assert pattern.match?("/app/ruby_on_rust/test/ruby_on_rust")
      assert pattern.match?("test/ruby_on_rust/test_harness_sanity.rb")
      assert pattern.match?("ruby_on_rust/test_harness_sanity.rb")
      assert pattern.match?("ruby_on_rust")

      refute pattern.match?("/app/ruby_on_rust/test/-ext-/debug/test_debug.rb")
      refute pattern.match?("test/ruby/test_foo.rb")
      refute pattern.match?("test/ostruct/test_ostruct.rb")
    end

    def test_ruby_on_rust_flag_filters_files
      runner = Test::Unit::AutoRunner.new(true, "test")
      runner.process_args(["--ruby-on-rust"])
      to_run = runner.to_run

      refute_empty to_run
      assert(to_run.all? { |f| Test::Unit::RubyOnRustOption::RUBY_ON_RUST_TEST_PATTERN.match?(f) })
    end

    def test_exclude_ruby_on_rust_flag_excludes_files
      runner = Test::Unit::AutoRunner.new(true, "test/ruby_on_rust")
      runner.process_args(["--exclude-ruby-on-rust"])
      to_run = runner.to_run

      assert_empty to_run
    end
  end
end
