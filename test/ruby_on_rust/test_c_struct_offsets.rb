# frozen_string_literal: true

require "test/unit"
require "json"
require "tmpdir"
require "fileutils"

class TestCStructOffsets < Test::Unit::TestCase
  def setup
    @root_dir = File.expand_path("../..", __dir__)
    @tool_script = File.join(@root_dir, "tool", "check_c_struct_offsets.rb")
    @reference_file = File.join(@root_dir, "tool", "c_struct_offsets_reference.json")
  end

  def test_tool_script_exists
    assert_path_exist @tool_script, "tool/check_c_struct_offsets.rb should exist"
  end

  def test_reference_file_exists
    assert_path_exist @reference_file, "tool/c_struct_offsets_reference.json should exist"
  end

  def test_reference_file_contains_key_structs
    data = JSON.parse(File.read(@reference_file))
    structs = data["structs"]
    assert structs, "Reference file should contain 'structs' key"

    %w[RString RArray RHash RTypedData rb_io].each do |struct_name|
      assert structs.key?(struct_name), "Reference file should contain benchmark for #{struct_name}"
      assert structs[struct_name]["size"].is_a?(Integer), "#{struct_name} should have a size"
      assert !structs[struct_name]["fields"].empty?, "#{struct_name} should have field offsets"
    end
  end

  def test_c_struct_offsets_match_golden_reference
    out = `ruby #{@tool_script} --check 2>&1`
    assert $?.success?, "C struct layout offset verification failed:\n#{out}"
    assert_includes out, "[OK]", "Expected success output from check_c_struct_offsets.rb"
  end

  def test_detects_layout_mismatch
    Dir.mktmpdir("test_c_struct_mismatch") do |tmpdir|
      fake_ref = File.join(tmpdir, "c_struct_offsets_reference.json")

      ref_data = JSON.parse(File.read(@reference_file))
      ref_data["structs"]["RString"]["fields"]["len"] += 999
      File.write(fake_ref, JSON.pretty_generate(ref_data))

      # Instantiate CStructOffsetChecker directly with custom root dir
      require_relative "../../tool/check_c_struct_offsets"
      checker = CStructOffsetChecker.new(root_dir: @root_dir)
      checker.instance_variable_set(:@reference_path, fake_ref)

      out = nil
      # Capture stderr/stdout
      begin
        orig_stdout = $stdout.dup
        orig_stderr = $stderr.dup
        pipe_r, pipe_w = IO.pipe
        $stdout.reopen(pipe_w)
        $stderr.reopen(pipe_w)

        success = checker.check!

        $stdout.reopen(orig_stdout)
        $stderr.reopen(orig_stderr)
        pipe_w.close
        out = pipe_r.read
      ensure
        $stdout.reopen(orig_stdout) if orig_stdout
        $stderr.reopen(orig_stderr) if orig_stderr
      end

      assert_equal false, success, "check! should return false when layout mismatches"
      assert_includes out, "[ERROR]", "Output should contain error notice"
      assert_match(/RString\.len/, out, "Output should mention mismatched field")
    end
  end
end
