# frozen_string_literal: true

require_relative 'base_generator'

module RubyOnRust
  module TypeGenerators
    class ArrayGenerator < BaseGenerator
      def target_name
        'Array'
      end

      def generate_snippet
        recv = choice(edge_array_exprs)
        method_call = generate_method_call

        <<~RUBY
          #{setup_prelude}
          arr = #{recv}
          #{method_call}
        RUBY
      end

      private

      def generate_method_call
        op = choice([
          :slice, :concat, :map, :select, :rotate, :flatten, :zip, :pack,
          :transpose, :combination, :permutation, :product, :compact, :uniq,
          :values_at, :fill, :bsearch, :sort, :index_op, :index_set_op,
          :plus, :minus, :intersection, :union
        ])

        case op
        when :slice, :index_op
          arg1 = choice(['0', '1', '-1', '100', '1..3', '0...5', 'CoercibleInt.new(0)'])
          arg2 = choice([nil, '1', '5', '0', '-1'])
          args = [arg1, arg2].compact.join(', ')
          "arr.#{op == :index_op ? "[]" : op}(#{args})"
        when :index_set_op
          idx = choice(['0', '1', '-1', '10', '1..2'])
          val = choice(['100', '"new_val"', 'nil', '[9, 9]'])
          "arr[#{idx}] = #{val}"
        when :concat, :plus, :minus, :intersection, :union
          other = choice(edge_array_exprs)
          case op
          when :concat then "arr.concat(#{other})"
          when :plus then "arr + #{other}"
          when :minus then "arr - #{other}"
          when :intersection then "arr & #{other}"
          when :union then "arr | #{other}"
          end
        when :map, :select
          "arr.#{op} { |x| x.is_a?(Numeric) ? x * 2 : x.to_s }"
        when :rotate
          n = choice([nil, '1', '-1', '100', 'CoercibleInt.new(2)'])
          n ? "arr.rotate(#{n})" : "arr.rotate"
        when :flatten
          lvl = choice([nil, '0', '1', '2', '-1'])
          lvl ? "arr.flatten(#{lvl})" : "arr.flatten"
        when :zip
          other = choice(edge_array_exprs)
          "arr.zip(#{other})"
        when :pack
          fmt = choice(['"C*"', '"c*"', '"i*"', '"I*"', '"s*"', '"A*"', '"w*"'])
          "arr.pack(#{fmt})"
        when :transpose
          "arr.transpose"
        when :combination, :permutation
          n = choice(['0', '1', '2', '5', '-1'])
          "arr.#{op}(#{n}).to_a"
        when :product
          other = choice(edge_array_exprs)
          "arr.product(#{other})"
        when :compact, :uniq
          "arr.#{op}"
        when :values_at
          idxs = choice(['0, 1', '-1, 2, 10', '0..2, -1'])
          "arr.values_at(#{idxs})"
        when :fill
          val = choice(['0', '"fill"', 'nil'])
          start = choice([nil, '0', '1', '-1'])
          length = choice([nil, '1', '5'])
          args = [val, start, length].compact.join(', ')
          "arr.fill(#{args})"
        when :bsearch
          "arr.bsearch { |x| x.is_a?(Numeric) ? x >= 2 : false }"
        when :sort
          "arr.sort"
        else
          "arr.inspect"
        end
      end
    end
  end
end
