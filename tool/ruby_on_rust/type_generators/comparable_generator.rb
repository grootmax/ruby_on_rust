# frozen_string_literal: true

require_relative 'base_generator'

module RubyOnRust
  module TypeGenerators
    class ComparableGenerator < BaseGenerator
      def target_name
        'Comparable'
      end

      def generate_snippet
        obj = choice(['CustomComp.new(5)', 'CustomComp.new(0)', 'CustomComp.new(-10)'])
        method_call = generate_method_call

        <<~RUBY
          #{setup_prelude}
          comp = #{obj}
          #{method_call}
        RUBY
      end

      private

      def generate_method_call
        op = choice([
          :between?, :clamp, :less_or_equal, :greater_or_equal,
          :less, :greater, :equal
        ])

        case op
        when :between?
          min = choice(['CustomComp.new(0)', 'CustomComp.new(10)', 'CustomComp.new(-20)'])
          max = choice(['CustomComp.new(10)', 'CustomComp.new(0)', 'CustomComp.new(20)'])
          "comp.between?(#{min}, #{max})"
        when :clamp
          min = choice(['CustomComp.new(0)', 'CustomComp.new(10)', 'nil'])
          max = choice(['CustomComp.new(10)', 'CustomComp.new(20)', 'nil'])
          range = choice([nil, "#{min}..#{max}", "#{min}...#{max}"])
          if range
            "comp.clamp(#{range})"
          else
            args = [min, max].compact.join(', ')
            args.empty? ? "comp.clamp(0..10)" : "comp.clamp(#{args})"
          end
        when :less_or_equal, :greater_or_equal, :less, :greater, :equal
          other = choice(['CustomComp.new(5)', 'CustomComp.new(10)', 'CustomComp.new(-5)', '"str"', 'nil'])
          sym = case op
                when :less_or_equal then '<='
                when :greater_or_equal then '>='
                when :less then '<'
                when :greater then '>'
                when :equal then '=='
                end
          "comp #{sym} #{other}"
        else
          "comp.inspect"
        end
      end
    end
  end
end
