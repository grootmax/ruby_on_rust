# frozen_string_literal: true

require 'test/unit'
require 'envutil'

class TestGeneratePortingLedger < Test::Unit::TestCase
  def test_porting_ledger_up_to_date
    script = File.expand_path('../generate_porting_ledger.rb', __dir__)
    stdout, stderr, status = EnvUtil.invoke_ruby([script, '--check'], '', true, true)
    assert_predicate status, :success?, "PORTING.md is out of date. Run 'make fix-porting-ledger' or 'ruby tool/generate_porting_ledger.rb' to regenerate.\nstdout: #{stdout}\nstderr: #{stderr}"
  end
end
