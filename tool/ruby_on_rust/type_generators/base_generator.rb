# frozen_string_literal: true

module RubyOnRust
  module TypeGenerators
    class BaseGenerator
      attr_reader :random

      def initialize(random = Random.new)
        @random = random
      end

      def choice(array)
        array[@random.rand(array.length)]
      end

      def rand_bool(prob = 0.5)
        @random.rand < prob
      end

      def rand_int(min, max)
        @random.rand(min..max)
      end

      # Helper prelude definitions for custom subclasses and coercion hooks
      def setup_prelude
        <<~RUBY
          class CustomString < String; end
          class CustomArray < Array; end
          class CustomHash < Hash; end

          class CoercibleInt
            def initialize(val); @val = val; end
            def to_int; @val; end
          end

          class CoercibleStr
            def initialize(val); @val = val; end
            def to_str; @val; end
          end

          class CoercibleAry
            def initialize(val); @val = val; end
            def to_ary; @val; end
          end

          class CoercibleHash
            def initialize(val); @val = val; end
            def to_hash; @val; end
          end

          class CoercibleFloat
            def initialize(val); @val = val; end
            def to_f; @val; end
          end

          class CustomComp
            include Comparable
            attr_reader :val
            def initialize(val); @val = val; end
            def <=>(other)
              return nil unless other.is_a?(CustomComp)
              @val <=> other.val
            end
          end

          class CustomEnum
            include Enumerable
            def initialize(items); @items = items; end
            def each(&block)
              @items.each(&block)
            end
          end
        RUBY
      end

      def edge_string_exprs
        [
          '""',
          '"hello world"',
          '"a" * 1000',
          '"\x80\xFF\xFE\x00".b', # invalid UTF-8 / ASCII-8BIT
          '"hello".encode("UTF-16LE")',
          '"hello".encode("ISO-8859-1")',
          '"hello".encode("EUC-JP")',
          '"\u{1F600}\u{1F30C}"',
          '"frozen_str".freeze',
          '"" .freeze',
          'CustomString.new("subclass")',
          'CoercibleStr.new("coerced")',
          '"line1\nline2\r\nline3\n"'
        ]
      end

      def edge_integer_exprs
        [
          '0',
          '1',
          '-1',
          '42',
          '4611686018427387903',  # Fixnum max boundary
          '-4611686018427387904', # Fixnum min boundary
          '2**100',
          '-2**100',
          '2**1000',
          '-2**1000',
          'CoercibleInt.new(123)'
        ]
      end

      def edge_float_exprs
        [
          '0.0',
          '-0.0',
          '1.0',
          '-1.0',
          '3.141592653589793',
          'Float::NAN',
          'Float::INFINITY',
          '-Float::INFINITY',
          'Float::MAX',
          'Float::MIN',
          'Float::EPSILON',
          'CoercibleFloat.new(2.718)'
        ]
      end

      def edge_array_exprs
        [
          '[]',
          '[nil]',
          '[1, 2, 3]',
          '["a", "b", "c"]',
          '[1, "a", :sym, Float::NAN, -0.0]',
          '[1, [2, [3, 4]]]',
          'Array.new(500) { |i| i }',
          '[1, 2, 3].freeze',
          '[].freeze',
          'CustomArray.new([10, 20, 30])',
          'CoercibleAry.new([100, 200])'
        ]
      end

      def edge_hash_exprs
        [
          '{}',
          '{ a: 1, b: 2 }',
          '{ "str" => "val", 123 => 456, :sym => nil }',
          'Hash.new(0).tap { |h| h[:a] = 1 }',
          'Hash.new { |h, k| h[k] = [] }',
          '{ a: 1, b: 2 }.freeze',
          '{}.freeze',
          'CustomHash.new.tap { |h| h[:x] = 100 }',
          'CoercibleHash.new({ k: "v" })'
        ]
      end

      def edge_range_exprs
        [
          '1..10',
          '1...10',
          '-10..10',
          '10..1', # empty range
          '"a".."z"',
          '0.0..1.0',
          '1..', # endless
          '..10', # beginless
          '1...1'
        ]
      end

      def edge_struct_exprs
        [
          'Struct.new(:x, :y).new(1, 2)',
          'Struct.new(:a, :b, :c).new("foo", nil, [1, 2])',
          'Struct.new(:name).new("test").freeze',
          'Struct.new(:val).new(Float::NAN)'
        ]
      end

      def edge_time_exprs
        [
          'Time.at(0)',
          'Time.at(-100000)',
          'Time.at(2**40)',
          'Time.now',
          'Time.utc(2026, 10, 8, 12, 0, 0)',
          'Time.at(100, 123456, :nsec)'
        ]
      end
    end
  end
end
