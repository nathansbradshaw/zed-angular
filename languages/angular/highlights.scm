; Match style content
((style_element (raw_text) @css)
 (#set! "language" "css"))
((tag_name) @html-tag)
((self_closing_tag) @html-tag)
((attribute_name) @attribute)

; Highlight Angular bindings, variable and function identifiers
((identifier) @variable)
((property_binding) @angular-binding)
((event_binding) @angular-binding)
((comment) @comment)
((doctype) @doctype)
((entity) @entity)
(tag_name) @tag
(erroneous_end_tag_name) @keyword
(doctype) @constant
(attribute_name) @property
(attribute_value) @string
(comment) @comment
; (raw_text) @embedded

;; Angular control flow
(control_keyword) @keyword.control
(special_keyword) @keyword.special
(let_statement (control_keyword) @keyword.control)

;; Pipe handling
(pipe_sequence
  (pipe_operator) @operator
  (pipe_call name: (identifier) @function))

;; Property bindings
(property_binding
  "[" @property.binding
  (binding_name) @property.binding
  "]" @property.binding)

;; Event bindings
(event_binding
  "(" @property.binding
  (binding_name) @property.binding
  ")" @property.binding)

;; Two-way bindings
(two_way_binding
  "[(" @property.binding
  (binding_name) @property.binding
  ")]" @property.binding)

;; Simple structural directive matching
(structural_directive
  (identifier) @property.binding)

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
((interpolation) @punctuation.special)

; Match script content
((script_element (raw_text) @javascript)
 (#set! "language" "javascript"))

 ; Ensure variables in expressions are highlighted
(expression
  (identifier) @variable)


"=" @operator
"@" @keyword.special
"*" @property.binding

[
  "<"
  ">"
  "<!"
  "</"
  "/>"
] @punctuation.bracket

;; Angular tags highlights
((tag_name) @type
  (#match? @type ".*-.*"))

; --- Angular Template String Highlighting ---

; Highlight the static text parts of the template string.
(template_chars) @string

; Highlight the interpolation markers: ${ and }
(template_substitution
  "${" @punctuation.special
  "}" @punctuation.special)

; Highlight the variable name inside the interpolation.
(expression
  (identifier) @variable)
