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


; Match property bindings
((property_binding) @property-binding)

; Match event bindings
((event_binding) @event-binding)
; Match interpolation expressions
((interpolation) @interpolation)
; Match script content
((script_element (raw_text) @javascript)
 (#set! "language" "javascript"))

; Match style content
((style_element (raw_text) @css)
 (#set! "language" "css"))
((tag_name) @html-tag)
((self_closing_tag) @html-tag)
((attribute_name) @attribute)
;; Highlight Angular bindings specially
((property_binding) @angular-binding)
((event_binding) @angular-binding)
((comment) @comment)
((doctype) @doctype)
((entity) @entity)
(tag_name) @keyword
(erroneous_end_tag_name) @keyword
(doctype) @constant
(attribute_name) @property
(attribute_value) @string
(comment) @comment

"=" @operator

[
  "<"
  ">"
  "<!"
  "</"
  "/>"
] @punctuation.bracket

(tag_name) @tag
(erroneous_end_tag_name) @keyword
(doctype) @tag.doctype
(attribute_name) @property
(attribute_value) @string
(comment) @comment

"=" @operator

[
  "<"
  ">"
  "<!"
  "</"
  "/>"
] @punctuation.bracket
