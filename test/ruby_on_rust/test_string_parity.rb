# frozen_string_literal: true

# String parity test snippet
s = "Ruby on Rust Differential Testing"
reversed = s.reverse
upcase = s.upcase
match = s.match(/(Ruby|Rust)/) ? $1 : nil

[s.bytesize, reversed, upcase, match]
