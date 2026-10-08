# frozen_string_literal: true

# Basic parity test snippet
puts "Testing basic parity"
a = 10
b = 20
c = a + b
hash = { sum: c, list: [1, 2, 3].map { |x| x * 2 } }
hash
