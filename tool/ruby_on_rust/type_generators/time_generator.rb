# frozen_string_literal: true

require_relative 'base_generator'

module RubyOnRust
  module TypeGenerators
    class TimeGenerator < BaseGenerator
      def target_name
        'Time'
      end

      def generate_snippet
        recv = choice(edge_time_exprs)
        method_call = generate_method_call

        <<~RUBY
          #{setup_prelude}
          tm = #{recv}
          #{method_call}
        RUBY
      end

      private

      def generate_method_call
        op = choice([
          :strftime, :add, :sub, :spaceship, :utc, :getlocal,
          :round, :floor, :ceil, :to_i, :to_f, :to_r, :subsec,
          :wday, :yday, :zone, :utc?
        ])

        case op
        when :strftime
          fmt = choice(['"%Y-%m-%d %H:%M:%S"', '"%Y-%m-%d %H:%M:%S.%N %z"', '"%s"', '"%z"', '"%Z"'])
          "tm.strftime(#{fmt})"
        when :add, :sub
          val = choice(['1', '3600', '86400', '0.5', '-100', 'CoercibleInt.new(60)'])
          op == :add ? "tm + #{val}" : "tm - #{val}"
        when :spaceship
          other = choice(edge_time_exprs + ['"str"', '12345'])
          "tm <=> #{other}"
        when :utc, :getlocal
          "tm.#{op}"
        when :round, :floor, :ceil
          ndigits = choice([nil, '0', '2', '6', '9'])
          ndigits ? "tm.#{op}(#{ndigits})" : "tm.#{op}"
        when :to_i, :to_f, :to_r, :subsec, :wday, :yday, :zone, :utc?
          "tm.#{op}"
        else
          "tm.inspect"
        end
      end
    end
  end
end
