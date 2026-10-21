# frozen_string_literal: false
#
# Ruby-level tests for the re.c functions of port unit re-A-01
# (core_rs/src/re.rs): char_to_option and option_to_str (Regexp#to_s,
# #inspect, Regexp.new with a string of options, RegexpError messages),
# pair_byte_cmp (MatchData character offsets), match_inspect_name_iter
# (MatchData#inspect with named groups) and read_escaped_byte (escaped bytes
# in a regexp source).  rb_char_to_option_kcode has no caller in CRuby; the
# Rust unit tests and the fuzz unit core_rs/fuzz/units/re_01.c cover it.
# These tests pass with and without --with-rust-ports.
require 'test/unit'

class TestRustPortsRe01 < Test::Unit::TestCase
  def test_to_s_embeds_options
    assert_equal("(?-mix:a)", /a/.to_s)
    assert_equal("(?m-ix:a)", /a/m.to_s)
    assert_equal("(?mix:a)", /a/mix.to_s)
    assert_equal("(?ix-m:a)", /a/xi.to_s)
    # rb_reg_str_with_term folds the leading (?flags-flags:...) group with
    # char_to_option.
    assert_equal("(?i-mx:a)", /(?i-mx:a)/.to_s)
    assert_equal("(?mi-x:a)", Regexp.new("(?im:a)").to_s)
    assert_equal("(?x-mi:a)", Regexp.new("(?mix-mi:a)").to_s)
    assert_equal("(?i-mx:a)", /(?i)a/.to_s)
  end

  def test_inspect_options
    assert_equal("/a/", /a/.inspect)
    assert_equal("/a/mix", /a/mix.inspect)
    assert_equal("/a/mi", /a/im.inspect)
    assert_equal("/a/x", /a/x.inspect)
    assert_equal("/a/n", /a/n.inspect)
  end

  def test_string_options
    assert_equal(Regexp::IGNORECASE | Regexp::EXTENDED | Regexp::MULTILINE, Regexp.new("a", "mix").options)
    assert_equal(Regexp::MULTILINE, Regexp.new("a", "m").options)
    assert_equal(0, Regexp.new("a", "").options)
    assert_raise_with_message(ArgumentError, /unknown regexp option: mz/) { Regexp.new("a", "mz") }
    assert_raise_with_message(ArgumentError, /unknown regexp option: I/) { Regexp.new("a", "I") }
  end

  def test_error_message_options
    assert_raise_with_message(RegexpError, %r{: /\(/ix\z}) { Regexp.new("(", Regexp::IGNORECASE | Regexp::EXTENDED) }
    assert_raise_with_message(RegexpError, %r{: /\(/mix\z}) { Regexp.new("(", Regexp::MULTILINE | Regexp::IGNORECASE | Regexp::EXTENDED) }
    assert_raise_with_message(RegexpError, %r{: /\(/\z}) { Regexp.new("(") }
  end

  def test_char_offsets
    m = "äöü abc".match(/(ö)(ü) (a)(z)?/)
    assert_equal([1, 5], [m.begin(0), m.end(0)])
    assert_equal([1, 2], m.offset(1))
    assert_equal([2, 3], m.offset(2))
    assert_equal([4, 5], m.offset(3))
    assert_equal([nil, nil], m.offset(4))
    assert_equal([7, 8], m.byteoffset(3))
    # Equal byte positions (an empty group) and positions out of order.
    m = "あいう".match(/(い)()((う))/)
    assert_equal([[1, 3], [1, 2], [2, 2], [2, 3], [2, 3]], (0..4).map { |i| m.offset(i) })
    m = "αβγδ".match(/(?=(γ))(β|γ)/)
    assert_equal([[2, 3], [2, 3], [2, 3]], (0..2).map { |i| m.offset(i) })
  end

  def test_inspect_named_groups
    assert_equal('#<MatchData "a" x:"a" y:nil>', /(?<x>a)(?<y>b)?/.match("a").inspect)
    assert_equal('#<MatchData "b" x:nil x:"b">', /(?<x>a)|(?<x>b)/.match("b").inspect)
    assert_equal('#<MatchData "あい" first:"あ" second:"い">', /(?<first>.)(?<second>.)/.match("あい").inspect)
    assert_equal('#<MatchData "ab" 1:"a" 2:"b">', /(a)(b)/.match("ab").inspect)
  end

  def test_escaped_multibyte
    assert_equal(0, Regexp.new("\\xE3\\x81\\x82") =~ "あ")
    assert_equal(0, Regexp.new("\\343\\201\\202") =~ "あ")
    assert_equal(0, Regexp.new("\\xe3\\201\\x82") =~ "あ")
    assert_equal(0, Regexp.new("\\M-c\\M-\\C-A\\M-\\C-b".b) =~ "\xE3\x81\x82".b)
    assert_equal(0, Regexp.new("\\M-\\C-a".b) =~ "\x81".b)
    assert_equal(0, Regexp.new("\\c\\M-a".b) =~ "\x81".b)
    assert_equal(0, Regexp.new("\\M-\\ca".b) =~ "\x81".b)
    assert_equal(0, Regexp.new("\\M-\\n".b) =~ "\x8A".b)
  end

  def test_escaped_us_ascii
    r = Regexp.new("\\M-a\\C-a\\ca\\x41\\101\\n".force_encoding("US-ASCII"))
    assert_equal("\\M-a\\C-a\\ca\\x41\\101\\n", r.source)
    assert_equal(0, r =~ "\xE1\x01\x01AA\n".b)
  end

  def test_escaped_byte_errors
    {
      "\\xE3" => "too short escaped multibyte character",
      "\\xE3\\x81" => "too short escaped multibyte character",
      "\\xE3\\" => "too short escape sequence",
      "\\xE3\\M-\\M-a" => "duplicate meta escape",
      "\\xE3\\M-" => "too short meta escape",
      "\\xE3\\c\\ca" => "duplicate control escape",
      "\\xE3\\C-\\c\\x" => "duplicate control escape",
      "\\xE3\\C" => "too short control escape",
      "\\xE3\\C+a" => "too short control escape",
      "\\xE3\\c" => "too short control escape",
      "\\xE3\\q" => "unexpected escape sequence",
      "\\xE3\\777" => "invalid escape code",
      "\\xE3\\xg" => "invalid hex escape",
      "\\xE3\\M-a" => "invalid multibyte escape",
    }.each do |src, msg|
      assert_raise_with_message(RegexpError, /\A#{Regexp.quote(msg)}: /, src) { Regexp.new(src) }
    end
  end
end
