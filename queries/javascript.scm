(import_statement source: (string) @import.path)
(call_expression function: (identifier) @_fn (#eq? @_fn "require") arguments: (arguments (string) @import.path))
(function_declaration name: (identifier) @symbol.fn)
