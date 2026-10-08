# frozen_string_literal: true

require 'test/unit'
require_relative '../../tool/doc_lock'

class TestDocLock < Test::Unit::TestCase
  def test_normalize_whitespace
    input = "Line 1  \r\nLine 2\n\n\nLine 3  "
    expected = "Line 1\nLine 2\n\nLine 3"
    assert_equal expected, DocLock.normalize_whitespace(input)
  end

  def test_comparator_detects_mismatches
    ref = {
      'Array' => {
        'name' => 'Array',
        'type' => 'class',
        'comment' => 'Array class doc',
        'methods' => {
          'Array#pack' => {
            'name' => 'pack',
            'full_name' => 'Array#pack',
            'singleton' => false,
            'visibility' => 'public',
            'params' => 'fmt',
            'call_seq' => 'pack(fmt) -> string',
            'comment' => 'Packs elements'
          }
        }
      }
    }

    target = {
      'Array' => {
        'name' => 'Array',
        'type' => 'class',
        'comment' => 'Array class doc',
        'methods' => {
          'Array#pack' => {
            'name' => 'pack',
            'full_name' => 'Array#pack',
            'singleton' => false,
            'visibility' => 'public',
            'params' => 'fmt, buffer: nil', # Parameter mismatch
            'call_seq' => 'pack(fmt) -> string',
            'comment' => 'Packs elements'
          }
        }
      }
    }

    allowlist = DocLock::Allowlist.new
    comparator = DocLock::Comparator.new(ref, target, allowlist)
    results = comparator.compare

    assert_equal 1, results['unapproved'].size
    diff = results['unapproved'].first
    assert_equal 'parameter_mismatch', diff['type']
    assert_equal 'Array#pack', diff['entity']
    assert_equal 'fmt', diff['expected']
    assert_equal 'fmt, buffer: nil', diff['actual']
  end

  def test_allowlist_suppresses_mismatches
    ref = {
      'Array' => {
        'name' => 'Array',
        'type' => 'class',
        'comment' => 'Array class doc',
        'methods' => {
          'Array#pack' => {
            'name' => 'pack',
            'full_name' => 'Array#pack',
            'singleton' => false,
            'visibility' => 'public',
            'params' => 'fmt',
            'call_seq' => 'pack(fmt) -> string',
            'comment' => 'Packs elements'
          }
        }
      }
    }

    target = {
      'Array' => {
        'name' => 'Array',
        'type' => 'class',
        'comment' => 'Array class doc',
        'methods' => {
          'Array#pack' => {
            'name' => 'pack',
            'full_name' => 'Array#pack',
            'singleton' => false,
            'visibility' => 'public',
            'params' => 'fmt, buffer: nil',
            'call_seq' => 'pack(fmt) -> string',
            'comment' => 'Packs elements'
          }
        }
      }
    }

    allowlist = DocLock::Allowlist.new
    allowlist.parameter_mismatches << 'Array#pack'

    comparator = DocLock::Comparator.new(ref, target, allowlist)
    results = comparator.compare

    assert_equal 0, results['unapproved'].size
    assert_equal 1, results['approved'].size
    assert_equal 'Array#pack', results['approved'].first['entity']
  end
end
