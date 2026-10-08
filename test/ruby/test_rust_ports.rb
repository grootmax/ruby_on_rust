# frozen_string_literal: false
require 'test/unit'
require 'rbconfig'

# Ruby on Rust: behaviour of C functions that have Rust ports in core_rs/.
#
# These tests must pass unchanged in both build modes:
#   ./configure                         (Rust ports, USE_RUST_PORTS=1)
#   ./configure --without-rust-ports    (original C, USE_RUST_PORTS=0)
# Expected values are what the original C implementation produces.
class TestRustPorts < Test::Unit::TestCase
  def test_build_mode_is_reported
    assert_include(%w[0 1], RbConfig::CONFIG["USE_RUST_PORTS"].to_s)
  end

  # ruby_scan_digits via Integer parsing (bignum.c str2big paths).
  def test_scan_digits_integer_parsing
    assert_equal(1234567890, Integer("1234567890"))
    assert_equal(0b1011, Integer("0b1011"))
    assert_equal(0o777, Integer("0o777"))
    assert_equal(0xdeadbeef, Integer("0xDEADbeef"))
    assert_equal(35 * 36 + 35, "zz".to_i(36))
    assert_equal(123, "123abc".to_i)
    assert_equal(0, "abc".to_i)
    assert_equal(1, "1_".to_i)
    assert_equal(1_000_000, "1_000_000".to_i)
  end

  def test_scan_digits_word_boundaries
    [32, 63, 64, 65, 127, 128].each do |bits|
      [2**bits - 1, 2**bits, 2**bits + 1].each do |n|
        [2, 8, 10, 16, 36].each do |base|
          s = n.to_s(base)
          assert_equal(n, s.to_i(base), "#{s}.to_i(#{base})")
          assert_equal(-n, "-#{s}".to_i(base), "-#{s}.to_i(#{base})")
          assert_equal(n, Integer(s, base))
        end
      end
    end
  end

  # ruby_scan_digits via Time#strftime width parsing (strftime.c).
  def test_scan_digits_strftime_width
    t = Time.utc(1970, 1, 2, 3, 4, 5)
    assert_equal("0000001970", t.strftime("%10Y"))
    assert_equal("         2", t.strftime("%_10d"))
    assert_equal("05", t.strftime("%2S"))
  end

  # ruby_scan_hex / ruby_scan_oct via regexp escapes (re.c).
  def test_scan_hex_and_oct_regexp_escapes
    assert_match(Regexp.new("\\x41\\x62"), "Ab")
    assert_match(Regexp.new("\\101\\142"), "Ab")
    assert_match(Regexp.new("\\u0041"), "A")
    assert_match(Regexp.new("\\u{1F600}"), "\u{1F600}")
    assert_match(Regexp.new("\\u{41 42}"), "AB")
    assert_no_match(Regexp.new("\\x41"), "B")
  end

  # ruby_scan_hex / ruby_scan_oct via string literal escapes (parser).
  def test_scan_hex_and_oct_string_escapes
    assert_equal("A", eval('"\x41"'))
    assert_equal("Ab", eval('"\101\142"'))
    assert_equal("é", eval('"é"'))
    assert_equal("\u{1F600}", eval('"\u{1F600}"'))
    assert_equal("\x0f", eval('"\xf"'))
    assert_equal("\x01" + "8", eval('"\18"'))
  end

  # rb_memsearch via String#index, #split and #each_line (re.c).  Covers each
  # algorithm: memchr (m == 1), short patterns, Quick Search for binary and
  # for UTF-8, and code-unit aligned search for UTF-16/UTF-32.
  def test_memsearch
    assert_equal(4, "hello world".index("o w"))
    assert_equal(6, "abcabcabcabcabcabd".b.index("abcabcabcabd".b))
    assert_equal(10, "abababababababababac".index("ababababac"))
    assert_equal(3, "ありがとうございます".index("とうござ"))
    assert_equal(5, "ありがとうございます、ありがとうございます".index("ございます、あり"))
    assert_equal(29, ("x" * 30 + "\u{1F600}y").index("x\u{1F600}y"))
    assert_equal(0, "abc".index(""))
    assert_equal(0, "abc".index("abc"))
    assert_nil("abc".index("abcd"))
    assert_equal(["a", "b", "", "c"], "a,b,,c".split(","))
    assert_equal(["one", "two", "three"], "one<>two<>three".split("<>"))
    assert_equal(["a\r\n", "b\r\n", "c"], "a\r\nb\r\nc".each_line("\r\n").to_a)
  end

  def test_memsearch_wide_encodings
    u16 = ->(s) { s.encode("UTF-16LE") }
    u32 = ->(s) { s.encode("UTF-32LE") }
    assert_equal(2, u16["abc"].index(u16["c"]))
    assert_equal(2, u32["abcd"].index(u32["cd"]))
    # The bytes of the needle occur only at an odd offset, which is not a
    # code unit boundary.
    assert_nil(u16["šĀ"].index(u16["\u0001"]))
    assert_nil(u16["愀b"].index(u16["a"]))
  end

  # rb_memcicmp via Float#round(half:) option parsing (numeric.c).
  def test_memcicmp
    assert_equal(3, 2.5.round(half: :up))
    assert_equal(3, 2.5.round(half: "UP"))
    assert_equal(2, 2.5.round(half: "Even"))
    assert_equal(2, 2.5.round(half: :even))
  end

  # ruby_each_words via --enable/--disable option lists (ruby.c).
  def test_each_words_feature_lists
    assert_in_out_err(%w[--disable=gems,did_you_mean -e p(defined?(Gem))], "", ["nil"], [])
    assert_in_out_err(["--disable", " gems ,, did_you_mean ", "-e", "p(defined?(Gem))"], "", ["nil"], [])
    assert_in_out_err(%w[--enable=frozen-string-literal --disable=gems -e p("a".frozen?)], "", ["true"], [])
  end
end
