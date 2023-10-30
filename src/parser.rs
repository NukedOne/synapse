use crate::tokenizer::{Token, TokenKind};
use std::collections::VecDeque;

pub struct Parser<'source> {
    current: Option<Token<'source>>,
    previous: Option<Token<'source>>,
    tokens: VecDeque<Token<'source>>,
}

impl<'source> Parser<'source> {
    pub fn new() -> Self {
        Parser {
            current: None,
            previous: None,
            tokens: VecDeque::new(),
        }
    }

    pub fn parse(&mut self, tokens: VecDeque<Token<'source>>) -> Vec<Statement<'source>> {
        self.tokens = tokens;
        self.advance();
        let mut statements = vec![];
        while self.current.is_some() {
            statements.push(self.parse_statement());
        }
        statements
    }

    fn is_next(&mut self, tokens: &[TokenKind]) -> bool {
        for token in tokens {
            if self.check(*token) {
                self.advance();
                return true;
            }
        }
        false
    }

    fn check(&self, kind: TokenKind) -> bool {
        self.current.unwrap().kind == kind
    }

    fn advance(&mut self) -> Option<Token<'source>> {
        self.previous = self.current;
        self.current = self.tokens.pop_front();
        self.previous
    }

    fn consume(&mut self, kind: TokenKind) -> Option<Token<'source>> {
        if self.check(kind) {
            return self.advance();
        }
        None
    }

    fn parse_statement(&mut self) -> Statement<'source> {
        if self.is_next(&[TokenKind::Print]) {
            self.parse_print_statement()
        } else if self.is_next(&[TokenKind::Fn]) {
            self.parse_fn_statement()
        } else if self.is_next(&[TokenKind::Return]) {
            self.parse_return_statement()
        } else if self.is_next(&[TokenKind::If]) {
            self.parse_if_statement()
        } else if self.is_next(&[TokenKind::LeftBrace]) {
            self.parse_block_statement()
        } else {
            self.parse_expression_statement()
        }
    }

    fn parse_print_statement(&mut self) -> Statement<'source> {
        let expression = self.parse_expression();
        self.consume(TokenKind::Semicolon);
        Statement::Print(PrintStatement { expression })
    }

    fn parse_fn_statement(&mut self) -> Statement<'source> {
        let name = self.consume(TokenKind::Identifier).unwrap();
        self.consume(TokenKind::LeftParen);
        let mut arguments = vec![];
        while !self.is_next(&[TokenKind::RightParen]) {
            let arg = self.consume(TokenKind::Identifier).unwrap();
            self.consume(TokenKind::Comma);
            arguments.push(arg.value);
        }
        self.consume(TokenKind::LeftBrace);
        let body = self.parse_block_statement();
        Statement::Fn(FnStatement {
            name: name.value,
            arguments,
            body: body.into(),
        })
    }

    fn parse_return_statement(&mut self) -> Statement<'source> {
        let expression = self.parse_expression();
        self.consume(TokenKind::Semicolon);
        Statement::Return(ReturnStatement { expression })
    }

    fn parse_if_statement(&mut self) -> Statement<'source> {
        self.consume(TokenKind::LeftParen);
        let condition = self.parse_expression();
        self.consume(TokenKind::RightParen);
        let if_branch = self.parse_statement();
        let else_branch: Statement = if self.is_next(&[TokenKind::Else]) {
            self.parse_statement()
        } else {
            Statement::Dummy
        };
        Statement::If(IfStatement {
            condition,
            if_branch: Box::new(if_branch),
            else_branch: Box::new(else_branch),
        })
    }

    fn parse_block_statement(&mut self) -> Statement<'source> {
        let mut body = vec![];
        while !self.is_next(&[TokenKind::RightBrace]) {
            body.push(self.parse_statement());
        }
        Statement::Block(BlockStatement { body })
    }

    fn parse_expression_statement(&mut self) -> Statement<'source> {
        let expr = self.parse_expression();
        self.consume(TokenKind::Semicolon);
        Statement::Expression(ExpressionStatement { expression: expr })
    }

    fn parse_expression(&mut self) -> Expression<'source> {
        self.equality()
    }

    fn equality(&mut self) -> Expression<'source> {
        let mut result = self.relational();
        while self.is_next(&[TokenKind::DoubleEqual, TokenKind::BangEqual]) {
            let negation = match self.previous.unwrap().kind {
                TokenKind::BangEqual => true,
                TokenKind::DoubleEqual => false,
                _ => unreachable!(),
            };
            result = Expression::Binary(BinaryExpression {
                kind: BinaryExpressionKind::Equality(negation),
                lhs: Box::new(result),
                rhs: Box::new(self.relational()),
            });
        }
        result
    }

    fn relational(&mut self) -> Expression<'source> {
        let mut result = self.term();
        while self.is_next(&[
            TokenKind::Less,
            TokenKind::Greater,
            TokenKind::LessEqual,
            TokenKind::GreaterEqual,
        ]) {
            let kind = match self.previous {
                Some(token) => match token.kind {
                    TokenKind::Less => BinaryExpressionKind::Less,
                    TokenKind::Greater => BinaryExpressionKind::Greater,
                    TokenKind::LessEqual => BinaryExpressionKind::LessEqual,
                    TokenKind::GreaterEqual => BinaryExpressionKind::GreaterEqual,
                    _ => unreachable!(),
                },
                None => unreachable!(),
            };
            result = Expression::Binary(BinaryExpression {
                kind,
                lhs: Box::new(result),
                rhs: Box::new(self.term()),
            });
        }
        result
    }

    fn term(&mut self) -> Expression<'source> {
        let mut result = self.factor();
        while self.is_next(&[TokenKind::Plus, TokenKind::Minus]) {
            let kind = match self.previous {
                Some(token) => match token.kind {
                    TokenKind::Plus => BinaryExpressionKind::Add,
                    TokenKind::Minus => BinaryExpressionKind::Sub,
                    _ => unreachable!(),
                },
                None => unreachable!(),
            };
            result = Expression::Binary(BinaryExpression {
                kind,
                lhs: Box::new(result),
                rhs: Box::new(self.factor()),
            });
        }
        result
    }

    fn factor(&mut self) -> Expression<'source> {
        let mut result = self.call();
        while self.is_next(&[TokenKind::Star, TokenKind::Slash]) {
            let kind = match self.previous {
                Some(token) => match token.kind {
                    TokenKind::Star => BinaryExpressionKind::Mul,
                    TokenKind::Slash => BinaryExpressionKind::Div,
                    _ => unreachable!(),
                },
                None => unreachable!(),
            };
            result = Expression::Binary(BinaryExpression {
                kind,
                lhs: Box::new(result),
                rhs: Box::new(self.call()),
            });
        }
        result
    }

    fn call(&mut self) -> Expression<'source> {
        let mut expr = self.primary();
        if self.is_next(&[TokenKind::LeftParen]) {
            let mut arguments = vec![];
            if !self.check(TokenKind::RightParen) {
                loop {
                    arguments.push(self.parse_expression());
                    if !self.is_next(&[TokenKind::Comma]) {
                        break;
                    }
                }
            }
            self.consume(TokenKind::RightParen);
            let name = match expr {
                Expression::Variable(v) => v.value,
                _ => unimplemented!(),
            };
            expr = Expression::Call(CallExpression {
                variable: name,
                arguments,
            });
        }
        expr
    }

    fn primary(&mut self) -> Expression<'source> {
        if self.is_next(&[TokenKind::Number]) {
            let n = self.previous.unwrap().value.parse().unwrap();
            Expression::Literal(LiteralExpression {
                value: Literal::Num(n),
            })
        } else if self.is_next(&[TokenKind::True, TokenKind::False, TokenKind::Null]) {
            let literal: Literal = self
                .previous
                .unwrap()
                .value
                .parse()
                .expect("Failed to parse a literal.");
            Expression::Literal(LiteralExpression { value: literal })
        } else if self.is_next(&[TokenKind::Identifier]) {
            let var = self.previous.unwrap().value;
            Expression::Variable(VariableExpression { value: var })
        } else if self.is_next(&[TokenKind::String]) {
            let string = self.previous.unwrap().value;
            Expression::Literal(LiteralExpression {
                value: Literal::String(string),
            })
        } else {
            println!("{:?}", self.current);
            todo!();
        }
    }
}

impl<'source> Default for Parser<'source> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub enum Statement<'source> {
    Print(PrintStatement<'source>),
    Fn(FnStatement<'source>),
    Return(ReturnStatement<'source>),
    If(IfStatement<'source>),
    Block(BlockStatement<'source>),
    Expression(ExpressionStatement<'source>),
    Dummy,
}

#[derive(Debug)]
pub struct PrintStatement<'source> {
    pub expression: Expression<'source>,
}

#[derive(Debug)]
pub struct FnStatement<'source> {
    pub name: &'source str,
    pub arguments: Vec<&'source str>,
    pub body: Box<Statement<'source>>,
}

#[derive(Debug)]
pub struct ReturnStatement<'source> {
    pub expression: Expression<'source>,
}

#[derive(Debug)]
pub struct IfStatement<'source> {
    pub condition: Expression<'source>,
    pub if_branch: Box<Statement<'source>>,
    pub else_branch: Box<Statement<'source>>,
}

#[derive(Debug)]
pub struct BlockStatement<'source> {
    pub body: Vec<Statement<'source>>,
}

#[derive(Debug)]
pub struct ExpressionStatement<'source> {
    pub expression: Expression<'source>,
}

#[derive(Debug)]
pub enum Expression<'source> {
    Literal(LiteralExpression<'source>),
    Variable(VariableExpression<'source>),
    Binary(BinaryExpression<'source>),
    Call(CallExpression<'source>),
}

#[derive(Debug)]
pub struct LiteralExpression<'source> {
    pub value: Literal<'source>,
}

#[derive(Debug)]
pub struct VariableExpression<'source> {
    pub value: &'source str,
}

#[derive(Debug)]
pub struct BinaryExpression<'source> {
    pub kind: BinaryExpressionKind,
    pub lhs: Box<Expression<'source>>,
    pub rhs: Box<Expression<'source>>,
}

#[derive(Debug)]
pub struct CallExpression<'source> {
    pub variable: &'source str,
    pub arguments: Vec<Expression<'source>>,
}

#[derive(Debug)]
pub enum BinaryExpressionKind {
    Add,
    Sub,
    Mul,
    Div,
    Equality(bool), /* negation */
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum Literal<'source> {
    Num(f64),
    String(&'source str),
    Bool(bool),
    Null,
}

impl<'source> std::str::FromStr for Literal<'source> {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "true" => Ok(Literal::Bool(true)),
            "false" => Ok(Literal::Bool(false)),
            "null" => Ok(Literal::Null),
            _ => Err(format!("{} is not a valid object literal", s)),
        }
    }
}
