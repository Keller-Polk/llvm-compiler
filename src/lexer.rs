use crate::parser::Expr;
use std::fs;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // literals
    Int(u32),
    Float(f64),
    Bool(bool),
    String(String),
    Array(Box<Vec<Expr>>),
    Call(String, Box<Vec<Expr>>),

    // identifiers
    Identifier(String),

    // operators
    Plus,
    Minus,
    Star, //*
    Slash,
    Percent, //in retrospect frogot modulus opperation change soon

    And,
    Or,

    EqualEqual,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,

    Not,
    Ampersand, //&

    // punctuation
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Colon,
    Equl,
    Semicolon,

    // keywords
    If,
    Else,
    For,
    While,
    Const,
    Let,
    Fn,
    Return,
    Continue,
    Break,
    SizeOf,

    // types
    Type(Type),

    EOF,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
    Bool,
    Char,
    Void,
    Array(Box<Type>, u32),
    Pointer(Box<Type>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct location {
    pub colum: u32,
    pub line: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct lexedFile {
    pub tokens: Vec<Token>,
    pub locations: Vec<location>,
}

impl Token {
    pub fn infix_binding_power(op: Token) -> (f32, f32) {
        match op {
            Token::Comma => (0.0, 0.0),
            Token::Equl => (0.2, 0.1),

            Token::Or => (0.3, 0.4),
            Token::And => (0.5, 0.6),

            Token::EqualEqual | Token::NotEqual => (0.7, 0.8),

            Token::Less | Token::LessEqual | Token::Greater | Token::GreaterEqual => (0.9, 1.0),

            Token::Plus | Token::Minus => (1.1, 1.2),

            Token::Star | Token::Slash | Token::Percent => (1.3, 1.4),

            Token::Ampersand => (1.5, 1.6),

            Token::LBracket | Token::RBracket => (50.0, 51.0),

            Token::LParen => (100.0, 101.0),

            _ => panic!("bad op: {:?}", op),
        }
    }
    pub fn lexer(line: &str) -> lexedFile {
        let mut locations = Vec::new();
        let chars: Vec<char> = line.chars().collect();
        let mut tokens = Vec::new();
        let mut i = 0;

        let mut row: u32 = 1;
        let mut col: u32 = 1;

        while i < chars.len() {
            let c = chars[i];

            //skip

            if c == '\n' {
                row += 1;
                col = 1;
                i += 1;
                continue;
            }
            if c.is_whitespace() {
                col += 1;
                i += 1;
                continue;
            }

            // text
            if c.is_alphabetic() || c == '_' {
                let start_i = i;
                // get item
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1
                }
                let text: String = chars[start_i..i].iter().collect();
                let token = match text.as_str() {
                    "if" => Token::If,
                    "else" => Token::Else,
                    "let" => Token::Let,
                    "for" => Token::For,
                    "while" => Token::While,
                    "const" => Token::Const,
                    "fn" => Token::Fn,
                    "return" => Token::Return,
                    "true" => Token::Bool(true),
                    "false" => Token::Bool(false),
                    "continue" => Token::Continue,
                    "break" => Token::Break,
                    "sizeof" => Token::SizeOf,
                    "i8" => Token::Type(Type::I8),
                    "i16" => Token::Type(Type::I16),
                    "i32" => Token::Type(Type::I32),
                    "i64" => Token::Type(Type::I64),
                    "u8" => Token::Type(Type::U8),
                    "u16" => Token::Type(Type::U16),
                    "u32" => Token::Type(Type::U32),
                    "u64" => Token::Type(Type::U64),
                    "f32" => Token::Type(Type::F32),
                    "f64" => Token::Type(Type::F64),
                    "bool" => Token::Type(Type::Bool),
                    "char" => Token::Type(Type::Char),
                    "void" => Token::Type(Type::Void),
                    _ => Token::Identifier(text),
                };
                locations.push(location {
                    colum: col,
                    line: row,
                });
                tokens.push(token);
                continue;
            }
            if c.is_numeric() {
                let mut has_decimal = false;
                let start_i = i;
                while i < chars.len()
                    && (chars[i].is_numeric() || (!has_decimal && chars[i] == '.'))
                {
                    if chars[i] == '.' {
                        has_decimal = true;
                    }
                    i += 1
                }
                let text: String = chars[start_i..i].iter().collect();
                let token = if has_decimal {
                    Token::Float(text.parse().unwrap())
                } else {
                    Token::Int(text.parse().unwrap())
                };
                locations.push(location {
                    colum: col,
                    line: row,
                });
                tokens.push(token);
                continue;
            }
            //operators
            let two_char = if i + 1 < chars.len() {
                Some(format!("{}{}", chars[i], chars[i + 1]))
            } else {
                None
            };
            if let Some(op) = two_char {
                let token = match op.as_str() {
                    "==" => Some(Token::EqualEqual),
                    "!=" => Some(Token::NotEqual),
                    "<=" => Some(Token::LessEqual),
                    ">=" => Some(Token::GreaterEqual),
                    "&&" => Some(Token::And),
                    "||" => Some(Token::Or),
                    _ => None,
                };

                if let Some(tok) = token {
                    tokens.push(tok);
                    i += 2;
                    continue;
                }
            }
            let token = match c {
                '+' => Token::Plus,
                '-' => Token::Minus,
                '*' => Token::Star,
                '/' => Token::Slash,
                '(' => Token::LParen,
                ')' => Token::RParen,
                '{' => Token::LBrace,
                '}' => Token::RBrace,
                '&' => Token::Ampersand,
                '=' => Token::Equl,
                '<' => Token::Less,
                '>' => Token::Greater,
                ',' => Token::Comma,
                ':' => Token::Colon,
                ';' => Token::Semicolon,
                '[' => Token::LBracket,
                ']' => Token::RBracket,
                _ => panic!("Unknown character: {}", c),
            };
            locations.push(location {
                colum: col,
                line: row,
            });
            tokens.push(token);
            i += 1;
        }
        return lexedFile {
            tokens: tokens,
            locations: locations,
        };
    }

    pub fn load_file(path: &str) -> lexedFile {
        let loadtmp = fs::read_to_string(path);
        let contents: String;
        match loadtmp {
            Ok(c) => {
                contents = c;
            }

            Err(err) => {
                panic!("Failed to load file: {}", err);
            }
        }
        return Self::lexer(&contents);
    }
}
