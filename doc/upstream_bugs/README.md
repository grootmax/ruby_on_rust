# Upstream Bugs & Quirks Index

This directory (`doc/upstream_bugs/`) systematically tracks CRuby quirks, edge-case behavior discrepancies, and bug report drafts discovered during the C-to-Rust porting process (Rule 4: Upstream Bug Tracking).

When porting C source code to Rust, developers may encounter unexpected CRuby behaviors, memory safety edge cases, or bugs in the baseline C codebase. Recording these drafts locally ensures quirks and findings are documented and tracked systematically before or alongside submission to the CRuby Redmine issue tracker ([bugs.ruby-lang.org](https://bugs.ruby-lang.org/)).

## Upstream Bug Reports & Quirks Table

| ID | Report Title | Status Flag | Redmine Ticket Link | Rust Port Source Reference | CRuby Baseline |
|---|---|---|---|---|---|
| *Example* | *Integer overflow in strtoul parsing* | `Draft` | *Unsubmitted Draft* | `core_rs/src/util.rs` | `4d835210` |

### Status Flag Legend

- **Draft:** Local report draft created; not yet submitted to upstream Redmine.
- **Submitted:** Report submitted to CRuby Redmine tracker; awaiting upstream triaging.
- **In Review:** Active discussion or review ongoing on the Redmine ticket.
- **Resolved:** Issue fixed in upstream CRuby master repository.
- **WontFix:** Upstream designated behavior as intended quirk or WontFix; Rust port maintains CRuby behavior.

## Workflow & Guidelines

1. **Creating a Report Draft:**
   - Copy [`TEMPLATE.md`](TEMPLATE.md) to a new Markdown file in `doc/upstream_bugs/` named after the issue (e.g., `0001-numeric-overflow-quirk.md`).
   - Fill in all template sections, including CRuby baseline version, reproduction snippet, behavior differences, Rust source path, and Redmine submission status.

2. **Registering in Index Table:**
   - Add a row to the table above in `doc/upstream_bugs/README.md`.
   - Ensure both unsubmitted drafts (`Draft`) and submitted tickets (`Submitted`, `In Review`, etc.) are tracked here.

3. **Submitting Upstream:**
   - Submit the issue to [bugs.ruby-lang.org](https://bugs.ruby-lang.org/) as outlined in [`doc/contributing/reporting_issues.md`](../contributing/reporting_issues.md).
   - Once submitted, update the report file and this index table with the Redmine ticket link and set the status flag to `Submitted`.
