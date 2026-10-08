# frozen_string_literal: true

module RubyOnRust
  class Shrinker
    attr_reader :evaluator

    def initialize(evaluator)
      @evaluator = evaluator
    end

    def shrink(snippet, target_type = 'Snippet')
      current_snippet = snippet

      # Phase 1: Line removal delta-debugging
      lines = current_snippet.lines.map(&:chomp)
      if lines.length > 3
        idx = 0
        while idx < lines.length
          # Try dropping line at idx (unless it's critical structure)
          candidate_lines = lines.dup
          candidate_lines.delete_at(idx)
          candidate_snippet = candidate_lines.join("\n")

          if evaluator.divergent?(candidate_snippet)
            lines = candidate_lines
            current_snippet = candidate_snippet
          else
            idx += 1
          end
        end
      end

      # Phase 2: Expression simplification
      simplified_snippet = simplify_expressions(current_snippet)
      if evaluator.divergent?(simplified_snippet)
        current_snippet = simplified_snippet
      end

      current_snippet.strip
    end

    private

    def simplify_expressions(snippet)
      # Attempt simplifications on string / numeric literals
      new_snippet = snippet.dup
      new_snippet.gsub!(/"a" \* 1000/, '"a"')
      new_snippet.gsub!(/2\*\*1000/, '100')
      new_snippet.gsub!(/Array\.new\(500\)\s*\{[^}]*\}/, '[1]')
      new_snippet
    end
  end
end
