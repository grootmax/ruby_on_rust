# frozen_string_literal: true

require_relative 'base_generator'

module RubyOnRust
  module TypeGenerators
    class EnumerableGenerator < BaseGenerator
      def target_name
        'Enumerable'
      end

      def generate_snippet
        enum = choice([
          'CustomEnum.new([1, 2, 2, 3, 4, 5])',
          'CustomEnum.new(["apple", "banana", "cherry", "date"])',
          'CustomEnum.new([])',
          'CustomEnum.new([1, "a", nil, :sym, Float::NAN])'
        ])
        method_call = generate_method_call

        <<~RUBY
          #{setup_prelude}
          enum = #{enum}
          #{method_call}
        RUBY
      end

      private

      def generate_method_call
        op = choice([
          :chunk, :slice_before, :slice_after, :slice_when, :tally,
          :grep, :grep_v, :inject, :partition, :find, :find_all,
          :minmax, :group_by, :take_while, :drop_while
        ])

        case op
        when :chunk
          "enum.chunk { |x| x.is_a?(Numeric) ? x.even? : false }.to_a"
        when :slice_before
          pattern = choice(['2', '"banana"', '/a/'])
          "enum.slice_before(#{pattern}).to_a"
        when :slice_after
          pattern = choice(['2', '"cherry"', '/e/'])
          "enum.slice_after(#{pattern}).to_a"
        when :slice_when
          "enum.slice_when { |i, j| i.is_a?(Numeric) && j.is_a?(Numeric) ? i + 1 != j : false }.to_a"
        when :tally
          "enum.tally"
        when :grep, :grep_v
          pattern = choice(['Numeric', 'String', '/a/', '1..3'])
          "enum.#{op}(#{pattern})"
        when :inject
          "enum.inject(0) { |acc, x| acc + (x.is_a?(Numeric) ? x : 0) }"
        when :partition
          "enum.partition { |x| x.is_a?(Numeric) }"
        when :find, :find_all
          "enum.#{op} { |x| x.is_a?(String) }"
        when :minmax
          "enum.minmax"
        when :group_by
          "enum.group_by { |x| x.class }"
        when :take_while, :drop_while
          "enum.#{op} { |x| x.is_a?(Numeric) }.to_a"
        else
          "enum.to_a"
        end
      end
    end
  end
end
