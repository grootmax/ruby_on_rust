# frozen_string_literal: true

require "test/unit"
require "tmpdir"
require "yaml"
require "open3"
require "date"

class TestGeneratePortingLedger < Test::Unit::TestCase
  REPO_ROOT = File.expand_path("../..", __dir__)
  SCRIPT_PATH = File.join(REPO_ROOT, "tool", "generate_porting_ledger.rb")

  def test_ledger_check_passes_on_current_repo
    stdout, stderr, status = Open3.capture3("ruby", SCRIPT_PATH, "--check", chdir: REPO_ROOT)
    assert_equal 0, status.exitstatus, "Expected --check to succeed, stderr: #{stderr}"
    assert_match(/PORTING\.md is up to date/, stdout)
  end

  def test_schema_validation_rejects_invalid_ported_date
    Dir.mktmpdir do |dir|
      status_path = File.join(dir, "tool", "porting_status.yml")
      FileUtils.mkdir_p(File.dirname(status_path))

      status_data = {
        "subsystems" => { "Core" => { "description" => "Core" } },
        "files" => {
          "foo.c" => {
            "status" => "Ported",
            "ported_date" => "not-a-valid-date",
            "subsystem" => "Core"
          }
        }
      }
      File.write(status_path, YAML.dump(status_data))
      File.write(File.join(dir, "foo.c"), "int main() { return 0; }\n")
      File.write(File.join(dir, "PORTING.md"), "# Old content\n")
      FileUtils.cp(SCRIPT_PATH, File.join(dir, "tool", "generate_porting_ledger.rb"))

      _stdout, stderr, status = Open3.capture3("ruby", "tool/generate_porting_ledger.rb", chdir: dir)
      refute_equal 0, status.exitstatus
      assert_match(/Invalid ported_date/i, stderr)
    end
  end

  def test_schema_validation_rejects_invalid_subsystems_schema
    Dir.mktmpdir do |dir|
      status_path = File.join(dir, "tool", "porting_status.yml")
      FileUtils.mkdir_p(File.dirname(status_path))

      status_data = {
        "subsystems" => "invalid_string_instead_of_hash",
        "files" => {}
      }
      File.write(status_path, YAML.dump(status_data))
      FileUtils.cp(SCRIPT_PATH, File.join(dir, "tool", "generate_porting_ledger.rb"))

      _stdout, stderr, status = Open3.capture3("ruby", "tool/generate_porting_ledger.rb", chdir: dir)
      refute_equal 0, status.exitstatus
      assert_match(/Invalid schema/i, stderr)
    end
  end

  def test_soak_gate_calculation
    Dir.mktmpdir do |dir|
      status_path = File.join(dir, "tool", "porting_status.yml")
      FileUtils.mkdir_p(File.dirname(status_path))

      date_passed = (Date.today - 30).strftime("%Y-%m-%d")
      date_in_soak = (Date.today - 10).strftime("%Y-%m-%d")

      status_data = {
        "subsystems" => { "Core" => { "description" => "Core Data" } },
        "files" => {
          "a.c" => { "status" => "Ported", "ported_date" => date_passed, "subsystem" => "Core" },
          "b.c" => { "status" => "Ported", "ported_date" => date_in_soak, "subsystem" => "Core" },
          "c.c" => { "status" => "Ported", "subsystem" => "Core" }
        }
      }
      File.write(status_path, YAML.dump(status_data))
      File.write(File.join(dir, "a.c"), "int a;\n")
      File.write(File.join(dir, "b.c"), "int b;\n")
      File.write(File.join(dir, "c.c"), "int c;\n")
      FileUtils.cp(SCRIPT_PATH, File.join(dir, "tool", "generate_porting_ledger.rb"))

      stdout, stderr, status = Open3.capture3("ruby", "tool/generate_porting_ledger.rb", chdir: dir)
      assert_equal 0, status.exitstatus, "Expected generation to succeed, stderr: #{stderr}"

      porting_md = File.read(File.join(dir, "PORTING.md"))
      assert_match(/Passed \(30 days\)/, porting_md)
      assert_match(/In Soak \(10\/28 days\)/, porting_md)
      assert_match(/Pending Date/, porting_md)
    end
  end
end
