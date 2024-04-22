; Combine all rules into one file for Angular projects
; TypeScript rules
((function_declaration
  body: (block) @fold)
 (method_definition
  body: (block) @fold)
 (class_declaration
  body: (class_body) @fold)
 (interface_declaration
  body: (object_type) @fold)
 (enum_declaration
  body: (enum_body) @fold)
 (block_statement) @fold)

; Angular HTML template rules
(element
  (start_tag
    (attribute
      (attribute_name) @directive
      (#match? @directive "ngFor|ngIf|ngSwitch")))
  @fold)
(element) @fold
(embedded_template) @fold
