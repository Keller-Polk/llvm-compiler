use crate::lexer::*;

// temp we consume while parseing
#[derive(Debug, Clone, PartialEq)]
pub enum TokenExpr {
    Operand(Token),
    Op(Token),
    Key(Token),
    Type(Token),
    Eof,
}
//larger structure of temp we consome while parseing.
#[derive(Debug, Clone, PartialEq)]
pub struct TokenParse {
    pub tokens: Vec<TokenExpr>,
    pub locations: Vec<location>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TokenParseItem {
    pub tokens: TokenExpr,
    pub locations: location,
}

// what we generate from parseing expresions
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Operand(Token),
    Operation(Token, Vec<Expr>),
    SizeOf(Type),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Function(FunctionDecl),
    Global(GlobalDecl),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Type,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GlobalDecl {
    pub name: String,
    pub ty: Type,
    pub value: Option<Expr>,
    pub pointer: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Assignment(AssignmentStmt),
    Expr(Expr),
    Return(Option<Expr>),
    If(IfStmt),
    While(WhileStmt),
    Break,
    Continue,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AssignmentStmt {
    pub name: String,
    pub ty: Option<Type>,
    pub value: Option<Expr>,
    pub pointer: bool,
    pub dereference: bool,
    pub index: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfStmt {
    pub condition: Expr,
    pub then_branch: Vec<Stmt>,
    pub else_branch: Option<Vec<Stmt>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WhileStmt {
    pub condition: Expr,
    pub body: Vec<Stmt>,
}

fn errorHandle(message: String, file: &mut TokenParse) -> ! {
    if file.locations.len() > 0 {
        println!(
            "parseing error at line:{} colum:{:?}, {:?}",
            (file.locations[file.locations.len() - 1].line),
            (file.locations[file.locations.len() - 1].colum),
            message
        );
    } else {
        println!("parseing error, {:?}", message);
    }
    return panic!();
}

fn errormess(message: String, file: &mut TokenParse) -> String {
    return format!(
        "parseing error at line:{:?} colum:{:?}, {:?}",
        (file.locations[0].line),
        (file.locations[0].colum),
        message
    );
}

// a state we convert our lexed expresions to in order to parse them
impl TokenParse {
    pub fn convert_from_lex(mut lex: lexedFile) -> Self {
        let mut lexer = vec![];
        for x in lex.tokens {
            lexer.push(TokenExpr::convert_from_token(x))
        }
        lexer.push(TokenExpr::Eof);
        lexer.reverse();
        lex.locations.reverse();
        TokenParse {
            tokens: lexer,
            locations: lex.locations,
        }
    }
    fn next(&mut self) -> TokenExpr {
        self.locations.pop();
        self.tokens.pop().unwrap()
    }
    fn nextP(&mut self) -> TokenParseItem {
        let y = self.locations.pop().unwrap();
        let x = self.tokens.pop().unwrap();
        TokenParseItem {
            tokens: x,
            locations: y,
        }
    }
    fn peek(&mut self) -> TokenExpr {
        self.tokens.last().unwrap().clone()
    }
    fn push(&mut self, pushing: TokenParseItem) {
        self.tokens.push(pushing.tokens);
        self.locations.push(pushing.locations);
    }
    pub fn parse(&mut self) -> Program {
        let mut program: Program = Program { items: Vec::new() };
        let mut item: Item;
        while self.tokens.len() > 1 {
            item = match self.next() {
                TokenExpr::Key(Token::Fn) => self.function_parse(),
                TokenExpr::Key(Token::Type(x)) => self.global_assignment_parse(x),
                x => errorHandle(format!("unexpected token {:?}", x), self),
            };
            program.items.push(item.clone());
        }
        program
    }
    pub fn function_parse(&mut self) -> Item {
        let mut ty = match self.next() {
            TokenExpr::Key(Token::Type(x)) => x,
            TokenExpr::Op(Token::Star) => match self.next() {
                TokenExpr::Key(Token::Type(x)) => Type::Pointer(Box::new(x)),
                _ => Type::Pointer(Box::new(Type::Void)),
            },
            _ => Type::Void,
        };

        ty = match self.peek() {
            TokenExpr::Op(Token::LBracket) => {
                self.next();
                let x = match self.next() {
                    TokenExpr::Operand(Token::Int(len)) => Type::Array(Box::new(ty), len),
                    t => errorHandle(
                        format!("cannot deffine an array with a non integer length. {:?}", t),
                        self,
                    ),
                };
                self.next();
                x
            }
            _ => ty,
        };

        let name = match self.next() {
            TokenExpr::Operand(Token::Identifier(name)) => name,
            x => errorHandle(
                format!("varbles must have valid alphanumeric names {:?}", x),
                self,
            ),
        };

        assert_eq!(
            self.peek(),
            TokenExpr::Op(Token::LParen),
            "all functions requre perenthisis {:?}",
            self.peek()
        );
        self.next();

        let mut params: Vec<Param> = Vec::new();
        let mut ty_p: Type;
        let mut name_P: String;

        while self.peek() != TokenExpr::Op(Token::RParen) {
            name_P = match self.next() {
                TokenExpr::Operand(Token::Identifier(name)) => name,
                x => errorHandle(
                    format!("parameters must have valid alphanumeric names {:?}", x),
                    self,
                ),
            };
            assert_eq!(
                self.peek(),
                TokenExpr::Key(Token::Colon),
                "all function perameters requre a colon between the name and the type. {:?}",
                self.peek()
            );
            self.next();

            ty_p = match self.next() {
                TokenExpr::Key(Token::Type(x)) => x,
                t => errorHandle(format!("perameters must have types {:?}", t), self),
            };
            ty_p = match self.peek() {
                TokenExpr::Op(Token::LBracket) => {
                    self.next();
                    let x = match self.next() {
                        TokenExpr::Operand(Token::Int(len)) => Type::Array(Box::new(ty_p), len),
                        t => errorHandle(
                            format!("cannot deffine an array with a non integer length. {:?}", t),
                            self,
                        ),
                    };
                    self.next();
                    x
                }
                _ => ty_p,
            };
            params.push(Param {
                name: name_P,
                ty: ty_p,
            });
            if self.peek() == TokenExpr::Op(Token::Comma) {
                self.next();
            }
        }
        // consume the right paren
        self.next();
        //consume the left brace
        assert_eq!(
            self.peek(),
            TokenExpr::Key(Token::LBrace),
            "all functions requre a body {:?}",
            self.peek()
        );
        self.next();

        let mut body: Vec<Stmt> = Vec::new();
        let mut cur_Stmt: Stmt;

        //parse body of function
        while self.peek() != TokenExpr::Key(Token::RBrace) {
            cur_Stmt = self.parse_Stmt();
            body.push(cur_Stmt);
        }
        //consume the right brace
        self.next();

        let mut function = FunctionDecl {
            name: name,
            params: params,
            return_type: ty,
            body: body,
        };
        Item::Function(function)
    }
    /*#[derive(Debug, Clone)]
    pub struct FunctionDecl {
        pub name: String,
        pub params: Vec<Param>,
        pub return_type: Type,
        pub body: Vec<Stmt>,
    }*/

    pub fn parse_Stmt(&mut self) -> Stmt {
        let x: Stmt = match self.nextP() {
            TokenParseItem {
                tokens: TokenExpr::Key(Token::Type(x)),
                locations: y,
            } => self.parse_Stmt_Assignment(Some(x)),
            TokenParseItem {
                tokens: TokenExpr::Key(Token::Let),
                locations: y,
            } => match self.next() {
                TokenExpr::Key(Token::Type(x)) => self.parse_Stmt_Assignment(Some(x)),
                x => errorHandle(
                    format!("must provide a type after a let statement {:?}", x),
                    self,
                ),
            },
            TokenParseItem {
                tokens: TokenExpr::Key(Token::Return),
                locations: y,
            } => self.parse_Stmt_Return(),
            TokenParseItem {
                tokens: TokenExpr::Key(Token::If),
                locations: y,
            } => self.parse_Stmt_If(),
            TokenParseItem {
                tokens: TokenExpr::Key(Token::While),
                locations: y,
            } => self.parse_Stmt_While(),
            TokenParseItem {
                tokens: TokenExpr::Key(Token::Break),
                locations: y,
            } => self.parse_Stmt_Break(),
            TokenParseItem {
                tokens: TokenExpr::Key(Token::Continue),
                locations: y,
            } => self.parse_Stmt_Continue(),
            x => self.parse_Stmt_Assignment_Short(x),
        };
        x
    }

    /*
    #[derive(Debug, Clone)]
    pub enum Stmt {
        Let(AssignmentStmt), done
        Expr(Expr), done
        Return(Option<Expr>), done
        If(IfStmt), done
        While(WhileStmt), done
        Break, done
        Continue, done
    }
    */

    pub fn parse_Stmt_Assignment_Short(&mut self, first: TokenParseItem) -> Stmt {
        match (&first.tokens, self.peek()) {
            (_, TokenExpr::Key(Token::Equl))
            | (TokenExpr::Op(Token::Star), _)
            | (TokenExpr::Operand(Token::Identifier(_)), TokenExpr::Op(Token::LBracket)) => {
                self.push(first);
                self.parse_Stmt_Assignment(None)
            }
            _ => self.parse_Stmt_Expr(first),
        }
    }
    pub fn parse_Stmt_Expr(&mut self, first: TokenParseItem) -> Stmt {
        self.push(first);
        let expr = Expr::parse_expression(self, 0.0);
        if self.peek() != TokenExpr::Key(Token::Semicolon) {
            errorHandle(
                format!(
                    "all statements must be completed with a Semicolon {:?}",
                    self.peek()
                ),
                self,
            );
        }
        self.next();
        Stmt::Expr(expr)
    }

    pub fn parse_Stmt_Break(&mut self) -> Stmt {
        if self.peek() != TokenExpr::Key(Token::Semicolon) {
            errorHandle(
                format!(
                    "all statements must be completed with a Semicolon {:?}",
                    self.peek()
                ),
                self,
            );
        }
        self.next();
        Stmt::Break
    }
    pub fn parse_Stmt_Continue(&mut self) -> Stmt {
        if self.peek() != TokenExpr::Key(Token::Semicolon) {
            errorHandle(
                format!(
                    "all statements must be completed with a Semicolon {:?}",
                    self.peek()
                ),
                self,
            );
        }
        self.next();
        Stmt::Continue
    }

    pub fn parse_Stmt_While(&mut self) -> Stmt {
        let condition: Expr = Expr::parse_expression(self, 0.0);
        assert_eq!(
            self.peek(),
            TokenExpr::Key(Token::LBrace),
            "all while statements must be opened with a left brace. {:?} ",
            self.peek()
        );
        self.next();

        let mut body: Vec<Stmt> = Vec::new();
        let mut cur_Stmt: Stmt;

        //parse body of function
        while self.peek() != TokenExpr::Key(Token::RBrace) {
            cur_Stmt = self.parse_Stmt();
            body.push(cur_Stmt);
        }

        assert_eq!(
            self.peek(),
            TokenExpr::Key(Token::RBrace),
            "all while statements must be closed with a right brace. {:?} ",
            self.peek()
        );
        self.next();

        Stmt::While(WhileStmt {
            condition: condition,
            body: body,
        })
    }

    /*#[derive(Debug, Clone)]
    pub struct WhileStmt {
        pub condition: Expr,
        pub body: Vec<Stmt>,
    }*/

    pub fn parse_Stmt_If(&mut self) -> Stmt {
        let condition: Expr = Expr::parse_expression(self, 0.0);
        assert_eq!(
            self.peek(),
            TokenExpr::Key(Token::LBrace),
            "all if statements must be opened with a left brace. {:?} ",
            self.peek()
        );
        self.next();

        let mut body: Vec<Stmt> = Vec::new();
        let mut cur_Stmt: Stmt;

        //parse body of function
        while self.peek() != TokenExpr::Key(Token::RBrace) {
            cur_Stmt = self.parse_Stmt();
            body.push(cur_Stmt);
        }

        assert_eq!(
            self.peek(),
            TokenExpr::Key(Token::RBrace),
            "all if statements must be closed with a right brace. {:?} ",
            self.peek()
        );
        self.next();

        let body_else: Option<Vec<Stmt>> = match self.peek() {
            TokenExpr::Key(Token::Else) => {
                self.next();
                assert_eq!(
                    self.peek(),
                    TokenExpr::Key(Token::LBrace),
                    "all if and else statements must be opened with a left brace. {:?} ",
                    self.peek()
                );
                self.next();
                let mut curr_else_body: Vec<Stmt> = Vec::new();
                let mut cur_Stmt: Stmt;

                while self.peek() != TokenExpr::Key(Token::RBrace) {
                    cur_Stmt = self.parse_Stmt();
                    curr_else_body.push(cur_Stmt);
                }

                assert_eq!(
                    self.peek(),
                    TokenExpr::Key(Token::RBrace),
                    "all if statements must be closed with a right brace. {:?} ",
                    self.peek()
                );
                self.next();
                Some(curr_else_body)
            }
            _ => None,
        };

        Stmt::If(IfStmt {
            condition: condition,
            then_branch: body,
            else_branch: body_else,
        })
    }

    /*#[derive(Debug, Clone)]
    pub struct IfStmt {
        pub condition: Expr,
        pub then_branch: Vec<Stmt>,
        pub else_branch: Option<Vec<Stmt>>,
    }
    */

    pub fn parse_Stmt_Return(&mut self) -> Stmt {
        let value: Option<Expr> = match self.peek() {
            TokenExpr::Key(Token::Semicolon) => None,
            _ => Some(Expr::parse_expression(self, 0.0)),
        };
        if self.peek() != TokenExpr::Key(Token::Semicolon) {
            errorHandle(
                format!(
                    "all statements must be completed with a Semicolon {:?}",
                    self.peek()
                ),
                self,
            );
        }
        self.next();
        Stmt::Return(value)
    }

    pub fn parse_Stmt_Assignment(&mut self, typ: Option<Type>) -> Stmt {
        let mut ty = typ;
        if !ty.is_none() {
            ty = match self.next() {
                TokenExpr::Key(Token::Colon) => Some(ty.unwrap()),
                TokenExpr::Op(Token::LBracket) => {
                    let x = match self.next() {
                        TokenExpr::Operand(Token::Int(len)) => {
                            Some(Type::Array(Box::new(ty.unwrap()), len))
                        }
                        t => errorHandle(
                            format!("cannot deffine an array with a non integer length. {:?}", t),
                            self,
                        ),
                    };
                    self.next();
                    let next = self.next();
                    if next != TokenExpr::Key(Token::Colon) {
                        errorHandle(
                            format!(
                                "unexpected token: there must be a colon between the assinments type and name: {:?}",
                                next
                            ),
                            self,
                        )
                    }
                    x
                }
                _ => errorHandle(format!("varbles must have valid types"), self),
            };
        }

        let mut pointer = false;
        let mut dereference = false;
        let name = match self.next() {
            TokenExpr::Operand(Token::Identifier(name)) => name,
            TokenExpr::Op(Token::Star) => {
                if ty.is_some() {
                    pointer = true;
                } else {
                    dereference = true;
                }

                match self.next() {
                    TokenExpr::Operand(Token::Identifier(name)) => name,
                    x => errorHandle(
                        format!("varbles must have valid alphanumeric names {:?}", x),
                        self,
                    ),
                }
            }
            x => errorHandle(
                format!("varbles must have valid alphanumeric names {:?}", x),
                self,
            ),
        };

        let index = if self.peek() == TokenExpr::Op(Token::LBracket) {
            self.next();
            let index_expr = Expr::parse_expression(self, 0.0);
            if self.peek() != TokenExpr::Op(Token::RBracket) {
                errorHandle(
                    format!("index must close with ']' got {:?}", self.peek()),
                    self,
                );
            }
            self.next();
            Some(index_expr)
        } else {
            None
        };

        let value: Option<Expr>;
        let next: TokenExpr = self.next();
        if next == TokenExpr::Key(Token::Equl) {
            value = match self.peek() {
                TokenExpr::Key(Token::Semicolon) => None,
                _ => Some(Expr::parse_expression(self, 0.0)),
            };
            if self.peek() != TokenExpr::Key(Token::Semicolon) {
                errorHandle(
                    format!(
                        "all statements must be completed with a Semicolon {:?}",
                        self.peek()
                    ),
                    self,
                );
            }
            self.next();
        } else if next == TokenExpr::Key(Token::Semicolon) {
            value = None;
        } else {
            errorHandle(
                format!("Assingments must end with Semicolons {:?}", next),
                self,
            )
        }
        let mut assinment = AssignmentStmt {
            name: name,
            ty: ty,
            value: value,
            pointer: pointer,
            dereference: dereference,
            index: index,
        };
        if pointer {
            assinment.ty = Some(Type::Pointer(Box::new(
                assinment.ty.expect("cannot have a void pointer"),
            )));
        }
        Stmt::Assignment(assinment)
    }
    /*    #[derive(Debug, Clone)]
    pub struct AssignmentStmt {
        pub name: String,
        pub ty: Option<Type>,
        pub value: Option<Expr>,
    }*/

    pub fn global_assignment_parse(&mut self, typ: Type) -> Item {
        let mut ty = typ;
        ty = match self.next() {
            TokenExpr::Key(Token::Colon) => ty,
            TokenExpr::Op(Token::LBracket) => {
                let x = match self.next() {
                    TokenExpr::Operand(Token::Int(len)) => Type::Array(Box::new(ty), len),
                    t => errorHandle(
                        format!("cannot deffine an array with a non integer length. {:?}", t),
                        self,
                    ),
                };
                self.next();
                let next = self.next();
                if next != TokenExpr::Key(Token::Colon) {
                    errorHandle(
                        format!(
                            "unexpected token: there must be a colon between the assinments type and name: {:?}",
                            next
                        ),
                        self,
                    )
                }
                x
            }
            _ => errorHandle(format!("varbles must have valid types"), self),
        };

        let mut pointer = false;
        let name = match self.next() {
            TokenExpr::Operand(Token::Identifier(name)) => name,
            TokenExpr::Op(Token::Star) => {
                pointer = true;
                match self.next() {
                    TokenExpr::Operand(Token::Identifier(name)) => name,
                    x => errorHandle(
                        format!("varbles must have valid alphanumeric names {:?}", x),
                        self,
                    ),
                }
            }
            x => errorHandle(
                format!("varbles must have valid alphanumeric names {:?}", x),
                self,
            ),
        };

        let value: Option<Expr>;
        let next: TokenExpr = self.next();
        if next == TokenExpr::Key(Token::Equl) {
            value = match self.peek() {
                TokenExpr::Key(Token::Semicolon) => None,
                _ => Some(Expr::parse_expression(self, 0.0)),
            };
            if self.peek() != TokenExpr::Key(Token::Semicolon) {
                errorHandle(
                    format!(
                        "all statements must be completed with a Semicolon {:?}",
                        self.peek()
                    ),
                    self,
                );
            }
            self.next();
        } else if next == TokenExpr::Key(Token::Semicolon) {
            value = None;
        } else {
            errorHandle(format!("Global assingments must end with Semicolons"), self)
        }

        let assinment = GlobalDecl {
            name: name,
            ty: ty,
            value: value,
            pointer: pointer,
        };
        Item::Global(assinment)
    }

    /*#[derive(Debug, Clone)]
    pub struct GlobalDecl {
        pub name: String,
        pub ty: Type,
        pub value: Option<Expr>,
        pub pointer: Bool,
    }*/
}

impl TokenExpr {
    pub fn convert_from_token(tok: Token) -> Self {
        match tok {
            // operands / values
            Token::Int(_)
            | Token::Float(_)
            | Token::Bool(_)
            | Token::String(_)
            | Token::Identifier(_)
            | Token::Array(_)
            | Token::Call(_, _) => TokenExpr::Operand(tok),

            // operators
            Token::Plus
            | Token::Minus
            | Token::Star
            | Token::Slash
            | Token::Percent
            | Token::And
            | Token::Or
            | Token::EqualEqual
            | Token::NotEqual
            | Token::Less
            | Token::LessEqual
            | Token::Greater
            | Token::GreaterEqual
            | Token::Not
            | Token::Ampersand
            | Token::Comma
            | Token::LParen
            | Token::RParen
            | Token::LBracket
            | Token::RBracket => TokenExpr::Op(tok),

            Token::LBrace
            | Token::RBrace
            | Token::Colon
            | Token::Equl
            | Token::Semicolon
            | Token::Type(_)
            | Token::If
            | Token::Else
            | Token::For
            | Token::While
            | Token::Return
            | Token::Const
            | Token::Let
            | Token::Continue
            | Token::Break
            | Token::SizeOf
            | Token::Fn => TokenExpr::Key(tok),

            Token::EOF => TokenExpr::Eof,
        }
    }
}

impl Expr {
    pub fn parse_expression(lexer: &mut TokenParse, min_bp: f32) -> Expr {
        let mut lhs = match lexer.next() {
            TokenExpr::Operand(it) => Expr::Operand(it),

            TokenExpr::Op(Token::Minus) => {
                Expr::Operation(Token::Minus, vec![Self::parse_expression(lexer, 10.0)])
            }

            TokenExpr::Op(Token::Star) => {
                Expr::Operation(Token::Star, vec![Self::parse_expression(lexer, 10.0)])
            }

            TokenExpr::Op(Token::Ampersand) => {
                Expr::Operation(Token::Ampersand, vec![Self::parse_expression(lexer, 10.0)])
            }

            TokenExpr::Op(Token::Not) => {
                Expr::Operation(Token::Not, vec![Self::parse_expression(lexer, 10.0)])
            }

            TokenExpr::Key(Token::SizeOf) => {
                if lexer.peek() != TokenExpr::Op(Token::LParen) {
                    errorHandle(
                        format!("sizeof must be followed by '(' got {:?}", lexer.peek()),
                        lexer,
                    );
                }
                lexer.next();

                let mut ty = match lexer.next() {
                    TokenExpr::Key(Token::Type(ty)) => ty,
                    t => errorHandle(format!("sizeof expects a type, got {:?}", t), lexer),
                };

                if lexer.peek() == TokenExpr::Op(Token::LBracket) {
                    lexer.next();
                    ty = match lexer.next() {
                        TokenExpr::Operand(Token::Int(len)) => Type::Array(Box::new(ty), len),
                        t => errorHandle(
                            format!("cannot use a non integer array length in sizeof: {:?}", t),
                            lexer,
                        ),
                    };
                    if lexer.peek() != TokenExpr::Op(Token::RBracket) {
                        errorHandle(
                            format!(
                                "sizeof array type must close with ']' got {:?}",
                                lexer.peek()
                            ),
                            lexer,
                        );
                    }
                    lexer.next();
                }

                if lexer.peek() != TokenExpr::Op(Token::RParen) {
                    errorHandle(
                        format!("sizeof must close with ')' got {:?}", lexer.peek()),
                        lexer,
                    );
                }
                lexer.next();

                Expr::SizeOf(ty)
            }

            TokenExpr::Op(Token::LBracket) => {
                let expr = Self::parse_expression(lexer, 0.0);
                match lexer.peek() {
                    TokenExpr::Op(Token::RBracket) => lexer.next(),
                    _ => errorHandle(
                        format!("array does not close properly {:?}", lexer.peek()),
                        lexer,
                    ),
                };
                let array = match expr {
                    Expr::Operation(Token::Comma, x) => x,
                    _ => errorHandle(
                        format!("not a properly errorHandle(formated array {:?}", expr),
                        lexer,
                    ),
                };
                Expr::Operand(Token::Array(Box::new(array)))
            }

            TokenExpr::Op(Token::LParen) => {
                let expr = Self::parse_expression(lexer, 0.0);
                match lexer.peek() {
                    TokenExpr::Op(Token::RParen) => {
                        lexer.next();
                    }
                    _ => {}
                };
                Expr::Operation(Token::LParen, vec![expr])
            }

            t => errorHandle(format!("bad token 1: {:?}       {:?}", t, lexer), lexer),
        };

        loop {
            let op = match lexer.peek() {
                TokenExpr::Eof => break,
                TokenExpr::Op(Token::RParen) => break,
                TokenExpr::Op(Token::RBracket) => break,
                TokenExpr::Key(_) => break,
                TokenExpr::Op(op) => op,
                t => errorHandle(format!("bad token 2: {:?}", t), lexer),
            };

            let (l_bp, r_bp) = Token::infix_binding_power(op.clone());

            if l_bp < min_bp {
                break;
            }

            lexer.next();

            if op == Token::Comma {
                let mut args = vec![];
                loop {
                    let expr = Self::parse_expression(lexer, 0.0);
                    match expr {
                        Expr::Operation(Token::Comma, x) => args.extend(x),
                        expr => args.push(expr),
                    }

                    if lexer.peek() == TokenExpr::Op(Token::Comma) {
                        lexer.next();
                    } else {
                        break;
                    }
                }

                let mut vals = vec![lhs];
                vals.extend(args);

                lhs = Expr::Operation(op.clone(), vals);

                continue;
            } else if op == Token::LBracket {
                let index = Self::parse_expression(lexer, 0.0);
                if lexer.peek() != TokenExpr::Op(Token::RBracket) {
                    errorHandle(
                        format!("index must close with ']' got {:?}", lexer.peek()),
                        lexer,
                    );
                }
                lexer.next();
                lhs = Expr::Operation(Token::LBracket, vec![lhs, index]);
                continue;
            } else if op == Token::LParen {
                let name_t = match lhs {
                    Expr::Operand(x) => x,
                    _ => errorHandle(format!("functions must have valid names {:?}", lhs), lexer),
                };
                let mut array = vec![];
                if lexer.peek() != TokenExpr::Op(Token::RParen) {
                    array = match Self::parse_expression(lexer, 0.0) {
                        Expr::Operation(Token::Comma, x) => x,
                        expr => vec![expr],
                    };
                }
                let name = match name_t {
                    Token::Identifier(x) => x,
                    _ => errorHandle(
                        format!("functions must have valid Identifier names {:?}", name_t),
                        lexer,
                    ),
                };
                lhs = Expr::Operand(Token::Call(name, Box::new(array)));
                match lexer.peek() {
                    TokenExpr::Op(Token::RParen) => {
                        lexer.next();
                    }
                    _ => errorHandle(
                        format!(
                            "functions calls must be closed with perenthisis {:?}",
                            lexer.peek()
                        ),
                        lexer,
                    ),
                };

                continue;
            }

            let rhs = Self::parse_expression(lexer, r_bp);
            lhs = Expr::Operation(op, vec![lhs, rhs]);
        }

        lhs
    }
}
