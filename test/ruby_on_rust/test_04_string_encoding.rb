# frozen_string_literal: true
require 'test/unit'

class TestStringEncoding < Test::Unit::TestCase
  def test_encodings
    str_utf8 = "hello"
    assert_equal(Encoding::UTF_8, str_utf8.encoding)
    assert_true(str_utf8.valid_encoding?)
    assert_true(str_utf8.ascii_only?)

    str_jp = "こんにちは"
    assert_equal(Encoding::UTF_8, str_jp.encoding)
    assert_equal(5, str_jp.length)
    assert_equal(15, str_jp.bytesize)
    assert_true(str_jp.valid_encoding?)
    assert_false(str_jp.ascii_only?)
  end

  def test_byte_operations
    str = "abc"
    assert_equal(3, str.bytesize)
    assert_equal([97, 98, 99], str.bytes)
    assert_equal(97, str.getbyte(0))
    assert_equal(98, str.getbyte(1))
    assert_equal(99, str.getbyte(2))

    m = str.dup
    m.setbyte(0, 120) # 'x'
    assert_equal("xbc", m)
  end

  def test_invalid_byte_sequence
    invalid_utf8 = +"hello \xFF\xFE world"
    invalid_utf8.force_encoding(Encoding::UTF_8)
    assert_false(invalid_utf8.valid_encoding?)
    assert_false(invalid_utf8.ascii_only?)
  end

  def test_force_encoding_and_encode
    str = "hello"
    b_str = str.b
    assert_equal(Encoding::ASCII_8BIT, b_str.encoding)
    assert_equal("hello", b_str)

    utf16 = str.encode(Encoding::UTF_16LE)
    assert_equal(Encoding::UTF_16LE, utf16.encoding)
    assert_equal(10, utf16.bytesize)
    assert_equal("hello", utf16.encode(Encoding::UTF_8))
  end

  def test_coderange_mutation_updates
    str = +"hello"
    assert_true(str.valid_encoding?)
    assert_true(str.ascii_only?)

    # Appending multi-byte UTF-8 character updates ascii_only
    str << " world \u{1F600}"
    assert_true(str.valid_encoding?)
    assert_false(str.ascii_only?)

    # Setting an invalid byte for UTF-8 invalidates coderange / valid_encoding
    str.setbyte(0, 0xFF)
    assert_false(str.valid_encoding?)
  end
end
