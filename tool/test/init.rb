# This file includes the settings for "make test-all" and "make test-tool".
# Note that this file is loaded not only by test/runner.rb but also by tool/lib/test/unit/parallel.rb.

# Prevent test-all from using bundled gems
["GEM_HOME", "GEM_PATH"].each do |gem_env|
  # Preserve the gem environment prepared by tool/runruby.rb for test-tool, which uses bundled gems.
  ENV["BUNDLED_#{gem_env}"] = ENV[gem_env]

  ENV[gem_env] = "".freeze
end
ENV["GEM_SKIP"] = "".freeze

ENV.delete("RUBY_CODESIGN")

Warning[:experimental] = false
# The tests assert the output of the ruby processes they spawn verbatim, so the
# children need this too.  The last -W wins, so the switch has to come after
# whatever RUBYOPT already carries, but before the bare "-" that common.mk puts
# there, which ends the option scan.
rubyopt = ENV["RUBYOPT"].to_s.split - ["-W:no-experimental"]
rubyopt.insert(rubyopt.index("-") || rubyopt.size, "-W:no-experimental")
ENV["RUBYOPT"] = rubyopt.join(" ")

$LOAD_PATH.unshift File.expand_path("../lib", __dir__)

require 'test/unit'

module Test
  module Unit
    module RubyOnRustOption
      RUBY_ON_RUST_TEST_PATTERN = %r{(?:\A|/)test/ruby_on_rust(?:/|\z)|\Aruby_on_rust(?:/|\z)}

      def setup_options(parser, options)
        super
        parser.separator "ruby_on_rust options:"
        parser.on '--ruby-on-rust', 'Run only Ruby on Rust tests' do
          options[:ruby_on_rust] = true
        end
        parser.on '--exclude-ruby-on-rust', 'Exclude Ruby on Rust tests' do
          options[:exclude_ruby_on_rust] = true
        end
      end

      def non_options(files, options)
        return false if !super
        if options.delete(:ruby_on_rust)
          files.select! { |f| RUBY_ON_RUST_TEST_PATTERN.match?(f) }
        end
        if options.delete(:exclude_ruby_on_rust)
          files.reject! { |f| RUBY_ON_RUST_TEST_PATTERN.match?(f) }
        end
        true
      end
    end

    Runner.prepend RubyOnRustOption
  end
end

require "profile_test_all" if ENV.key?('RUBY_TEST_ALL_PROFILE')
require "tracepointchecker"
require "zombie_hunter"
require "iseq_loader_checker"
require "gc_checker"
require_relative "../test-coverage.rb" if ENV.key?('COVERAGE')
