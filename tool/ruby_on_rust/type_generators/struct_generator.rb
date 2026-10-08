# frozen_string_literal: true

require_relative 'base_generator'

module RubyOnRust
  module TypeGenerators
    class StructGenerator < BaseGenerator
      def target_name
        'Struct'
      end

      def generate_snippet
        recv = choice(edge_struct_exprs)
        method_call = generate_method_call

        <<~RUBY
          #{setup_prelude}
          st = #{recv}
          #{method_call}
        RUBY
      end

      private

      def generate_method_call
        op = choice([
          :members, :values_at, :deconstruct, :dig, :select,
          :to_h, :to_a, :index_op, :index_set_op, :each, :each_pair, :size
        ])

        case op
        when :members, :deconstruct, :to_h, :to_a, :size
          "st.#{op}"
        when :index_op
          idx = choice(['0', '1', ':x', ':a', ':nonexistent', '-1'])
          "st[#{idx}]"
        when :index_set_op
          idx = choice(['0', ':x', ':a'])
          val = choice(['999', '"new_val"', 'nil'])
          "st[#{idx}] = #{val}"
        when :values_at
          idxs = choice(['0, 1', ':x, :y', '-1, 0'])
          "st.values_at(#{idxs})"
        when :dig
          keys = choice(['0', ':a, 0', ':nonexistent'])
          "st.dig(#{keys})"
        when :select
          "st.select { |val| val.is_a?(Numeric) || val.nil? }"
        when :each, :each_pair
          "st.#{op}.to_a"
        else
          "st.inspect"
        end
      end
    end
  end
end
