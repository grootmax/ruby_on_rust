# frozen_string_literal: true

require_relative 'base_generator'

module RubyOnRust
  module TypeGenerators
    class IntegerGenerator < BaseGenerator
      def target_name
        'Integer'
      end

      def generate_snippet
        recv = choice(edge_integer_exprs)
        method_call = generate_method_call

        <<~RUBY
          #{setup_prelude}
          num = #{recv}
          #{method_call}
        RUBY
      end

      private

      def generate_method_call
        op = choice([
          :pow, :digits, :upto, :downto, :times, :chr, :gcd, :lcm, :abs,
          :bit_length, :bit_access, :lshift, :rshift, :and_op, :or_op, :xor_op,
          :ceil, :floor, :round, :truncate, :plus, :minus, :multiply, :div, :mod, :exponent
        ])

        case op
        when :pow
          exponent = choice(['0', '1', '2', '10', '100', '-1', '-2'])
          mod = choice([nil, '7', '1000000007', '1', '0'])
          mod ? "num.pow(#{exponent}, #{mod})" : "num.pow(#{exponent})"
        when :digits
          base = choice([nil, '10', '2', '16', '1', '0', '-10'])
          base ? "num.digits(#{base})" : "num.digits"
        when :upto, :downto
          limit = choice(['10', '-10', '0', '100'])
          "num.#{op}(#{limit}).to_a"
        when :times
          "num.times.to_a"
        when :chr
          enc = choice([nil, 'Encoding::UTF_8', 'Encoding::ASCII_8BIT', 'Encoding::ISO_8859_1'])
          enc ? "num.chr(#{enc})" : "num.chr"
        when :gcd, :lcm
          other = choice(edge_integer_exprs)
          "num.#{op}(#{other})"
        when :abs, :bit_length, :ceil, :floor, :truncate
          "num.#{op}"
        when :round
          ndigits = choice([nil, '0', '1', '2', '-1', '-2'])
          ndigits ? "num.round(#{ndigits})" : "num.round"
        when :bit_access
          idx = choice(['0', '1', '31', '63', '100', '-1'])
          "num[#{idx}]"
        when :lshift, :rshift, :and_op, :or_op, :xor_op
          arg = choice(['0', '1', '8', '32', '64', 'CoercibleInt.new(2)'])
          sym = case op
                when :lshift then '<<'
                when :rshift then '>>'
                when :and_op then '&'
                when :or_op then '|'
                when :xor_op then '^'
                end
          "num #{sym} #{arg}"
        when :plus, :minus, :multiply, :div, :mod, :exponent
          other = choice(edge_integer_exprs + edge_float_exprs)
          sym = case op
                when :plus then '+'
                when :minus then '-'
                when :multiply then '*'
                when :div then '/'
                when :mod then '%'
                when :exponent then '**'
                end
          "num #{sym} #{other}"
        else
          "num.inspect"
        end
      end
    end
  end
end
