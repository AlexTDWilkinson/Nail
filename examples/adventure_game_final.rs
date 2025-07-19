warning: unused imports: `DisableMouseCapture`, `EnableMouseCapture`, `EnterAlternateScreen`, `Event`, `KeyCode`, `LeaveAlternateScreen`, `disable_raw_mode`, `enable_raw_mode`, `execute`, and `self`
 --> src/colorizer.rs:3:13
  |
3 |     event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
  |             ^^^^  ^^^^^^^^^^^^^^^^^^^  ^^^^^^^^^^^^^^^^^^  ^^^^^  ^^^^^^^
4 |     execute,
  |     ^^^^^^^
5 |     terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
  |                ^^^^^^^^^^^^^^^^  ^^^^^^^^^^^^^^^  ^^^^^^^^^^^^^^^^^^^^  ^^^^^^^^^^^^^^^^^^^^
  |
  = note: `#[warn(unused_imports)]` on by default

warning: unused imports: `Block`, `Borders`, `Constraint`, `Direction`, `Frame`, `Layout`, `Paragraph`, `ScrollbarOrientation`, `ScrollbarState`, `Scrollbar`, `Tabs`, `Terminal`, and `backend::CrosstermBackend`
  --> src/colorizer.rs:9:5
   |
9  |     backend::CrosstermBackend,
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^
10 |     layout::{Constraint, Direction, Layout},
   |              ^^^^^^^^^^  ^^^^^^^^^  ^^^^^^
...
13 |     widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Tabs},
   |               ^^^^^  ^^^^^^^  ^^^^^^^^^  ^^^^^^^^^  ^^^^^^^^^^^^^^^^^^^^  ^^^^^^^^^^^^^^  ^^^^
14 |     Frame, Terminal,
   |     ^^^^^  ^^^^^^^^

warning: unused import: `regex::Regex`
  --> src/colorizer.rs:17:5
   |
17 | use regex::Regex;
   |     ^^^^^^^^^^^^

warning: unused import: `std::sync::Mutex`
   --> src/colorizer.rs:132:5
    |
132 | use std::sync::Mutex;
    |     ^^^^^^^^^^^^^^^^

warning: unused import: `crate::statics_for_tests::*`
 --> src/lexer.rs:2:5
  |
2 | use crate::statics_for_tests::*;
  |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: unused import: `dashmap::DashMap`
 --> src/lexer.rs:3:5
  |
3 | use dashmap::DashMap;
  |     ^^^^^^^^^^^^^^^^

warning: unused import: `lazy_static::lazy_static`
 --> src/lexer.rs:4:5
  |
4 | use lazy_static::lazy_static;
  |     ^^^^^^^^^^^^^^^^^^^^^^^^

warning: unused imports: `Command` and `ExitCode`
 --> src/parser/std_lib/process.rs:1:20
  |
1 | use std::process::{Command, ExitCode};
  |                    ^^^^^^^  ^^^^^^^^

warning: unused import: `std::hash::Hasher`
 --> src/lexer.rs:9:5
  |
9 | use std::hash::Hasher;
  |     ^^^^^^^^^^^^^^^^^

warning: unused variable: `code_span`
   --> src/checker.rs:134:40
    |
134 |         ASTNode::Program { statements, code_span, .. } => statements.iter_mut().for_each(|statement| visit_node(statement, state)),
    |                                        ^^^^^^^^^-
    |                                        |
    |                                        help: try removing the field
    |
    = note: `#[warn(unused_variables)]` on by default

warning: unused variable: `code_span`
   --> src/checker.rs:143:65
    |
143 | ...   ASTNode::IfStatement { condition_branches, else_branch, code_span, .. } => visit_if_statement(condition_branches, else_branch, state),
    |                                                               ^^^^^^^^^-
    |                                                               |
    |                                                               help: try removing the field

warning: unused variable: `field_name`
   --> src/checker.rs:287:56
    |
287 |         if let ASTNode::StructDeclarationField { name: field_name, data_type, .. } = field {
    |                                                        ^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_field_name`

warning: unused variable: `struct_name`
   --> src/checker.rs:289:48
    |
289 |                 NailDataTypeDescriptor::Struct(struct_name) => {
    |                                                ^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_struct_name`

warning: unused variable: `enum_name`
   --> src/checker.rs:293:46
    |
293 |                 NailDataTypeDescriptor::Enum(enum_name) => {
    |                                              ^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_enum_name`

warning: unused variable: `code_span`
   --> src/checker.rs:302:88
    |
302 | fn visit_enum_declaration(name: &str, variants: &[ASTNode], state: &mut AnalyzerState, code_span: &mut CodeSpan) {
    |                                                                                        ^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_code_span`

warning: unused variable: `scope`
   --> src/checker.rs:446:45
    |
446 |         ASTNode::FunctionCall { name, args, scope, .. } => {
    |                                             ^^^^^-
    |                                             |
    |                                             help: try removing the field

warning: unused variable: `value_type`
   --> src/checker.rs:683:13
    |
683 |         let value_type = check_type(value, state);
    |             ^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_value_type`

warning: value assigned to `need_space` is never read
   --> src/colorizer.rs:463:9
    |
463 |         need_space = false;
    |         ^^^^^^^^^^
    |
    = help: maybe it is overwritten before being read?
    = note: `#[warn(unused_assignments)]` on by default

warning: static `NUMBERS` is never used
  --> src/lexer.rs:19:8
   |
19 | static NUMBERS: &str = "0123456789";
   |        ^^^^^^^
   |
   = note: `#[warn(dead_code)]` on by default

warning: function `is_function_call` is never used
   --> src/lexer.rs:812:4
    |
812 | fn is_function_call(chars: &mut std::iter::Peekable<std::str::Chars>) -> bool {
    |    ^^^^^^^^^^^^^^^^

warning: function `skip_whitespace` is never used
   --> src/lexer.rs:830:4
    |
830 | fn skip_whitespace(chars: &mut std::iter::Peekable<std::str::Chars>, state: &mut LexerState) {
    |    ^^^^^^^^^^^^^^^

warning: function `is_array` is never used
   --> src/lexer.rs:999:4
    |
999 | fn is_array(chars: &mut std::iter::Peekable<std::str::Chars>) -> bool {
    |    ^^^^^^^^

warning: function `parse_if_statement` is never used
   --> src/parser.rs:560:4
    |
560 | fn parse_if_statement(state: &mut ParserState) -> Result<ASTNode, CodeError> {
    |    ^^^^^^^^^^^^^^^^^^

warning: type alias `RouteHandler` is never used
 --> src/parser/std_lib/http.rs:7:6
  |
7 | type RouteHandler = Arc<dyn Fn() -> String + Send + Sync>;
  |      ^^^^^^^^^^^^

warning: field `scope_level` is never read
 --> src/transpilier.rs:8:5
  |
6 | pub struct Transpiler {
  |            ---------- field in this struct
7 |     indent_level: usize,
8 |     scope_level: usize,
  |     ^^^^^^^^^^^

warning: method `rust_async_return_type` is never used
   --> src/transpilier.rs:342:8
    |
13  | impl Transpiler {
    | --------------- method in this implementation
...
342 |     fn rust_async_return_type(&self, data_type: &NailDataTypeDescriptor, name: &str) -> String {
    |        ^^^^^^^^^^^^^^^^^^^^^^

warning: function `insert_semicolons` is never used
   --> src/transpilier.rs:633:4
    |
633 | fn insert_semicolons(code: String) -> String {
    |    ^^^^^^^^^^^^^^^^^

warning: crate `Nail` should have a snake case name
  |
  = help: convert the identifier to snake case: `nail`
  = note: `#[warn(non_snake_case)]` on by default

warning: `Nail` (lib) generated 28 warnings (run `cargo fix --lib -p Nail` to apply 11 suggestions)
warning: unused variable: `used_stdlib_functions`
  --> src/bin/nailc.rs:50:15
   |
50 |     let (ast, used_stdlib_functions) = match parse(tokens) {
   |               ^^^^^^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_used_stdlib_functions`
   |
   = note: `#[warn(unused_variables)]` on by default

warning: `Nail` (bin "nailc") generated 1 warning
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.05s
     Running `target/debug/nailc examples/adventure_game_final.nail`
=== Lexing examples/adventure_game_final.nail ===

=== Parsing ===
Parse successful!
Used stdlib functions: {"process_exit", "range", "io_read_line_prompt", "array_get", "print", "string_from", "string_contains", "dangerous", "reduce_int", "string_concat"}

=== Type Checking ===
Type check errors:
  Error at line 126, column 3: Type mismatch in constant declaration named `final_state`: expected a:i, got i
  Error at line 126, column 2: reduce_int parameter 'initial' expects type Int, got ArrayInt

Use --skip-check to transpile anyway
