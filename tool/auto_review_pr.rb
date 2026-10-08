#!/usr/bin/env ruby
# frozen_string_literal: true

require 'json'
require 'net/http'
require 'uri'
require_relative './sync_default_gems'

class GitHubAPIClient
  def initialize(token)
    @token = token
  end

  def get(path)
    response = Net::HTTP.get_response(URI("https://api.github.com#{path}"), {
      'Authorization' => "token #{@token}",
      'Accept' => 'application/vnd.github.v3+json',
    }).tap(&:value)
    JSON.parse(response.body, symbolize_names: true)
  end

  def post(path, body = {})
    body = JSON.dump(body)
    response = Net::HTTP.post(URI("https://api.github.com#{path}"), body, {
      'Authorization' => "token #{@token}",
      'Accept' => 'application/vnd.github.v3+json',
      'Content-Type' => 'application/json',
    }).tap(&:value)
    JSON.parse(response.body, symbolize_names: true)
  end
end

class AutoReviewPR
  REPO = 'ruby/ruby'

  COMMENT_USER = 'github-actions[bot]'

  UPSTREAM_COMMENT_PREFIX = 'The following files are maintained in the following upstream repositories:'
  UPSTREAM_COMMENT_SUFFIX = 'Please file a pull request to the above instead. Thank you!'

  REDMINE_TICKET_PATTERN = /\[(Bug|Feature|Misc)\s*#(\d+)\]/
  REDMINE_COMMENT_PREFIX = 'This pull request references the following Redmine tickets:'

  FORK_COMMENT_PREFIX = 'It looks like this pull request was filed from a branch in ruby/ruby.'
  FORK_COMMENT_BODY = <<~COMMENT
    #{FORK_COMMENT_PREFIX}

    Since ruby/ruby is bi-directionally mirrored with the official git repository at git.ruby-lang.org, \
    having topic branches in ruby/ruby makes it harder to manage the mirror.

    Could you please close this pull request and re-file it from a branch in your personal fork instead? \
    You can fork https://github.com/ruby/ruby, push your branch there, and open a new pull request from it.

    Thank you for your contribution!
  COMMENT

  RULE_16_COMMENT_PREFIX = 'This pull request appears to modify C-to-Rust porting files or PORTING.md, but is missing required Rule 16 documentation.'

  RULE_16_SECTIONS = [
    { title: 'Ported Functions/Files', pattern: /^#+\s*(?:\d+\.\s*)?Ported\s+Functions(?:\s*[\/&]\s*Files|\s+and\s+Files)?/i },
    { title: 'Exported Symbols Kept', pattern: /^#+\s*(?:\d+\.\s*)?Exported\s+Symbols(?:\s+Kept)?/i },
    { title: 'Test Run Results', pattern: /^#+\s*(?:\d+\.\s*)?Test\s+Run\s+Results/i },
    { title: 'Benchmark Numbers', pattern: /^#+\s*(?:\d+\.\s*)?Benchmark\s+Numbers/i },
    { title: 'Unsafe Blocks & Safety Rationales', pattern: /^#+\s*(?:\d+\.\s*)?Unsafe\s+Blocks(?:\s*(?:&|and)\s*Safety\s*Rationales)?/i },
  ].freeze

  def initialize(client)
    @client = client
  end

  def review(pr_number)
    existing_comments = fetch_existing_comments(pr_number)
    pr = @client.get("/repos/#{REPO}/pulls/#{pr_number}")
    review_non_fork_branch(pr_number, pr, existing_comments)
    review_upstream_repos(pr_number, existing_comments)
    review_redmine_links(pr_number, pr, existing_comments)
    review_rule_16_compliance(pr_number, pr, existing_comments)
  end

  def check_missing_rule_16_sections(body)
    return RULE_16_SECTIONS.map { |s| s[:title] } if body.nil? || body.to_s.strip.empty?

    sections_content = {}
    current_section = nil

    body.to_s.each_line do |line|
      if line =~ /^\s*#+\s*(.*)$/
        matching = RULE_16_SECTIONS.find { |s| line =~ s[:pattern] }
        if matching
          current_section = matching[:title]
          sections_content[current_section] ||= +''
        else
          current_section = nil
        end
      elsif current_section
        sections_content[current_section] << line
      end
    end

    missing = []
    RULE_16_SECTIONS.each do |section|
      title = section[:title]
      if !sections_content.key?(title)
        missing << title
      else
        cleaned = sections_content[title].gsub(/<!--.*?-->/m, '').strip
        missing << title if cleaned.empty?
      end
    end

    missing
  end

  def porting_pr?(pr_number, changed_files = nil)
    changed_files ||= fetch_changed_files(pr_number)
    porting_files = porting_status_c_files
    changed_files.any? do |file|
      file == 'PORTING.md' ||
        file == 'tool/porting_status.yml' ||
        file.start_with?('core_rs/') ||
        file.end_with?('.rs') ||
        porting_files.include?(file)
    end
  end

  def format_rule_16_comment(missing_sections)
    comment = +"#{RULE_16_COMMENT_PREFIX}\n\n"
    comment << "According to Conversion Rule 16, pull requests modifying C-to-Rust porting files must include details for all required sections. "
    comment << "The following required sections are missing or incomplete in your PR description:\n\n"
    missing_sections.each do |section|
      comment << "* #{section}\n"
    end
    comment << "\nPlease update the pull request description using `.github/PULL_REQUEST_TEMPLATE.md` to include these details. Thank you!\n"
    comment
  end

  private

  def fetch_existing_comments(pr_number)
    comments = @client.get("/repos/#{REPO}/issues/#{pr_number}/comments")
    comments.map { |c| [c.fetch(:user).fetch(:login), c.fetch(:body)] }
  end

  def fetch_changed_files(pr_number)
    files = @client.get("/repos/#{REPO}/pulls/#{pr_number}/files")
    files.map { |f| f[:filename] || f['filename'] || f.fetch(:filename) }
  end

  def porting_status_c_files
    @porting_status_c_files ||= begin
      yaml_path = File.expand_path('porting_status.yml', __dir__)
      if File.exist?(yaml_path)
        require 'yaml'
        data = YAML.load_file(yaml_path)
        (data['files'] || {}).keys
      else
        []
      end
    rescue
      []
    end
  end

  def already_commented?(existing_comments, prefix)
    existing_comments.any? { |user, comment| user == COMMENT_USER && comment.start_with?(prefix) }
  end

  def post_comment(pr_number, comment)
    result = @client.post("/repos/#{REPO}/issues/#{pr_number}/comments", { body: comment })
    puts "Success: #{JSON.pretty_generate(result)}"
  end

  # Check Rule 16 compliance for C-to-Rust porting PRs
  def review_rule_16_compliance(pr_number, pr, existing_comments)
    if already_commented?(existing_comments, RULE_16_COMMENT_PREFIX)
      puts "Skipped: The PR ##{pr_number} already has a Rule 16 compliance comment."
      return
    end

    unless porting_pr?(pr_number)
      puts "Skipped: The PR ##{pr_number} does not modify C-to-Rust porting files or PORTING.md."
      return
    end

    missing_sections = check_missing_rule_16_sections(pr[:body])
    if missing_sections.empty?
      puts "Skipped: The PR ##{pr_number} complies with Rule 16 requirements."
      return
    end

    post_comment(pr_number, format_rule_16_comment(missing_sections))
  end

  # Suggest re-filing from a fork if the PR branch is in ruby/ruby itself
  def review_non_fork_branch(pr_number, pr, existing_comments)
    if already_commented?(existing_comments, FORK_COMMENT_PREFIX)
      puts "Skipped: The PR ##{pr_number} already has a fork branch comment."
      return
    end

    head_repo = pr.dig(:head, :repo, :full_name)
    if head_repo != REPO
      puts "Skipped: The PR ##{pr_number} is already from a fork (#{head_repo})."
      return
    end

    author = pr.dig(:user, :login)
    if author == 'dependabot[bot]'
      puts "Skipped: The PR ##{pr_number} is from dependabot."
      return
    end

    post_comment(pr_number, FORK_COMMENT_BODY)
  end

  # Suggest filing PRs to upstream repositories for files that have one
  def review_upstream_repos(pr_number, existing_comments)
    if already_commented?(existing_comments, UPSTREAM_COMMENT_PREFIX)
      puts "Skipped: The PR ##{pr_number} already has an upstream repos comment."
      return
    end

    changed_files = fetch_changed_files(pr_number)

    upstream_repos = SyncDefaultGems::Repository.group(changed_files)
    upstream_repos.delete(nil)
    # Onigmo fixes land in ruby/ruby directly since the two copies have diverged.
    upstream_repos.delete('k-takata/Onigmo')
    upstream_repos.delete('ruby/prism') if changed_files.include?('prism_compile.c')
    if upstream_repos.empty?
      puts "Skipped: The PR ##{pr_number} doesn't have upstream repositories."
      return
    end

    post_comment(pr_number, format_upstream_comment(upstream_repos))
  end

  def review_redmine_links(pr_number, pr, existing_comments)
    if already_commented?(existing_comments, REDMINE_COMMENT_PREFIX)
      puts "Skipped: The PR ##{pr_number} already has a Redmine links comment."
      return
    end

    text = "#{pr[:title]}\n#{pr[:body]}"

    tickets = text.scan(REDMINE_TICKET_PATTERN).uniq
    tickets.reject! { |_, number| text.include?("https://bugs.ruby-lang.org/issues/#{number}") }
    if tickets.empty?
      puts "Skipped: The PR ##{pr_number} doesn't reference any Redmine tickets."
      return
    end

    post_comment(pr_number, format_redmine_comment(tickets))
  end

  def format_redmine_comment(tickets)
    comment = +"#{REDMINE_COMMENT_PREFIX}\n\n"
    tickets.each do |type, number|
      comment << "* [#{type} ##{number}](https://bugs.ruby-lang.org/issues/#{number})\n"
    end
    comment
  end

  def format_upstream_comment(upstream_repos)
    comment = +''
    comment << "#{UPSTREAM_COMMENT_PREFIX}\n\n"

    upstream_repos.each do |upstream_repo, files|
      comment << "* https://github.com/#{upstream_repo}\n"
      files.each do |file|
        comment << "    * #{file}\n"
      end
    end

    comment << "\n#{UPSTREAM_COMMENT_SUFFIX}"
    comment
  end
end

if __FILE__ == $0
  pr_number = ARGV[0] || abort("Usage: #{$0} <pr_number>")
  client = GitHubAPIClient.new(ENV.fetch('GITHUB_TOKEN'))

  AutoReviewPR.new(client).review(pr_number)
end
