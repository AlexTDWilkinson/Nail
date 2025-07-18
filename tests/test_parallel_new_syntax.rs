#[cfg(test)]
mod test_new_parallel {
    use Nail::{lexer::lexer, parser::{parse, ASTNode}};

    #[test]
    fn test_new_parallel_syntax() {
        let code = r#"
p
    task1:s = to_string(42);
    task2:i = 100;
/p
print(task1);
"#;
        
        let tokens = lexer(code);
        println!("Tokens: {:?}", tokens);
        
        let ast = parse(tokens).unwrap();
        println!("AST: {:?}", ast);
        
        // Verify we have a ParallelAssignment node
        if let ASTNode::Program { statements, .. } = ast {
            assert!(statements.len() >= 2);
            
            // First statement should be ParallelAssignment
            match &statements[0] {
                ASTNode::ParallelAssignment { assignments, .. } => {
                    assert_eq!(assignments.len(), 2);
                    assert_eq!(assignments[0].0, "task1");
                    assert_eq!(assignments[1].0, "task2");
                }
                _ => panic!("Expected ParallelAssignment, got {:?}", statements[0])
            }
            
            // Second statement should be print call that can access task1
            match &statements[1] {
                ASTNode::FunctionCall { name, args, .. } => {
                    assert_eq!(name, "print");
                    match &args[0] {
                        ASTNode::Identifier { name, .. } => {
                            assert_eq!(name, "task1");
                        }
                        _ => panic!("Expected identifier task1")
                    }
                }
                _ => panic!("Expected print function call")
            }
        } else {
            panic!("Expected Program node");
        }
    }
}