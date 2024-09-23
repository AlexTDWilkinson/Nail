pub static IF_STATEMENT: &str = r#"
  if x:i {
     x:i >= 0:i => { return 4; },
     x:i < 123:i => { return 5; },
     else => { return 6; },
 };
 "#;

pub static FUNCTION_DEFINITION: &str = "
 let seven:i = 7;
 fn add(x:i, y:i):i = x + y;
 add(2:i, 3:i) * -seven";

pub static CONST_ASSIGNMENT: &str = "c x:i = 10;";
pub static NESTED_EXPRESSION: &str = "2 + 3 * 4";
pub static SIMPLE_ADDITION_OF_NEGATIVE_NUMBERS: &str = "-1 + -2";
pub static SIMPLE_ADDITION: &str = "1 + 2";
pub static SIMPLE_DECLARATION: &str = "c bill:i = bob";
pub static SIMPLE_IDENT: &str = "bob:i";
pub static SIMPLE_CONST: &str = "c bob:i";
pub static SIMPLE_CONST_OPERATOR: &str = "c bob:i =";
pub static SIMPLE_CONST_ASSIGNMENT: &str = "c x:i = 10;";
pub static MULTILINE_STRING: &str = r#""This is a story all about how my life
got flipped turned upside down, and I'd like to take a minute just sit right
there, I'll tell you how I became the
prince of a town called Bel-Air.""#;

pub static RUST_ESCAPE: &str = "R{ let x:i = 10; }";
pub static RUST_ESCAPE_MULTILINE: &str = "R{
x:i = 10;
}";

pub static RUST_ESCAPE_NESTED_BLOCKS: &str = "R{ { let x:i = 10; } }";

pub static RUST_ESCAPE_NAIL_INJECTION: &str = r#"R{ println!("Hello, ^[name]^! You are ^[ 18 + 8 ]^ years old."); }"#;
