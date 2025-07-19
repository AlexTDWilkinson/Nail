// Step-by-step boolean testing

#[cfg(test)]
mod boolean_step_tests {
    use crate::checker::*;
    use crate::lexer::*;
    use crate::parser::*;

    #[test]
    fn step1_boolean_type_exists() {
        println!("STEP 1: Testing if Boolean type exists");
        let bool_type = NailDataTypeDescriptor::Boolean;
        println!("✅ Boolean type created: {:?}", bool_type);
        assert_eq!(format!("{:?}", bool_type), "Boolean");
    }

    #[test]
    fn step2_parse_boolean_type_annotation() {
        println!("STEP 2: Testing if we can parse 'b' type annotation");
        let result = parse_data_type("b");
        println!("Parse result for 'b': {:?}", result);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), NailDataTypeDescriptor::Boolean);
    }

    #[test]
    fn step3_simple_boolean_variable() {
        println!("STEP 3: Testing simple boolean variable declaration");
        // This should test if we can declare a boolean variable
        // We'll see if this causes issues
        let test_code = "x:b = true;";
        println!("Testing code: {}", test_code);

        // Try to tokenize it
        let mut state = LexerState { line: 1, column: 1, errors: vec![] };
        let result = tokenize(test_code, &mut state);

        println!("Tokenize result: {:?}", result);
        println!("Lexer errors: {:?}", state.errors);

        // Don't assert success yet, just see what happens
    }

    #[test]
    fn step4_boolean_function_signature() {
        println!("STEP 4: Testing function with boolean return type");
        let test_code = "f test():b { r true; }";
        println!("Testing code: {}", test_code);

        let mut state = LexerState { line: 1, column: 1, errors: vec![] };
        let result = tokenize(test_code, &mut state);

        println!("Tokenize result: {:?}", result);
        println!("Lexer errors: {:?}", state.errors);
    }

    #[test]
    fn step5_comparison_operator() {
        println!("STEP 5: Testing comparison operator");
        let test_code = "x:i = 5 == 5;";
        println!("Testing code: {}", test_code);

        let mut state = LexerState { line: 1, column: 1, errors: vec![] };
        let result = tokenize(test_code, &mut state);

        println!("Tokenize result: {:?}", result);
        println!("Lexer errors: {:?}", state.errors);
    }
}
