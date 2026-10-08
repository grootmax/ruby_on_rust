# frozen_string_literal: true

require_relative 'base_generator'

module RubyOnRust
  module TypeGenerators
    class StringGenerator < BaseGenerator
      def target_name
        'String'
      end

      def generate_snippet
        recv = choice(edge_string_exprs)
        method_call = generate_method_call

        <<~RUBY
          #{setup_prelude}
          str = #{recv}
          #{method_call}
        RUBY
      end

      private

      def generate_method_call
        op = choice([
          :slice, :byteslice, :concat, :gsub, :sub, :tr, :split, :encode,
          :index, :rindex, :center, :ljust, :rjust, :strip, :unpack1,
          :scrub, :valid_encoding?, :force_encoding, :chop, :chomp, :casecmp,
          :plus, :multiply, :index_op, :index_set_op
        ])

        case op
        when :slice, :byteslice, :index_op
          arg1 = choice(['0', '1', '-1', '100', '-100', '1..3', '0...5', '10..100', 'CoercibleInt.new(1)'])
          arg2 = choice([nil, '1', '5', '0', '-1', '100'])
          args = [arg1, arg2].compact.join(', ')
          "str.#{op == :index_op ? "[]" : op}(#{args})"
        when :index_set_op
          idx = choice(['0', '1', '-1', '100', '1..3'])
          val = choice(edge_string_exprs)
          "str[#{idx}] = #{val}"
        when :concat, :plus
          val = choice(edge_string_exprs)
          op == :plus ? "str + #{val}" : "str.concat(#{val})"
        when :multiply
          n = choice(['0', '1', '5', '100', '-1', 'CoercibleInt.new(2)'])
          "str * #{n}"
        when :gsub, :sub
          pattern = choice(['"l"', '"o"', '/[aeiou]/', '/\d+/', '"nonexistent"'])
          replacement = choice(['"X"', '"\\1_replacement"', '""'])
          "str.#{op}(#{pattern}, #{replacement})"
        when :tr
          from = choice(['"a-z"', '"A-Z"', '"aeiou"', '"\x80-\xFF"'])
          to = choice(['"A-Z"', '"a-z"', '"*"', '""'])
          "str.tr(#{from}, #{to})"
        when :split
          pattern = choice(['nil', '" "', '","', '"\n"', '/\s+/', '""'])
          limit = choice([nil, '0', '1', '2', '-1'])
          args = [pattern, limit].compact.join(', ')
          "str.split(#{args})"
        when :encode
          enc = choice(['"UTF-8"', '"ASCII-8BIT"', '"UTF-16LE"', '"ISO-8859-1"', '"EUC-JP"'])
          "str.encode(#{enc})"
        when :index, :rindex
          substr = choice(['"h"', '"l"', '"xyz"', '"\n"'])
          pos = choice([nil, '0', '2', '-1'])
          args = [substr, pos].compact.join(', ')
          "str.#{op}(#{args})"
        when :center, :ljust, :rjust
          width = choice(['0', '5', '20', '100', '-5'])
          pad = choice([nil, '" "', '"*"', '"-="'])
          args = [width, pad].compact.join(', ')
          "str.#{op}(#{args})"
        when :strip, :chop, :chomp, :scrub, :valid_encoding?
          "str.#{op}"
        when :force_encoding
          enc = choice(['Encoding::UTF_8', 'Encoding::ASCII_8BIT', 'Encoding::UTF_16LE', 'Encoding::ISO_8859_1'])
          "str.force_encoding(#{enc})"
        when :casecmp
          other = choice(edge_string_exprs)
          "str.casecmp(#{other})"
        when :unpack1
          fmt = choice(['"A*"', '"a*"', '"H*"', '"h*"', '"C*"', '"c*"', '"m0"', '"u*"'])
          "str.unpack1(#{fmt})"
        else
          "str.inspect"
        end
      end
    end
  end
end
