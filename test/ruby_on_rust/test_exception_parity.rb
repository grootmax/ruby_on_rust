# frozen_string_literal: true

# Exception parity test snippet
begin
  1 / 0
rescue ZeroDivisionError => e
  puts "Caught: #{e.class} - #{e.message}"
end

"exception handled"
