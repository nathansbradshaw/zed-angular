; Highlight variable and function identifiers
((identifier) @variable)

; Highlight angular bindings and directives
("[" @punctuation.bracket
 (identifier) @directive
 "]") @punctuation.bracket

("(" @punctuation.bracket
 (identifier) @directive
 ")") @punctuation.bracket

("[(" @punctuation.bracket
 (identifier) @directive
 ")]") @punctuation.bracket

; Directives like *ngIf, *ngFor
((identifier) @directive
 (#match? @directive "^\\*"))

; Highlighting strings
((string) @string)

; Highlight numbers
((number) @number)

; Highlight function calls
((call_expression
  function: (identifier) @function))

; Highlight function parameters in a call
((call_expression
  arguments: (arguments) @parameter))

; Highlight property accesses
((member_expression
  property: (identifier) @property))


