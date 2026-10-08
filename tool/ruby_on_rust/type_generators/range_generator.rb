# frozen_string_literal: true

require_relative 'base_generator'

module RubyOnRust
  module TypeGenerators
    class RangeGenerator < BaseGenerator
      def target_name
        'Range'
      end

      def generate_snippet
        recv = choice(edge_range_exprs)
        method_call = generate_method_call

        <<~RUBY
          #{setup_prelude}
          rng = #{recv}
          #{method_call}
        RUBY
      end

      private

      def generate_method_call
        op = choice([
          :step, :cover?, :include?, :bsearch, :first, :last,
          :to_a, :min, :max, :count, :size, :triple_equals
        ])

        case op
        when :step
          n = choice([nil, '1', '2', '0.5', '0', '-1', 'CoercibleInt.new(2)'])
          n ? "rng.step(#{n}).to_a" : "rng.step.to_a"
        when :cover?, :include?, :triple_equals
          val = choice(['0', '5', '10', '0.5', '"c"', '"z"', 'Float::NAN', 'nil', '-1'])
          method_name = op == :triple_equals ? "===" : op.to_s
          "rng.#{method_name}(#{val})"
        when :bsearch
          "rng.bsearch { |x| x.is_a?(Numeric) ? x >= 5 : false }"
        when :first, :last
          n = choice([nil, '1', '3', '0', '-1'])
          n ? "rng.#{op}(#{n})" : "rng.#{op}"
        when :min, :max
          "rng.#{op}"
        when :to_a, :count, :size
          "rng.#{op}"
        else
          "rng.inspect"
        end
      end
    end
  end
end
