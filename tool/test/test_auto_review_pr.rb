# frozen_string_literal: true

require 'test/unit'
require_relative '../auto_review_pr'

class TestAutoReviewPR < Test::Unit::TestCase
  class DummyClient
    attr_reader :posts, :get_calls

    def initialize(responses = {})
      @responses = responses
      @posts = []
      @get_calls = []
    end

    def get(path)
      @get_calls << path
      @responses[path] || []
    end

    def post(path, body = {})
      @posts << { path: path, body: body }
      { id: 1 }
    end
  end

  def setup
    @client = DummyClient.new
    @auto_review = AutoReviewPR.new(@client)
  end

  def test_check_missing_rule_16_sections_when_all_present
    body = <<~BODY
      ## Objective
      Port complex.c functions.

      ## Ported Functions/Files
      Ported `issign` in `complex.c` to `core_rs::complex::issign`.

      ## Exported Symbols Kept
      None.

      ## Test Run Results
      `cargo test` and `make test` passed cleanly.

      ## Benchmark Numbers
      0% performance degradation.

      ## Unsafe Blocks & Safety Rationales
      No unsafe blocks added.
    BODY

    missing = @auto_review.check_missing_rule_16_sections(body)
    assert_empty missing
  end

  def test_check_missing_rule_16_sections_when_heading_unfilled
    body = <<~BODY
      ## Objective
      Port complex.c functions.

      ## Ported Functions/Files
      Ported `issign` in `complex.c`.

      ## Exported Symbols Kept
      <!-- List any exported C symbols -->

      ## Test Run Results
      <!-- Include test output -->

      ## Benchmark Numbers
      <!-- Benchmark details -->

      ## Unsafe Blocks & Safety Rationales
      <!-- Unsafe blocks details -->
    BODY

    missing = @auto_review.check_missing_rule_16_sections(body)
    assert_equal(
      [
        'Exported Symbols Kept',
        'Test Run Results',
        'Benchmark Numbers',
        'Unsafe Blocks & Safety Rationales'
      ],
      missing
    )
  end

  def test_check_missing_rule_16_sections_when_body_nil_or_empty
    assert_equal AutoReviewPR::RULE_16_SECTIONS.map { |s| s[:title] }, @auto_review.check_missing_rule_16_sections(nil)
    assert_equal AutoReviewPR::RULE_16_SECTIONS.map { |s| s[:title] }, @auto_review.check_missing_rule_16_sections('')
  end

  def test_porting_pr_detection
    assert_true @auto_review.porting_pr?(1, ['PORTING.md'])
    assert_true @auto_review.porting_pr?(1, ['tool/porting_status.yml'])
    assert_true @auto_review.porting_pr?(1, ['core_rs/src/complex.rs'])
    assert_true @auto_review.porting_pr?(1, ['complex.c'])

    assert_false @auto_review.porting_pr?(1, ['README.md'])
    assert_false @auto_review.porting_pr?(1, ['doc/contributing.md'])
  end

  def test_format_rule_16_comment
    missing = ['Benchmark Numbers', 'Unsafe Blocks & Safety Rationales']
    comment = @auto_review.format_rule_16_comment(missing)

    assert_include comment, AutoReviewPR::RULE_16_COMMENT_PREFIX
    assert_include comment, '* Benchmark Numbers'
    assert_include comment, '* Unsafe Blocks & Safety Rationales'
    assert_include comment, '.github/PULL_REQUEST_TEMPLATE.md'
  end

  def test_review_rule_16_compliance_skips_when_already_commented
    comments = [
      ['github-actions[bot]', "#{AutoReviewPR::RULE_16_COMMENT_PREFIX}\n\n* Benchmark Numbers"]
    ]
    pr = { body: '' }

    @auto_review.send(:review_rule_16_compliance, 100, pr, comments)
    assert_empty @client.posts
  end

  def test_review_rule_16_compliance_skips_non_porting_pr
    responses = {
      '/repos/ruby/ruby/pulls/101/files' => [{ filename: 'README.md' }]
    }
    client = DummyClient.new(responses)
    auto_review = AutoReviewPR.new(client)

    pr = { body: '' }
    auto_review.send(:review_rule_16_compliance, 101, pr, [])
    assert_empty client.posts
  end

  def test_review_rule_16_compliance_posts_comment_for_incomplete_porting_pr
    responses = {
      '/repos/ruby/ruby/pulls/102/files' => [{ filename: 'complex.c' }]
    }
    client = DummyClient.new(responses)
    auto_review = AutoReviewPR.new(client)

    pr = { body: "## Objective\nPort complex" }
    auto_review.send(:review_rule_16_compliance, 102, pr, [])

    assert_equal 1, client.posts.size
    post = client.posts.first
    assert_equal '/repos/ruby/ruby/issues/102/comments', post[:path]
    assert_include post[:body][:body], AutoReviewPR::RULE_16_COMMENT_PREFIX
    assert_include post[:body][:body], '* Ported Functions/Files'
  end
end
