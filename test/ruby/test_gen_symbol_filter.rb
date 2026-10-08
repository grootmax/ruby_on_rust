# frozen_string_literal: true

require 'test/unit'
require 'tmpdir'
require 'open3'
require_relative '../../tool/gen_symbol_filter'

class TestGenSymbolFilter < Test::Unit::TestCase
  def setup
    @tmpdir = Dir.mktmpdir("test_gen_symbol_filter")
    @manifest_path = File.join(@tmpdir, "test_symbols.sym")
    File.write(@manifest_path, <<~SYM)
      # Allowed export patterns
      rb_*
      ruby_*

      Init_*
      rust_eh_personality
    SYM
  end

  def teardown
    FileUtils.remove_entry(@tmpdir) if @tmpdir && File.directory?(@tmpdir)
  end

  def test_objcopy_format
    generator = SymbolFilterGenerator.new(@manifest_path, prefix: "", format: "objcopy")
    expected = "--keep-global-symbol=rb_* --keep-global-symbol=ruby_* --keep-global-symbol=Init_*"
    assert_equal expected, generator.generate
  end

  def test_objcopy_format_with_prefix
    generator = SymbolFilterGenerator.new(@manifest_path, prefix: "_", format: "objcopy")
    expected = "--keep-global-symbol=_rb_* --keep-global-symbol=_ruby_* --keep-global-symbol=_Init_* --keep-global-symbol=_rust_eh_personality"
    assert_equal expected, generator.generate
  end

  def test_ld_u_format
    generator = SymbolFilterGenerator.new(@manifest_path, prefix: "", format: "ld-u")
    nm_input = <<~NM
      0000000000000000 T rb_func1
      0000000000000010 T ruby_func2
      0000000000000020 T Init_func3
      0000000000000030 T rust_eh_personality
      0000000000000040 T unexported_helper
      0000000000000050 t static_rb_func
                     U rb_undefined
    NM
    expected = "-u rb_func1 -u ruby_func2 -u Init_func3"
    assert_equal expected, generator.generate(nm_input)
  end

  def test_ld_u_format_with_prefix
    generator = SymbolFilterGenerator.new(@manifest_path, prefix: "_", format: "ld-u")
    nm_input = <<~NM
      0000000000000000 T _rb_func1
      0000000000000010 T _ruby_func2
      0000000000000020 T _Init_func3
      0000000000000030 T _rust_eh_personality
      0000000000000040 T _unexported_helper
    NM
    expected = "-u _rb_func1 -u _ruby_func2 -u _Init_func3 -u _rust_eh_personality"
    assert_equal expected, generator.generate(nm_input)
  end

  def test_symbols_format
    generator = SymbolFilterGenerator.new(@manifest_path, prefix: "", format: "symbols")
    nm_input = <<~NM
      0000000000000000 T rb_func1
      0000000000000010 T ruby_func2
      0000000000000020 T Init_func3
      0000000000000030 T rust_eh_personality
      0000000000000040 T unexported_helper
    NM
    expected = "rb_func1\nruby_func2\nInit_func3"
    assert_equal expected, generator.generate(nm_input)
  end

  def test_symbols_format_with_prefix
    generator = SymbolFilterGenerator.new(@manifest_path, prefix: "_", format: "symbols")
    nm_input = <<~NM
      0000000000000000 T _rb_func1
      0000000000000010 T _ruby_func2
      0000000000000020 T _Init_func3
      0000000000000030 T _rust_eh_personality
      0000000000000040 T _unexported_helper
    NM
    expected = "_rb_func1\n_ruby_func2\n_Init_func3\n_rust_eh_personality"
    assert_equal expected, generator.generate(nm_input)
  end

  def test_cli_execution
    script_path = File.expand_path("../../tool/gen_symbol_filter.rb", __dir__)
    stdout, status = Open3.capture2(script_path, "--format=objcopy", @manifest_path)
    assert status.success?
    expected = "--keep-global-symbol=rb_* --keep-global-symbol=ruby_* --keep-global-symbol=Init_*"
    assert_equal expected, stdout.strip
  end
end
