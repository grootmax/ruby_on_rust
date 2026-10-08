# frozen_string_literal: true

require 'test/unit'
require_relative '../../tool/ruby_on_rust/fuzz_runner'

class TestFuzzRunnerHarness < Test::Unit::TestCase
  def setup
    @output_dir = File.expand_path('../tmp_fuzz_tests', __dir__)
    FileUtils.rm_rf(@output_dir)
    FileUtils.mkdir_p(@output_dir)
  end

  def teardown
    FileUtils.rm_rf(@output_dir)
  end

  def test_type_generators_coverage
    target_names = RubyOnRust::TypeGenerators::GeneratorFactory.all_target_names
    expected_targets = %w[String Array Hash Integer Float Range Struct Time Comparable Enumerable]

    expected_targets.each do |expected|
      assert_include target_names, expected, "Generator for #{expected} should exist"
    end

    generators = RubyOnRust::TypeGenerators::GeneratorFactory.create('all')
    assert_equal 10, generators.length, "Should instantiate all 10 generators"

    generators.each do |gen|
      snippet = gen.generate_snippet
      assert_kind_of String, snippet
      refute_empty snippet
    end
  end

  def test_divergence_detection_and_shrinking
    mock_c_bin = "ruby"
    mock_rust_bin = "ruby"

    evaluator = RubyOnRust::SubprocessEvaluator.new(
      c_ruby: mock_c_bin,
      rust_ruby: mock_rust_bin,
      dry_run: true
    )

    res1 = RubyOnRust::Result.new
    res1.exit_code = 0
    res1.inspect_str = '"foo"'

    res2 = RubyOnRust::Result.new
    res2.exit_code = 1
    res2.inspect_str = '"bar"'

    assert evaluator.divergent_results?(res1, res2), "Should detect divergence when exit codes or inspect strings differ"

    shrinker = RubyOnRust::Shrinker.new(evaluator)
    sample_snippet = <<~RUBY
      class CustomString < String; end
      a = "a" * 1000
      b = 2**1000
      str = "test"
      str.slice(0, 2)
    RUBY

    shrunk = shrinker.shrink(sample_snippet, 'String')
    assert_kind_of String, shrunk
    refute_empty shrunk
  end

  def test_save_test_case
    runner = RubyOnRust::FuzzRunner.new([
      '--dry-run',
      '--output-dir', @output_dir
    ])

    sample_snippet = 'str = "test"\nstr.slice(0, 1)'
    saved_file = runner.send(:save_test_case, 'String', sample_snippet)

    assert File.exist?(saved_file)
    content = File.read(saved_file)
    assert_match(/class TestFuzzString < Test::Unit::TestCase/, content)
    assert_match(/def test_fuzz_divergence_1/, content)

    # Test appending second divergence
    saved_file2 = runner.send(:save_test_case, 'String', sample_snippet)
    content2 = File.read(saved_file2)
    assert_match(/def test_fuzz_divergence_2/, content2)
  end
end
