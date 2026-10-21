# frozen_string_literal: false
require 'test/unit'

# Ruby on Rust: bignum.c's digit-array primitives bary_addc, bary_add,
# bary_add_one, bigdivrem_mulsub, bary_mulsub_1xN, bary_sq_fast, max_size
# and the schoolbook division loop bigdivrem1 are ported to
# core_rs/src/bignum.rs (port unit bignum-A-04).
#
# These tests must pass unchanged in both build modes:
#   ./configure                         (Rust ports, USE_RUST_PORTS=1)
#   ./configure --without-rust-ports    (original C, USE_RUST_PORTS=0)
# Expected values are closed forms checked through hexadecimal strings,
# so they do not depend on the arithmetic under test.
#
# Sizes are chosen around bignum.c's thresholds (32-bit digits):
# below KARATSUBA_MUL_DIGITS (70) squaring uses bary_sq_fast; Karatsuba
# (70..149 digits) uses bary_addc / bary_add / bary_add_one; Toom-3
# (>= 150 digits) uses bary_add and bary_mulsub_1xN; unbalanced products
# go through bary_mul_balance_with_mulfunc (max_size).  Division by a
# multi-digit divisor runs bigdivrem1 (through rb_nogvl() above 10000
# digits).
class TestRustPortsBignum04 < Test::Unit::TestCase
  D = 32 # bits per BDIGIT

  def ones(bits)
    (1 << bits) - 1
  end

  def hex(n)
    n.to_s(16)
  end

  # A new Integer object equal to x, so that x * copy(x) is a general
  # product rather than a square.
  def copy(x)
    x + 1 - 1
  end

  def test_add_carry_chains
    [2, 3, 7, 40, 69, 70, 150, 300].each do |k|
      a = ones(D * k)
      assert_equal("1" + "0" * (8 * k), hex(a + 1), "k=#{k}")
      assert_equal("1" + "0" * (8 * k), hex(1 + a), "k=#{k}")
      assert_equal("1" + "f" * (8 * k - 1) + "e", hex(a + a), "k=#{k}")
      # Shorter operand, carry stopping in the middle, zero tail.
      b = (1 << (D * k)) + 5
      assert_equal("1" + "0" * (8 * k - 1) + "6", hex(b + 1), "k=#{k}")
      assert_equal("2" + "0" * (8 * k - 2) + "64", hex(b + (1 << (D * k)) + 0x5f), "k=#{k}")
    end
  end

  def test_add_mixed_signs
    a = ones(D * 5)
    b = 1 << (D * 3)
    sum = "1" + "0" * 16 + "f" * 24 # a + b
    assert_equal(sum, hex(a + b))
    assert_equal(sum, hex(b + a))
    assert_equal(sum, hex(a - (-b)))
    assert_equal("-" + sum, hex(-a - b))
    assert_equal("-" + sum, hex(-b + -a))
    assert_equal("-1" + "0" * 40, hex(-a - 1))
  end

  def test_complement_add_one
    # Integer#~ on a positive Bignum increments its digits (bary_add_one),
    # on a negative one it increments the negated digits.
    [2, 3, 10, 100].each do |k|
      a = ones(D * k)
      assert_equal("-1" + "0" * (8 * k), hex(~a), "k=#{k}")
      assert_equal(hex(a), hex(~(-(1 << (D * k)))), "k=#{k}")
      b = (1 << (D * k)) + 7
      assert_equal("-" + "1" + "0" * (8 * k - 1) + "8", hex(~b), "k=#{k}")
      assert_equal("1" + "0" * (8 * k - 1) + "6", hex(~(-b)), "k=#{k}")
    end
  end

  def test_square_all_ones
    # (2**n - 1)**2 == 2**2n - 2**(n+1) + 1
    [2, 3, 5, 17, 69, 70, 100, 149, 150, 400].each do |k|
      x = ones(D * k)
      want = "f" * (8 * k - 1) + "e" + "0" * (8 * k - 1) + "1"
      assert_equal(want, hex(x * x), "k=#{k}")
      assert_equal(want, hex(x * copy(x)), "k=#{k}")
    end
  end

  def test_square_sparse_and_high_bit
    [2, 5, 30, 69, 120, 200].each do |k|
      # Zero digits in the middle: (2**n + 1)**2 == 2**2n + 2**(n+1) + 1.
      x = (1 << (D * k)) + 1
      assert_equal("1" + "0" * (8 * k - 1) + "2" + "0" * (8 * k - 1) + "1", hex(x * x), "k=#{k}")
      # Top bit of each digit set (vh in bary_sq_fast): 0x8000...0001.
      y = (1 << (D * k - 1)) | 1
      assert_equal(y * copy(y), y * y, "k=#{k}")
      assert_equal((1 << (2 * D * k - 2)) + (1 << (D * k)) + 1, y * y, "k=#{k}")
    end
  end

  def test_square_matches_general_product
    seed = 0x1234_5678_9abc_def0
    [2, 3, 8, 33, 69, 71, 149, 151, 300].each do |k|
      x = 0
      k.times do
        seed = (seed * 6364136223846793005 + 1442695040888963407) & ones(64)
        x = (x << D) | (seed >> 32)
      end
      x |= 1 << (D * k - 1)
      assert_equal(x * copy(x), x * x, "k=#{k}")
      assert_equal(x * x, x ** 2, "k=#{k}")
      assert_equal(x * x * x, x ** 3, "k=#{k}")
    end
  end

  def test_karatsuba_and_toom3_products
    # (2**n - 1) * (2**m + 1) == 2**(n+m) + 2**n - 2**m - 1
    [[80, 80], [100, 120], [149, 149], [150, 150], [160, 300], [400, 400]].each do |n, m|
      a = ones(D * n)
      b = (1 << (D * m)) + 1
      assert_equal((1 << (D * (n + m))) + (1 << (D * n)) - (1 << (D * m)) - 1, a * b, "n=#{n} m=#{m}")
      assert_equal(a * b, b * a, "n=#{n} m=#{m}")
      # (2**n - 1) * (2**n - 1) through a general product.
      assert_equal("f" * (8 * n - 1) + "e" + "0" * (8 * n - 1) + "1", hex(a * copy(a)), "n=#{n}")
    end
  end

  def test_unbalanced_products
    # Very different lengths go through bary_mul_balance_with_mulfunc.
    [[3, 200], [80, 1000], [150, 2000], [10, 5000]].each do |n, m|
      a = ones(D * n)
      b = ones(D * m)
      want = "f" * (8 * n - 1) + "e" + "f" * (8 * (m - n)) + "0" * (8 * n - 1) + "1"
      assert_equal(want, hex(a * b), "n=#{n} m=#{m}")
      assert_equal(want, hex(b * a), "n=#{n} m=#{m}")
    end
  end

  def assert_divmod(x, y, msg = nil)
    q, r = x.divmod(y)
    assert_equal(x, q * y + r, msg)
    if y > 0
      assert_operator(r, :>=, 0, msg)
      assert_operator(r, :<, y, msg)
    else
      assert_operator(r, :<=, 0, msg)
      assert_operator(r, :>, y, msg)
    end
    assert_equal(q, x / y, msg)
    assert_equal(r, x % y, msg)
    [q, r]
  end

  def test_divmod_closed_forms
    # 2**2n - 1 == (2**n - 1) * (2**n + 1)
    [2, 3, 7, 50, 200].each do |k|
      x = ones(2 * D * k)
      assert_equal([ones(D * k), 0], x.divmod((1 << (D * k)) + 1), "k=#{k}")
      assert_equal([(1 << (D * k)) + 1, 0], x.divmod(ones(D * k)), "k=#{k}")
      assert_equal([ones(D * k), ones(D * k)], x.divmod(1 << (D * k)), "k=#{k}")
      # Remainder that needs the "add back" step: (2**2n - 1) divided by
      # 2**n + 2**(n-1) (top digit 0xc000... after normalization).
      assert_divmod(x, (1 << (D * k)) + (1 << (D * k - 1)), "k=#{k}")
    end
    assert_equal([10**50, 7], (10**100 + 7).divmod(10**50))
    assert_equal([10**50 + 1, 10**49], (10**100 + 10**50 + 10**49).divmod(10**50))
  end

  def test_divmod_identities
    seed = 0x0fed_cba9_8765_4321
    rnd = lambda do |digits|
      v = 0
      digits.times do
        seed = (seed * 6364136223846793005 + 1442695040888963407) & ones(64)
        v = (v << D) | (seed >> 32)
      end
      v
    end
    [[3, 2], [5, 2], [10, 3], [40, 39], [100, 7], [200, 100], [300, 299]].each do |xn, yn|
      x = rnd.(xn) | (1 << (D * xn - 1))
      [rnd.(yn) | (1 << (D * yn - 1)),     # normalized divisor (shift 0)
       rnd.(yn) | 1,                       # divisor needing a shift
       ones(D * yn),                       # top digit 0xffffffff
       (1 << (D * (yn - 1))) * 3 + 1].each do |y|
        next if y.bit_length > x.bit_length
        assert_divmod(x, y, "xn=#{xn} yn=#{yn}")
        assert_divmod(-x, y, "xn=#{xn} yn=#{yn}")
        assert_divmod(x, -y, "xn=#{xn} yn=#{yn}")
        assert_divmod(-x, -y, "xn=#{xn} yn=#{yn}")
      end
    end
  end

  def test_divmod_large_without_gvl
    # More than 10000 digits: bigdivrem_restoring runs bigdivrem1 through
    # rb_nogvl().
    n = D * 6000
    x = ones(2 * n)
    assert_equal([ones(n), 0], x.divmod((1 << n) + 1))
    y = (1 << (n + 77)) + 12345
    assert_divmod(x, y)
  end

  def test_divmod_resumes_after_trap
    # A signal trap handler that raises nothing makes bigdivrem1 stop (it
    # stores its progress in bds->zn) and bigdivrem_restoring resume it.
    sig = Signal.list.key?("USR2") ? :USR2 : nil
    omit "SIGUSR2 is not available" unless sig
    n = D * 15000
    x = ones(2 * n)
    y = (1 << n) + 1
    hits = 0
    old = trap(sig) { hits += 1 }
    begin
      done = false
      pid = Process.pid
      sender = Thread.new do
        until done
          Process.kill(sig, pid)
          sleep 0.005
        end
      end
      q, r = x.divmod(y)
      done = true
      sender.join
    ensure
      trap(sig, old)
    end
    assert_equal(ones(n), q)
    assert_equal(0, r)
  end
end
