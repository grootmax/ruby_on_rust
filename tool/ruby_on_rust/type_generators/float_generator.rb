# frozen_string_literal: true

require_relative 'base_generator'

module RubyOnRust
  module TypeGenerators
    class FloatGenerator < BaseGenerator
      def target_name
        'Float'
      end

      def generate_snippet
        recv = choice(edge_float_exprs)
        method_call = generate_method_call

        <<~RUBY
          #{setup_prelude}
          flt = #{recv}
          #{method_call}
        RUBY
      end

      private

      def generate_method_call
        op = choice([
          :round, :truncate, :floor, :ceil, :next_float, :prev_float,
          :angle, :abs, :to_s, :to_i, :to_r, :nan?, :infinite?, :zero?,
          :plus, :minus, :multiply, :div, :exponent, :spaceship
        ])

        case op
        when :round, :truncate, :floor, :ceil
          ndigits = choice([nil, '0', '1', '2', '-1', '-2', 'half: :up', 'half: :down', 'half: :even'])
          ndigits ? "flt.#{op}(#{ndigits})" : "flt.#{op}"
        when :next_float, :prev_float, :angle, :abs, :to_s, :to_i, :to_r, :nan?, :infinite?, :zero?
          "flt.#{op}"
        when :plus, :minus, :multiply, :div, :exponent
          other = choice(edge_float_exprs + edge_integer_exprs)
          sym = case op
                when :plus then '+'
                when :minus then '-'
                when :multiply then '*'
                when :div then '/'
                when :exponent then '**'
                end
          "flt #{sym} #{other}"
        when :spaceship
          other = choice(edge_float_exprs + edge_integer_exprs + ['"str"', 'nil'])
          "flt <=> #{other}"
        else
          "flt.inspect"
        end
      end
    end
  end
end
