use compiler::lexer::*;
use compiler::parser::*;

#[cfg(test)]
mod lexer_tests {
    use super::*;

    #[test]
    fn test_int_and_float() {
        let tokens = Token::lexer("123 4.5").tokens;
        assert_eq!(tokens, vec![Token::Int(123), Token::Float(4.5),]);
    }

    #[test]
    fn test_ident_and_keywords() {
        let tokens = Token::lexer("let x if else true false").tokens;
        assert_eq!(
            tokens,
            vec![
                Token::Let,
                Token::Identifier("x".into()),
                Token::If,
                Token::Else,
                Token::Bool(true),
                Token::Bool(false),
            ]
        );
    }

    #[test]
    fn test_operators() {
        let tokens = Token::lexer("+ - * / == != <= >=").tokens;
        assert_eq!(
            tokens,
            vec![
                Token::Plus,
                Token::Minus,
                Token::Star,
                Token::Slash,
                Token::EqualEqual,
                Token::NotEqual,
                Token::LessEqual,
                Token::GreaterEqual,
            ]
        );
    }

    #[test]
    fn test_parens_and_comma() {
        let tokens = Token::lexer("(a, b)").tokens;
        assert_eq!(
            tokens,
            vec![
                Token::LParen,
                Token::Identifier("a".into()),
                Token::Comma,
                Token::Identifier("b".into()),
                Token::RParen,
            ]
        );
    }

    #[test]
    fn test_whitespace_variants() {
        let tokens = Token::lexer("  let   x\t=\n123  ").tokens;
        assert_eq!(
            tokens,
            vec![
                Token::Let,
                Token::Identifier("x".into()),
                Token::Equl,
                Token::Int(123),
            ]
        );
    }

    #[test]
    fn test_bool_keywords() {
        let tokens = Token::lexer("true false").tokens;
        assert_eq!(tokens, vec![Token::Bool(true), Token::Bool(false),]);
    }

    #[test]
    fn test_multi_char_operators() {
        let tokens = Token::lexer("== != <= >=").tokens;
        assert_eq!(
            tokens,
            vec![
                Token::EqualEqual,
                Token::NotEqual,
                Token::LessEqual,
                Token::GreaterEqual,
            ]
        );
    }
}

#[cfg(test)]
mod parser_tests {
    use super::*;

    fn parse_program(input: &str) -> Program {
        let tokens = Token::lexer(input);
        let mut parser = TokenParse::convert_from_lex(tokens);
        parser.parse()
    }

    #[test]
    fn test_parse_simple_global() {
        let program = parse_program("i32: x = 5;");

        assert_eq!(
            program.items,
            vec![Item::Global(GlobalDecl {
                name: "x".to_string(),
                ty: Type::I32,
                value: Some(Expr::Operand(Token::Int(5))),
                pointer: false,
            })]
        );
    }

    #[test]
    fn test_parse_pointer_global() {
        let program = parse_program("i32: *ptr;");

        assert_eq!(
            program.items,
            vec![Item::Global(GlobalDecl {
                name: "ptr".to_string(),
                ty: Type::I32,
                value: None,
                pointer: true,
            })]
        );
    }

    #[test]
    fn test_parse_array_global() {
        let program = parse_program("i32[3]: nums = [1,2,3];");

        assert_eq!(
            program.items,
            vec![Item::Global(GlobalDecl {
                name: "nums".to_string(),
                ty: Type::Array(Box::new(Type::I32), 3),
                value: Some(Expr::Operand(Token::Array(Box::new(vec![
                    Expr::Operand(Token::Int(1)),
                    Expr::Operand(Token::Int(2)),
                    Expr::Operand(Token::Int(3)),
                ])))),
                pointer: false,
            })]
        );
    }

    #[test]
    fn test_parse_math_expression() {
        let program = parse_program("i32: x = 1 + 2 * 3;");

        assert_eq!(
            program.items,
            vec![Item::Global(GlobalDecl {
                name: "x".to_string(),
                ty: Type::I32,
                value: Some(Expr::Operation(
                    Token::Plus,
                    vec![
                        Expr::Operand(Token::Int(1)),
                        Expr::Operation(
                            Token::Star,
                            vec![Expr::Operand(Token::Int(2)), Expr::Operand(Token::Int(3)),]
                        )
                    ]
                )),
                pointer: false,
            })]
        );
    }

    #[test]
    fn test_parse_empty_function() {
        let program = parse_program(
            "
            fn i32 main() {
            }
        ",
        );

        assert_eq!(
            program.items,
            vec![Item::Function(FunctionDecl {
                name: "main".to_string(),
                params: vec![],
                return_type: Type::I32,
                body: vec![],
            })]
        );
    }

    #[test]
    fn test_parse_function_params() {
        let program = parse_program(
            "
            fn i32 add(a: i32 b: i32) {
            }
        ",
        );

        assert_eq!(
            program.items,
            vec![Item::Function(FunctionDecl {
                name: "add".to_string(),
                params: vec![
                    Param {
                        name: "a".to_string(),
                        ty: Type::I32,
                    },
                    Param {
                        name: "b".to_string(),
                        ty: Type::I32,
                    }
                ],
                return_type: Type::I32,
                body: vec![],
            })]
        );
    }

    #[test]
    fn test_parse_return_statement() {
        let program = parse_program(
            "
            fn i32 main() {
                return 5;
            }
        ",
        );

        assert_eq!(
            program.items,
            vec![Item::Function(FunctionDecl {
                name: "main".to_string(),
                params: vec![],
                return_type: Type::I32,
                body: vec![Stmt::Return(Some(Expr::Operand(Token::Int(5))))],
            })]
        );
    }

    #[test]
    fn test_parse_assignment_statement() {
        let program = parse_program(
            "
            fn Void main() {
                let i32: x = 10;
            }
        ",
        );

        assert_eq!(
            program.items,
            vec![Item::Function(FunctionDecl {
                name: "main".to_string(),
                params: vec![],
                return_type: Type::Void,
                body: vec![Stmt::Assignment(AssignmentStmt {
                    name: "x".to_string(),
                    ty: Some(Type::I32),
                    value: Some(Expr::Operand(Token::Int(10))),
                    pointer: false,
                })],
            })]
        );
    }

    #[test]
    fn test_parse_if_statement() {
        let program = parse_program(
            "
            fn Void main() {
                if x {
                    return;
                }
            }
        ",
        );

        assert_eq!(program.items.len(), 1);

        match &program.items[0] {
            Item::Function(func) => match &func.body[0] {
                Stmt::If(if_stmt) => {
                    assert_eq!(
                        if_stmt.condition,
                        Expr::Operand(Token::Identifier("x".to_string()))
                    );

                    assert_eq!(if_stmt.then_branch.len(), 1);
                }

                _ => panic!("expected if statement"),
            },

            _ => panic!("expected function"),
        }
    }

    #[test]
    fn test_parse_while_statement() {
        let program = parse_program(
            "
            fn Void loop_test() {
                while true {
                    break;
                }
            }
        ",
        );

        match &program.items[0] {
            Item::Function(func) => match &func.body[0] {
                Stmt::While(WhileStmt { condition, body }) => {
                    assert_eq!(*condition, Expr::Operand(Token::Bool(true)));

                    assert_eq!(body[0], Stmt::Break);
                }

                _ => panic!("expected while"),
            },

            _ => panic!("expected function"),
        }
    }

    #[test]
    fn test_parse_function_call() {
        let program = parse_program(
            "
            fn Void main() {
                print(1, 2);
            }
        ",
        );

        match &program.items[0] {
            Item::Function(func) => match &func.body[0] {
                Stmt::Expr(Expr::Operand(Token::Call(name, args))) => {
                    assert_eq!(name, "print");

                    assert_eq!(
                        **args,
                        vec![Expr::Operand(Token::Int(1)), Expr::Operand(Token::Int(2)),]
                    );
                }

                _ => panic!("expected function call"),
            },

            _ => panic!("expected function"),
        }
    }

    #[test]
    #[should_panic]
    fn test_missing_semicolon_panics() {
        parse_program("i32: x = 5");
    }

    #[test]
    #[should_panic]
    fn test_unclosed_array_panics() {
        parse_program("i32[3]: x = [1,2,3;");
    }

    #[test]
    #[should_panic]
    fn test_invalid_function_panics() {
        parse_program(
            "
            fn main() {
            }
        ",
        );
    }
}
