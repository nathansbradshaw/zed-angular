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

; Highlight types in declarations and type assertions
((type_annotation
  type: (identifier) @type)
 (as_expression
  type: (identifier) @type))

; Highlight decorators
((decorator
  (field_identifier) @decorator))

; Special keywords and control flow
("if" @keyword)
("else" @keyword)
("for" @keyword)
("while" @keyword)
("return" @keyword)

; Operators
("=" @operator)
("+" @operator)
("-" @operator)
("==" @operator)
("===" @operator)
("&&" @operator)
("||" @operator)

; Brackets and punctuation
("{" @punctuation.bracket)
("}" @punctuation.bracket)
("(" @punctuation.bracket)
(")" @punctuation.bracket)
("[" @punctuation.bracket)
("]" @punctuation.bracket)
("<" @punctuation.bracket)
(">" @punctuation.bracket)
("," @punctuation.delimiter)
(";" @punctuation.delimiter)

; Comments
((comment) @comment)
