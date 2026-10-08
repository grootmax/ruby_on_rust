# frozen_string_literal: true

require_relative 'base_generator'

module RubyOnRust
  module TypeGenerators
    class HashGenerator < BaseGenerator
      def target_name
        'Hash'
      end

      def generate_snippet
        recv = choice(edge_hash_exprs)
        method_call = generate_method_call

        <<~RUBY
          #{setup_prelude}
          hsh = #{recv}
          #{method_call}
        RUBY
      end

      private

      def generate_method_call
        op = choice([
          :merge, :transform_values, :transform_keys, :dig, :compact,
          :reject, :select, :invert, :slice, :except, :fetch, :assoc,
          :rassoc, :flatten, :index_op, :index_set_op
        ])

        case op
        when :index_op
          key = choice([':a', '"str"', '123', ':nonexistent', 'nil'])
          "hsh[#{key}]"
        when :index_set_op
          key = choice([':new_k', '"k"', '100'])
          val = choice(['123', '"v"', 'nil', '[1, 2]'])
          "hsh[#{key}] = #{val}"
        when :merge
          other = choice(edge_hash_exprs)
          "hsh.merge(#{other})"
        when :transform_values
          "hsh.transform_values { |v| v.nil? ? 'nil_val' : v.to_s }"
        when :transform_keys
          "hsh.transform_keys { |k| k.to_s.upcase }"
        when :dig
          keys = choice([':a', '"str", "nested"', '123, 0'])
          "hsh.dig(#{keys})"
        when :compact, :invert
          "hsh.#{op}"
        when :reject, :select
          "hsh.#{op} { |k, v| v.nil? || k == :a }"
        when :slice
          keys = choice([':a, :b', '"str", 123', ':nonexistent'])
          "hsh.slice(#{keys})"
        when :except
          keys = choice([':a', '"str"', ':nonexistent'])
          "hsh.except(#{keys})"
        when :fetch
          key = choice([':a', ':missing'])
          default = choice([nil, '"default_val"', '-> { "block_val" }'])
          if default == '-> { "block_val" }'
            "hsh.fetch(#{key}) { |k| \"missing_\#{k}\" }"
          elsif default
            "hsh.fetch(#{key}, #{default})"
          else
            "hsh.fetch(#{key})"
          end
        when :assoc, :rassoc
          arg = choice([':a', '"str"', '1', 'nil'])
          "hsh.#{op}(#{arg})"
        when :flatten
          level = choice([nil, '1', '2', '0', '-1'])
          level ? "hsh.flatten(#{level})" : "hsh.flatten"
        else
          "hsh.inspect"
        end
      end
    end
  end
end
