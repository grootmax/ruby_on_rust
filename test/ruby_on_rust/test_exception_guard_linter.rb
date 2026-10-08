require 'minitest/autorun'

class TestRubyGuardMacrosAndLinter < Minitest::Test
  def test_macro_and_linter_build_and_pass
    cargo_bin = 'cargo'
    linter_test_status = system("cd #{File.expand_path('../..', __dir__)} && #{cargo_bin} test -p ruby_guard_macros -p ruby_guard_linter > /dev/null 2>&1")
    assert linter_test_status, "cargo test -p ruby_guard_macros -p ruby_guard_linter failed"
  end
end
