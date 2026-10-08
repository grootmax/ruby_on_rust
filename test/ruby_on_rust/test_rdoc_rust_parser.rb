# frozen_string_literal: false
require 'test/unit'
require 'fileutils'
require 'tmpdir'
require 'rbconfig'
require 'rdoc/rdoc'

class TestRDocRustParser < Test::Unit::TestCase
  def setup
    @tmpdir = Dir.mktmpdir('test_rdoc_rust_')
    @srcdir = File.join(@tmpdir, 'src')
    @op_dir = File.join(@tmpdir, 'rdoc_out')
    FileUtils.mkdir_p(@srcdir)
  end

  def teardown
    FileUtils.rm_rf(@tmpdir)
  end

  def test_rust_doc_extraction_and_rdoc_generation
    rs_code = <<~RUST
      //! Document-module: TestRustModule
      //!
      //! Inner module documentation for TestRustModule.

      /// Document-class: TestRustClass
      ///
      /// Outer class documentation for TestRustClass.
      pub struct TestRustClass;

      /// Document-method: TestRustClass#instance_func
      ///
      /// call-seq:
      ///   obj.instance_func -> String
      ///
      /// Instance function documentation.
      #[no_mangle]
      pub unsafe extern "C" fn rust_instance_func() {}

      /// Document-method: TestRustClass.singleton_func
      ///
      /// Singleton function documentation.
      pub fn rust_singleton_func() {}

      /// Implicit global function documentation.
      #[unsafe(no_mangle)]
      pub unsafe extern "C" fn rust_implicit_global_func() {}
    RUST

    File.write(File.join(@srcdir, 'test.rs'), rs_code)

    tool_rdoc_srcdir = File.expand_path('../../tool/rdoc-srcdir', __dir__)

    # Run tool/rdoc-srcdir on @srcdir
    cmd = [
      RbConfig.ruby,
      tool_rdoc_srcdir,
      '--ri',
      '--op', @op_dir,
      @srcdir
    ]

    pid = Process.spawn(*cmd, chdir: @srcdir)
    _, status = Process.wait2(pid)
    assert_predicate status, :success?, "tool/rdoc-srcdir process failed"

    # Verify generated RI files exist
    assert_path_exist File.join(@op_dir, 'TestRustClass', 'cdesc-TestRustClass.ri')
    assert_path_exist File.join(@op_dir, 'TestRustClass', 'instance_func-i.ri')
    assert_path_exist File.join(@op_dir, 'TestRustClass', 'singleton_func-c.ri')
    assert_path_exist File.join(@op_dir, 'TestRustModule', 'cdesc-TestRustModule.ri')
    assert_path_exist File.join(@op_dir, 'Kernel', 'rust_implicit_global_func-i.ri')

    # Verify content in generated RI files
    content_inst = File.read(File.join(@op_dir, 'TestRustClass', 'instance_func-i.ri'))
    assert_match(/Instance function documentation/, content_inst)

    content_sing = File.read(File.join(@op_dir, 'TestRustClass', 'singleton_func-c.ri'))
    assert_match(/Singleton function documentation/, content_sing)

    content_global = File.read(File.join(@op_dir, 'Kernel', 'rust_implicit_global_func-i.ri'))
    assert_match(/Implicit global function documentation/, content_global)
  end
end
