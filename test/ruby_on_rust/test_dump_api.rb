# frozen_string_literal: true

require "minitest/autorun"
require "json"
require "open3"
require "fileutils"
require "tmpdir"

class TestCoreApiDumper < Minitest::Test
  RUBY_BIN = RbConfig.ruby
  DUMP_API_SCRIPT = File.expand_path("../../tool/dump_api.rb", __dir__)
  LOCKFILE = File.expand_path("../../spec/core_api_lock.json", __dir__)
  ALLOWLIST = File.expand_path("../../tool/api_allowlist.yml", __dir__)

  def test_dump_api_script_exists
    assert File.exist?(DUMP_API_SCRIPT), "tool/dump_api.rb must exist"
    assert File.exist?(LOCKFILE), "spec/core_api_lock.json must exist"
    assert File.exist?(ALLOWLIST), "tool/api_allowlist.yml must exist"
  end

  def test_dump_api_generates_canonical_json
    stdout, stderr, status = Open3.capture3(RUBY_BIN, DUMP_API_SCRIPT, "--internal-dump")
    assert status.success?, "Internal dump failed: #{stderr}"

    data = JSON.parse(stdout)
    assert data.key?("String"), "Dump must include String"
    assert data.key?("Array"), "Dump must include Array"
    assert data.key?("Kernel"), "Dump must include Kernel"

    string_data = data["String"]
    assert_equal "class", string_data["type"]
    assert_kind_of Array, string_data["ancestors"]
    assert_kind_of Array, string_data["constants"]
    assert_kind_of Array, string_data["instance_methods"]
    assert_kind_of Array, string_data["singleton_methods"]

    # Verify method structure
    slice_m = string_data["instance_methods"].find { |m| m["name"] == "slice" }
    refute_nil slice_m, "String must have #slice"
    assert_equal "public", slice_m["visibility"]
    assert_kind_of Integer, slice_m["arity"]
    assert_kind_of Array, slice_m["parameters"]
  end

  def test_check_mode_passes_against_baseline
    stdout, stderr, status = Open3.capture3(
      RUBY_BIN, DUMP_API_SCRIPT,
      "--check",
      "--lockfile", LOCKFILE,
      "--allowlist", ALLOWLIST
    )
    assert status.success?, "API check failed against baseline lockfile:\nSTDOUT:\n#{stdout}\nSTDERR:\n#{stderr}"
    assert_includes stdout, "API Lock Check: OK"
  end

  def test_check_mode_fails_on_unauthorized_divergence
    Dir.mktmpdir do |dir|
      temp_lock = File.join(dir, "temp_lock.json")
      lock_data = JSON.parse(File.read(LOCKFILE))

      # Introduce unauthorized changes: modify arity of String#slice
      if lock_data["String"] && lock_data["String"]["instance_methods"]
        m = lock_data["String"]["instance_methods"].find { |item| item["name"] == "slice" }
        m["arity"] = 999 if m
      end

      File.write(temp_lock, JSON.pretty_generate(lock_data))

      stdout, stderr, status = Open3.capture3(
        RUBY_BIN, DUMP_API_SCRIPT,
        "--check",
        "--lockfile", temp_lock,
        "--allowlist", ALLOWLIST
      )

      refute status.success?, "API check should fail when unauthorized differences exist"
      assert_includes stderr, "API Lock Divergence Detected!"
      assert_includes stderr, "String#slice"
      assert_includes stderr, "arity:"
    end
  end

  def test_check_mode_respects_allowlist
    Dir.mktmpdir do |dir|
      temp_lock = File.join(dir, "temp_lock.json")
      temp_allow = File.join(dir, "temp_allowlist.yml")

      lock_data = JSON.parse(File.read(LOCKFILE))

      # Modify String#slice
      if lock_data["String"] && lock_data["String"]["instance_methods"]
        m = lock_data["String"]["instance_methods"].find { |item| item["name"] == "slice" }
        m["arity"] = 999 if m
      end

      File.write(temp_lock, JSON.pretty_generate(lock_data))

      # Allow changes to String#slice
      allowlist_yaml = <<~YAML
        methods:
          changed:
            - "String#slice"
      YAML
      File.write(temp_allow, allowlist_yaml)

      stdout, stderr, status = Open3.capture3(
        RUBY_BIN, DUMP_API_SCRIPT,
        "--check",
        "--lockfile", temp_lock,
        "--allowlist", temp_allow
      )

      assert status.success?, "API check should pass when differences are in allowlist:\nSTDOUT:\n#{stdout}\nSTDERR:\n#{stderr}"
      assert_includes stdout, "1 allowed difference"
    end
  end
end
