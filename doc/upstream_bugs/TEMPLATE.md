# Upstream Bug / Quirk Report: [Title]

## 1. Metadata
- **Report Title:** [Short descriptive title]
- **CRuby Baseline Version:** [e.g., CRuby 3.4.0 / git commit SHA `4d835210ecbf9de894f39041ae2d7d59b80bb06f`]
- **Redmine Submission Status:** `[ ] Draft / Unsubmitted` | `[ ] Submitted`
- **Redmine Ticket Link:** [N/A or https://bugs.ruby-lang.org/issues/XXXXX]
- **Status Flag:** `Draft` | `Submitted` | `In Review` | `Resolved` | `WontFix`
- **Rust Port Source Path:** [e.g., `core_rs/src/numeric.rs`]
- **Original C Source Path:** [e.g., `numeric.c`]

## 2. Description
Provide a detailed explanation of the CRuby bug, quirk, edge case, or unexpected behavior encountered during C-to-Rust porting.

## 3. Minimal Reproduction Code Snippet
```ruby
# Minimal Ruby script reproducing the behavior
def reproduce
  # Code here
end

reproduce
```

## 4. Expected vs Actual Behavior
- **Expected Behavior:** [Describe expected behavior per Ruby spec, ISO standard, or general consistency]
- **Actual CRuby Behavior:** [Describe actual behavior in baseline C implementation]

## 5. Behavior Differences & Porting Impact
- **Impact on Rust Port:** [Explain how this quirk or bug affects the Rust implementation in core_rs]
- **Rust Workaround / Parity Shim:** [Describe any compatibility shims or handling added to Rust code]

## 6. Upstream Redmine Details
- **Redmine Project:** Ruby Master / Ruby 3.x
- **Redmine Issue ID:** [e.g., #12345 or Unsubmitted Draft]
- **Redmine Author:** [Submitter username]
- **Redmine Status:** [New / Open / Assigned / Feedback / Closed / Rejected]
- **Upstream Fix Commit:** [Git commit SHA if fixed upstream]
