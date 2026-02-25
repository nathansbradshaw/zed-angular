
(start_tag ">" @end) @indent
(self_closing_tag "/>" @end) @indent

(element
  (start_tag) @start
  (end_tag)? @end) @indent

; Angular control flow blocks (@if, @for, @switch, @defer, etc.)
(statement_block "{" "}" @end) @indent
(switch_body "{" "}" @end) @indent
