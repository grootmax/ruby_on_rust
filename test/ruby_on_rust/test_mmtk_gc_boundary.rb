# frozen_string_literal: true

require "test/unit"
require "core_assertions"

module RubyOnRust
  class TestMMTkGCBoundary < Test::Unit::TestCase
    include Test::Unit::CoreAssertions

    def setup
      super
      @using_mmtk = GC.respond_to?(:config) && GC.config[:implementation] == "mmtk"
    end

    def test_gc_boundary_lifecycle
      # Ensure object allocation and garbage collection across C/Rust boundary execute cleanly
      objects = []
      1_000.times { |i| objects << "string_candidate_#{i}" }

      GC.start
      assert_operator objects.compact.size, :>, 0

      # Force stress collection cycle if supported
      if @using_mmtk
        assert_nothing_raised do
          GC.stress = true
          10.times { Array.new(100) { Object.new } }
          GC.stress = false
        end
      end
    end

    def test_gc_boundary_statistics
      if @using_mmtk
        config = GC.config
        assert_kind_of Hash, config
        assert_equal "mmtk", config[:implementation]
        assert_kind_of String, config[:mmtk_plan]
        assert_kind_of String, config[:mmtk_heap_mode]
      else
        assert_kind_of Integer, GC.count
      end
    end

    def test_malformed_ffi_env_handling
      return unless @using_mmtk

      # Verify defensive validation of invalid FFI initialization parameters
      exit_code = assert_in_out_err(
        [{ "MMTK_HEAP_MIN" => "100MiB", "MMTK_HEAP_MAX" => "10MiB" }, "--"],
        "",
        [],
        ["[FATAL] MMTK_HEAP_MIN(104857600) >= MMTK_HEAP_MAX(10485760)"]
      )
      assert_equal 1, exit_code.exitstatus
    end

    def test_invalid_plan_ffi_handling
      return unless @using_mmtk

      exit_code = assert_in_out_err(
        [{ "MMTK_PLAN" => "InvalidPlanName" }, "--"],
        "",
        [],
        ["[FATAL] Invalid MMTK_PLAN InvalidPlanName"]
      )
      assert_equal 1, exit_code.exitstatus
    end
  end
end
