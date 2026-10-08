# frozen_string_literal: true

require_relative 'base_generator'
require_relative 'string_generator'
require_relative 'array_generator'
require_relative 'hash_generator'
require_relative 'integer_generator'
require_relative 'float_generator'
require_relative 'range_generator'
require_relative 'struct_generator'
require_relative 'time_generator'
require_relative 'comparable_generator'
require_relative 'enumerable_generator'

module RubyOnRust
  module TypeGenerators
    class GeneratorFactory
      GENERATORS = {
        'string' => StringGenerator,
        'array' => ArrayGenerator,
        'hash' => HashGenerator,
        'integer' => IntegerGenerator,
        'float' => FloatGenerator,
        'range' => RangeGenerator,
        'struct' => StructGenerator,
        'time' => TimeGenerator,
        'comparable' => ComparableGenerator,
        'enumerable' => EnumerableGenerator
      }.freeze

      def self.all_target_names
        GENERATORS.keys.map(&:capitalize)
      end

      def self.create(target, random = Random.new)
        target_str = target.to_s.downcase.strip
        if target_str == 'all' || target_str.empty?
          GENERATORS.values.map { |klass| klass.new(random) }
        elsif GENERATORS.key?(target_str)
          [GENERATORS[target_str].new(random)]
        else
          raise ArgumentError, "Unknown target type '#{target}'. Supported types: #{all_target_names.join(', ')}, All"
        end
      end
    end
  end
end
