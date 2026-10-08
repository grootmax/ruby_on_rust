# frozen_string_literal: true

require "test/unit"
require_relative "../generate_porting_ledger"

class TestGeneratePortingLedger < Test::Unit::TestCase
  def test_load_valid_statuses_from_config
    config = {
      "valid_statuses" => ["Not Started", "In Progress", "Hardened", "Done"]
    }
    statuses = PortingLedger.load_valid_statuses(config)
    assert_equal(["Not Started", "In Progress", "Hardened", "Done"], statuses)
  end

  def test_load_valid_statuses_fallback_when_missing
    config = {
      "subsystems" => {}
    }
    statuses = PortingLedger.load_valid_statuses(config)
    assert_equal(DEFAULT_VALID_STATUSES, statuses)
    assert_include(statuses, "Hardened")
  end

  def test_load_valid_statuses_fallback_when_empty_array
    config = { "valid_statuses" => [] }
    statuses = PortingLedger.load_valid_statuses(config)
    assert_equal(DEFAULT_VALID_STATUSES, statuses)
  end

  def test_load_valid_statuses_fallback_when_not_array
    config = { "valid_statuses" => "Not Started" }
    statuses = PortingLedger.load_valid_statuses(config)
    assert_equal(DEFAULT_VALID_STATUSES, statuses)
  end

  def test_load_valid_statuses_fallback_when_non_string_elements
    config = { "valid_statuses" => ["Not Started", 123, nil] }
    statuses = PortingLedger.load_valid_statuses(config)
    assert_equal(DEFAULT_VALID_STATUSES, statuses)
  end

  def test_invalid_file_status_warning_and_fallback
    config = {
      "valid_statuses" => ["Not Started", "Ported"],
      "files" => {
        "test.c" => { "status" => "UnknownStatus" }
      }
    }

    warnings = []
    original_warn = method(:warn)
    begin
      entries = nil
      # Capture stderr warning
      stderr_output = capture_output do
        entries = PortingLedger.build_file_entries(config, c_files: ["/path/to/test.c"])
      end
      assert_equal(1, entries.size)
      assert_equal("Not Started", entries.first[:status])
      assert_match(/Invalid status 'UnknownStatus' for test.c, defaulting to 'Not Started'/, stderr_output)
    end
  end

  def test_summary_stats_includes_custom_valid_statuses
    config = {
      "valid_statuses" => ["Not Started", "In Progress", "Hardened"],
      "subsystems" => { "Core" => { "description" => "Core subsystem" } },
      "files" => {
        "a.c" => { "status" => "Not Started", "subsystem" => "Core" },
        "b.c" => { "status" => "Hardened", "subsystem" => "Core" }
      }
    }

    markdown, total_files, total_loc = PortingLedger.generate_ledger_markdown(config, c_files: ["a.c", "b.c"])
    assert_equal(2, total_files)
    assert_match(/\| \*\*Not Started\*\* \| 1 \|/, markdown)
    assert_match(/\| \*\*In Progress\*\* \| 0 \|/, markdown)
    assert_match(/\| \*\*Hardened\*\* \| 1 \|/, markdown)
    refute_match(/\| \*\*Blocked\*\*/, markdown)
  end

  def test_hardened_status_rendering
    config = {
      "files" => {
        "hardened_module.c" => { "status" => "Hardened", "target" => "core_rs::hardened" }
      }
    }

    markdown, total_files, _ = PortingLedger.generate_ledger_markdown(config, c_files: ["hardened_module.c"])
    assert_equal(1, total_files)
    assert_match(/\| `hardened_module.c` \| \d+ \| Hardened \| `core_rs::hardened` \|/, markdown)
    assert_match(/\| \*\*Hardened\*\* \| 1 \|/, markdown)
  end

  private

  def capture_output
    old_stderr = $stderr
    $stderr = StringIO.new
    yield
    $stderr.string
  ensure
    $stderr = old_stderr
  end
end
