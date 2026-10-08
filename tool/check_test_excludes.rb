#!/usr/bin/env ruby
# frozen_string_literal: true

# CI linter to verify that no test exclusion directories/files or MSpec exclusion calls exist.

repo_root = File.expand_path("..", __dir__)
errors = []

# 1. Check test/ for any .excludes* directory or file
test_dir = File.join(repo_root, "test")
if Dir.exist?(test_dir)
  Dir.glob(File.join(test_dir, ".excludes*")).each do |entry|
    rel_path = entry.sub("#{repo_root}/", "")
    errors << "Forbidden test exclusion directory/file found: #{rel_path}"
  end
end

# 2. Check spec/ for any MSpec exclusion calls
spec_dir = File.join(repo_root, "spec")
if Dir.exist?(spec_dir)
  Dir.glob(File.join(spec_dir, "**", "*.mspec")).each do |mspec_file|
    File.readlines(mspec_file).each_with_index do |line, idx|
      if line =~ /MSpec\.register\s*\(?\s*:exclude\b/
        rel_path = mspec_file.sub("#{repo_root}/", "")
        errors << "Forbidden MSpec exclusion directive in #{rel_path}:#{idx + 1}: #{line.strip}"
      end
    end
  end
end

if errors.any?
  puts "ERROR: CI test exclusion linter failed with #{errors.size} error(s):"
  errors.each do |err|
    puts "  - #{err}"
  end
  exit 1
else
  puts "SUCCESS: CI test exclusion linter passed (no test exclusion files or MSpec exclusion directives found)."
  exit 0
end
