use crate::tokenizer::Token;
use anyhow::{bail, Result};
use std::collections::VecDeque;

pub struct Parser<'src> {
    current: Option<Token<'src>>,
    previous: Option<Token<'src>>,
    tokens: Option<VecDeque<Token<'src>>>,
}

impl<'src> Parser<'src> {
    pub fn new() -> Self {
        Parser {
            current: None,
            previous: None,
            tokens: None,
        }
    }

    pub fn parse(&mut self, tokens: VecDeque<Token<'src>>) -> Result<Vec<Statement<'src>>> {
        self.tokens = Some(tokens);
        self.advance();
        let mut statements = vec![];
        while self.current.is_some() {
            statements.push(match self.parse_declaration() {
                Ok(stmt) => stmt,
                Err(e) => bail!(e),
            });
        }
        Ok(statements)
    }

    fn is_next(&mut self, tokens: &[Token]) -> bool {
        for token in tokens {
            if self.check(*token) {
                self.advance();
                return true;
            }
        }
        false
    }

    fn check(&self, kind: Token) -> bool {
        std::mem::discriminant(self.current.as_ref().unwrap()) == std::mem::discriminant(&kind)
    }

    fn advance(&mut self) -> Option<Token<'src>> {
        self.previous = self.current;
        self.current = self.tokens.as_mut().and_then(|tokens| tokens.pop_front());
        self.previous
    }

    fn consume(&mut self, kind: Token) -> Option<Token<'src>> {
        if self.check(kind) {
            return self.advance();
        }
        None
    }

    fn parse_declaration(&mut self) -> Result<Statement<'src>> {
        if self.is_next(&[Token::Fn]) {
            self.parse_fn_statement()
        } else if self.is_next(&[Token::Struct]) {
            self.parse_struct_statement()
        } else if self.is_next(&[Token::Impl]) {
            self.parse_impl_statement()
        } else if self.is_next(&[Token::Use]) {
            self.parse_use_statement()
        } else {
            bail!("parser: expected a declaration (like 'fn' or 'struct')");
        }
    }

    fn parse_statement(&mut self) -> Result<Statement<'src>> {
        if self.is_next(&[Token::Print]) {
            self.parse_print_statement()
        } else if self.is_next(&[Token::Return]) {
            self.parse_return_statement()
        } else if self.is_next(&[Token::If]) {
            self.parse_if_statement()
        } else if self.is_next(&[Token::While]) {
            self.parse_while_statement()
        } else if self.is_next(&[Token::For]) {
            self.parse_for_statement()
        } else if self.is_next(&[Token::Break]) {
            self.parse_break_statement()
        } else if self.is_next(&[Token::Continue]) {
            self.parse_continue_statement()
        } else if self.is_next(&[Token::LeftBrace]) {
            self.parse_block_statement()
        } else {
            self.parse_expression_statement()
        }
    }

    fn parse_print_statement(&mut self) -> Result<Statement<'src>> {
        let expression = self.parse_expression()?;
        self.consume(Token::Semicolon);
        Ok(Statement::Print(PrintStatement { expression }))
    }

    fn parse_fn_statement(&mut self) -> Result<Statement<'src>> {
        let name = self.consume(Token::Identifier("")).unwrap();
        self.consume(Token::LeftParen);
        let mut arguments = vec![];
        while !self.is_next(&[Token::RightParen]) {
            let arg = self.consume(Token::Identifier("")).unwrap();
            self.consume(Token::Comma);
            arguments.push(arg);
        }
        self.consume(Token::LeftBrace);
        let body = self.parse_block_statement()?;
        Ok(Statement::Fn(FnStatement {
            name,
            arguments,
            body: body.into(),
        }))
    }

    fn parse_return_statement(&mut self) -> Result<Statement<'src>> {
        let expression = self.parse_expression()?;
        self.consume(Token::Semicolon);
        Ok(Statement::Return(ReturnStatement { expression }))
    }

    fn parse_if_statement(&mut self) -> Result<Statement<'src>> {
        self.consume(Token::LeftParen);
        let condition = self.parse_expression()?;
        self.consume(Token::RightParen);
        let if_branch = self.parse_statement()?;
        let else_branch: Statement = if self.is_next(&[Token::Else]) {
            self.parse_statement()?
        } else {
            Statement::Dummy
        };
        Ok(Statement::If(IfStatement {
            condition,
            if_branch: if_branch.into(),
            else_branch: else_branch.into(),
        }))
    }

    fn parse_while_statement(&mut self) -> Result<Statement<'src>> {
        self.consume(Token::LeftParen);
        let condition = self.parse_expression()?;
        self.consume(Token::RightParen);
        let body = self.parse_statement()?;
        Ok(Statement::While(WhileStatement {
            condition,
            body: body.into(),
        }))
    }

    fn parse_for_statement(&mut self) -> Result<Statement<'src>> {
        self.consume(Token::LeftParen);
        let initializer = self.parse_expression()?;
        self.consume(Token::Semicolon);
        let condition = self.parse_expression()?;
        self.consume(Token::Semicolon);
        let advancement = self.parse_expression()?;
        self.consume(Token::RightParen);
        let body = self.parse_statement()?;
        Ok(Statement::For(ForStatement {
            initializer,
            condition,
            advancement,
            body: body.into(),
        }))
    }

    fn parse_break_statement(&mut self) -> Result<Statement<'src>> {
        self.consume(Token::Semicolon);
        Ok(Statement::Break(BreakStatement {}))
    }

    fn parse_continue_statement(&mut self) -> Result<Statement<'src>> {
        self.consume(Token::Semicolon);
        Ok(Statement::Continue(ContinueStatement {}))
    }

    fn parse_struct_statement(&mut self) -> Result<Statement<'src>> {
        let name = match self.consume(Token::Identifier("")) {
            Some(Token::Identifier(ident)) => ident,
            Some(_) | None => bail!(
                "parser: expected identifier after 'struct' keyword, got: {}",
                self.current.unwrap().get_value()
            ),
        };
        self.consume(Token::LeftBrace);
        let mut members = vec![];
        while !self.is_next(&[Token::RightBrace]) {
            members.push(match self.parse_struct_member() {
                Ok(member) => member,
                Err(e) => bail!(e),
            });
        }
        Ok(Statement::Struct(StructStatement { name, members }))
    }

    fn parse_struct_member(&mut self) -> Result<&'src str> {
        let member = match self.consume(Token::Identifier("")) {
            Some(token) => token.get_value(),
            None => bail!("parser: structs should be declared as: `struct s {{ x, y, z, }}`"),
        };
        self.consume(Token::Comma);
        Ok(member)
    }

    fn parse_impl_statement(&mut self) -> Result<Statement<'src>> {
        let name = match self.consume(Token::Identifier("")) {
            Some(Token::Identifier(ident)) => ident,
            Some(_) | None => bail!(
                "parser: expected identifier after 'impl' keyword, got: {}",
                self.current.unwrap().get_value()
            ),
        };
        self.consume(Token::LeftBrace);
        let mut methods = vec![];
        while !self.is_next(&[Token::RightBrace]) {
            methods.push(match self.parse_declaration() {
                Ok(method) => method,
                Err(e) => bail!(e),
            });
        }

        Ok(Statement::Impl(ImplStatement { name, methods }))
    }

    fn parse_use_statement(&mut self) -> Result<Statement<'src>> {
        let module = match self.consume(Token::String("")) {
            Some(Token::String(string)) => string,
            Some(_) | None => bail!("parser: expected module after use"),
        };
        self.consume(Token::Semicolon);
        Ok(Statement::Use(UseStatement { module }))
    }

    fn parse_block_statement(&mut self) -> Result<Statement<'src>> {
        let mut body = vec![];
        while !self.is_next(&[Token::RightBrace]) {
            body.push(self.parse_statement()?);
        }
        Ok(Statement::Block(BlockStatement { body }))
    }

    fn parse_expression_statement(&mut self) -> Result<Statement<'src>> {
        let expr = self.parse_expression()?;
        self.consume(Token::Semicolon);
        Ok(Statement::Expression(ExpressionStatement {
            expression: expr,
        }))
    }

    fn parse_expression(&mut self) -> Result<Expression<'src>> {
        self.assignment()
    }

    fn assignment(&mut self) -> Result<Expression<'src>> {
        let mut result = self.or()?;
        while self.is_next(&[
            Token::Equal,
            Token::PlusEqual,
            Token::MinusEqual,
            Token::StarEqual,
            Token::SlashEqual,
            Token::PercentEqual,
            Token::LessLessEqual,
            Token::GreaterGreaterEqual,
            Token::AmpersandEqual,
            Token::CaretEqual,
            Token::PipeEqual,
        ]) {
            let op = self.previous.unwrap();
            result = Expression::Assign(AssignExpression {
                lhs: result.into(),
                rhs: self.or()?.into(),
                op,
            });
        }
        Ok(result)
    }

    fn or(&mut self) -> Result<Expression<'src>> {
        let mut result = self.and()?;
        while self.is_next(&[Token::DoublePipe]) {
            result = Expression::Logical(LogicalExpression {
                lhs: result.into(),
                rhs: self.and()?.into(),
                op: Token::DoublePipe,
            });
        }
        Ok(result)
    }

    fn and(&mut self) -> Result<Expression<'src>> {
        let mut result = self.bitwise_or()?;
        while self.is_next(&[Token::DoubleAmpersand]) {
            result = Expression::Logical(LogicalExpression {
                lhs: result.into(),
                rhs: self.bitwise_or()?.into(),
                op: Token::DoubleAmpersand,
            });
        }
        Ok(result)
    }

    fn bitwise_or(&mut self) -> Result<Expression<'src>> {
        let mut result = self.bitwise_xor()?;
        while self.is_next(&[Token::Pipe]) {
            result = Expression::Binary(BinaryExpression {
                lhs: result.into(),
                rhs: self.bitwise_xor()?.into(),
                kind: BinaryExpressionKind::BitwiseOr,
            });
        }
        Ok(result)
    }

    fn bitwise_xor(&mut self) -> Result<Expression<'src>> {
        let mut result = self.bitwise_and()?;
        while self.is_next(&[Token::Caret]) {
            result = Expression::Binary(BinaryExpression {
                lhs: result.into(),
                rhs: self.bitwise_and()?.into(),
                kind: BinaryExpressionKind::BitwiseXor,
            });
        }
        Ok(result)
    }

    fn bitwise_and(&mut self) -> Result<Expression<'src>> {
        let mut result = self.equality()?;
        while self.is_next(&[Token::Ampersand]) {
            result = Expression::Binary(BinaryExpression {
                lhs: result.into(),
                rhs: self.equality()?.into(),
                kind: BinaryExpressionKind::BitwiseAnd,
            });
        }
        Ok(result)
    }

    fn equality(&mut self) -> Result<Expression<'src>> {
        let mut result = self.relational()?;
        while self.is_next(&[Token::DoubleEqual, Token::BangEqual]) {
            let negation = match self.previous.unwrap() {
                Token::BangEqual => true,
                Token::DoubleEqual => false,
                _ => unreachable!(),
            };
            result = Expression::Binary(BinaryExpression {
                kind: BinaryExpressionKind::Equality(negation),
                lhs: result.into(),
                rhs: self.relational()?.into(),
            });
        }
        Ok(result)
    }

    fn relational(&mut self) -> Result<Expression<'src>> {
        let mut result = self.bitwise_shift()?;
        while self.is_next(&[
            Token::Less,
            Token::Greater,
            Token::LessEqual,
            Token::GreaterEqual,
        ]) {
            let kind = match self.previous {
                Some(token) => match token {
                    Token::Less => BinaryExpressionKind::Less,
                    Token::Greater => BinaryExpressionKind::Greater,
                    Token::LessEqual => BinaryExpressionKind::LessEqual,
                    Token::GreaterEqual => BinaryExpressionKind::GreaterEqual,
                    _ => unreachable!(),
                },
                None => unreachable!(),
            };
            result = Expression::Binary(BinaryExpression {
                kind,
                lhs: result.into(),
                rhs: self.bitwise_shift()?.into(),
            });
        }
        Ok(result)
    }

    fn bitwise_shift(&mut self) -> Result<Expression<'src>> {
        let mut result = self.term()?;
        while self.is_next(&[Token::GreaterGreater, Token::LessLess]) {
            let kind = match self.previous {
                Some(token) => match token {
                    Token::GreaterGreater => BinaryExpressionKind::BitwiseShr,
                    Token::LessLess => BinaryExpressionKind::BitwiseShl,
                    _ => unreachable!(),
                },
                None => unreachable!(),
            };
            result = Expression::Binary(BinaryExpression {
                kind,
                lhs: result.into(),
                rhs: self.term()?.into(),
            });
        }
        Ok(result)
    }

    fn term(&mut self) -> Result<Expression<'src>> {
        let mut result = self.factor()?;
        while self.is_next(&[Token::Plus, Token::Minus, Token::PlusPlus]) {
            let kind = match self.previous {
                Some(token) => match token {
                    Token::Plus => BinaryExpressionKind::Add,
                    Token::Minus => BinaryExpressionKind::Sub,
                    Token::PlusPlus => BinaryExpressionKind::Strcat,
                    _ => unreachable!(),
                },
                None => unreachable!(),
            };
            result = Expression::Binary(BinaryExpression {
                kind,
                lhs: result.into(),
                rhs: self.factor()?.into(),
            });
        }
        Ok(result)
    }

    fn factor(&mut self) -> Result<Expression<'src>> {
        let mut result = self.unary()?;
        while self.is_next(&[Token::Star, Token::Slash, Token::Percent]) {
            let kind = match self.previous {
                Some(token) => match token {
                    Token::Star => BinaryExpressionKind::Mul,
                    Token::Slash => BinaryExpressionKind::Div,
                    Token::Percent => BinaryExpressionKind::Mod,
                    _ => unreachable!(),
                },
                None => unreachable!(),
            };
            result = Expression::Binary(BinaryExpression {
                kind,
                lhs: result.into(),
                rhs: self.unary()?.into(),
            });
        }
        Ok(result)
    }

    fn unary(&mut self) -> Result<Expression<'src>> {
        if self.is_next(&[
            Token::Minus,
            Token::Bang,
            Token::Ampersand,
            Token::Star,
            Token::Tilde,
        ]) {
            let op = self.previous.unwrap();
            let expr = self.unary()?;
            return Ok(Expression::Unary(UnaryExpression {
                expr: expr.into(),
                op,
            }));
        }
        self.call()
    }

    fn call(&mut self) -> Result<Expression<'src>> {
        let mut expr = self.primary()?;
        loop {
            if self.is_next(&[Token::LeftParen]) {
                let mut arguments = vec![];
                if !self.check(Token::RightParen) {
                    loop {
                        arguments.push(self.parse_expression()?);
                        if !self.is_next(&[Token::Comma]) {
                            break;
                        }
                    }
                }
                self.consume(Token::RightParen);
                expr = Expression::Call(CallExpression {
                    callee: expr.into(),
                    arguments,
                });
            } else if self.is_next(&[Token::Dot, Token::Arrow]) {
                let op = self.previous.unwrap();
                let member = self.consume(Token::Identifier("")).unwrap().get_value();
                expr = Expression::Get(GetExpression {
                    expr: expr.into(),
                    member,
                    op,
                });
            } else if self.is_next(&[Token::LeftBracket]) {
                let index = self.parse_expression()?;
                self.consume(Token::RightBracket);
                expr = Expression::Sub(SubscriptExpression {
                    expr: expr.into(),
                    index: index.into(),
                });
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn primary(&mut self) -> Result<Expression<'src>> {
        if self.is_next(&[Token::Number(""), Token::String("")]) {
            match self.previous.unwrap() {
                Token::Number(n) => self.parse_number(n.parse().unwrap()),
                Token::String(s) => self.parse_string(s),
                _ => unreachable!(),
            }
        } else if self.is_next(&[Token::LeftParen]) {
            self.parse_grouping()
        } else if self.is_next(&[Token::True, Token::False, Token::Null]) {
            self.parse_literal()
        } else if self.is_next(&[Token::Identifier("")]) {
            if self.check(Token::LeftBrace) {
                self.parse_struct_expression()
            } else {
                self.parse_variable()
            }
        } else if self.is_next(&[Token::LeftBracket]) {
            self.parse_vec_expression()
        } else {
            println!("{:?}", self.current);
            bail!("parser: expected: number, string, (, true, false, null, identifier");
        }
    }

    fn parse_number(&mut self, n: f64) -> Result<Expression<'src>> {
        Ok(Expression::Literal(LiteralExpression { value: n.into() }))
    }

    fn parse_string(&mut self, s: &'src str) -> Result<Expression<'src>> {
        Ok(Expression::Literal(LiteralExpression { value: s.into() }))
    }

    fn parse_grouping(&mut self) -> Result<Expression<'src>> {
        let expr = self.parse_expression();
        self.consume(Token::RightParen);
        expr
    }

    fn parse_struct_expression(&mut self) -> Result<Expression<'src>> {
        let name = self.previous.unwrap().get_value();

        self.consume(Token::LeftBrace);

        let mut initializers = vec![];
        while !self.is_next(&[Token::RightBrace]) {
            initializers.push(self.parse_struct_initializer()?);
            self.consume(Token::Comma);
        }

        Ok(Expression::Struct(StructExpression { name, initializers }))
    }

    fn parse_struct_initializer(&mut self) -> Result<Expression<'src>> {
        let member = self.parse_expression()?;
        self.consume(Token::Colon);
        let value = self.parse_expression()?;

        Ok(Expression::StructInitializer(StructInitializerExpression {
            member: member.into(),
            value: value.into(),
        }))
    }

    fn parse_vec_expression(&mut self) -> Result<Expression<'src>> {
        let mut elements = vec![];
        while !self.is_next(&[Token::RightBracket]) {
            elements.push(self.parse_expression()?);
            self.consume(Token::Comma);
        }
        Ok(Expression::Vec(VecExpression { elements }))
    }

    fn parse_variable(&mut self) -> Result<Expression<'src>> {
        let value = self.previous.unwrap().get_value();
        Ok(Expression::Variable(VariableExpression { value }))
    }

    fn parse_literal(&mut self) -> Result<Expression<'src>> {
        let literal = match self.previous.unwrap() {
            Token::True => Literal::Bool(true),
            Token::False => Literal::Bool(false),
            Token::Null => Literal::Null,
            _ => unreachable!(),
        };
        Ok(Expression::Literal(LiteralExpression { value: literal }))
    }
}

impl Default for Parser<'_> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub enum Statement<'src> {
    Print(PrintStatement<'src>),
    Fn(FnStatement<'src>),
    Return(ReturnStatement<'src>),
    If(IfStatement<'src>),
    While(WhileStatement<'src>),
    For(ForStatement<'src>),
    Break(BreakStatement),
    Continue(ContinueStatement),
    Struct(StructStatement<'src>),
    Impl(ImplStatement<'src>),
    Use(UseStatement<'src>),
    Block(BlockStatement<'src>),
    Expression(ExpressionStatement<'src>),
    Dummy,
}

#[derive(Debug)]
pub struct PrintStatement<'src> {
    pub expression: Expression<'src>,
}

#[derive(Debug)]
pub struct FnStatement<'src> {
    pub name: Token<'src>,
    pub arguments: Vec<Token<'src>>,
    pub body: Box<Statement<'src>>,
}

#[derive(Debug)]
pub struct ReturnStatement<'src> {
    pub expression: Expression<'src>,
}

#[derive(Debug)]
pub struct IfStatement<'src> {
    pub condition: Expression<'src>,
    pub if_branch: Box<Statement<'src>>,
    pub else_branch: Box<Statement<'src>>,
}

#[derive(Debug)]
pub struct WhileStatement<'src> {
    pub condition: Expression<'src>,
    pub body: Box<Statement<'src>>,
}

#[derive(Debug)]
pub struct ForStatement<'src> {
    pub initializer: Expression<'src>,
    pub condition: Expression<'src>,
    pub advancement: Expression<'src>,
    pub body: Box<Statement<'src>>,
}

#[derive(Debug)]
pub struct BreakStatement;

#[derive(Debug)]
pub struct ContinueStatement;

#[derive(Debug)]
pub struct StructStatement<'src> {
    pub name: &'src str,
    pub members: Vec<&'src str>,
}

#[derive(Debug)]
pub struct ImplStatement<'src> {
    pub name: &'src str,
    pub methods: Vec<Statement<'src>>,
}

#[derive(Debug)]
pub struct UseStatement<'src> {
    pub module: &'src str,
}

#[derive(Debug)]
pub struct BlockStatement<'src> {
    pub body: Vec<Statement<'src>>,
}

#[derive(Debug)]
pub struct ExpressionStatement<'src> {
    pub expression: Expression<'src>,
}

#[derive(Debug, Clone)]
pub enum Expression<'src> {
    Literal(LiteralExpression<'src>),
    Variable(VariableExpression<'src>),
    Binary(BinaryExpression<'src>),
    Call(CallExpression<'src>),
    Assign(AssignExpression<'src>),
    Logical(LogicalExpression<'src>),
    Unary(UnaryExpression<'src>),
    Get(GetExpression<'src>),
    Struct(StructExpression<'src>),
    StructInitializer(StructInitializerExpression<'src>),
    Vec(VecExpression<'src>),
    Sub(SubscriptExpression<'src>),
}

#[derive(Debug, Clone)]
pub struct LiteralExpression<'src> {
    pub value: Literal<'src>,
}

#[derive(Debug, Clone)]
pub struct VariableExpression<'src> {
    pub value: &'src str,
}

#[derive(Debug, Clone)]
pub struct BinaryExpression<'src> {
    pub kind: BinaryExpressionKind,
    pub lhs: Box<Expression<'src>>,
    pub rhs: Box<Expression<'src>>,
}

#[derive(Debug, Clone)]
pub struct CallExpression<'src> {
    pub callee: Box<Expression<'src>>,
    pub arguments: Vec<Expression<'src>>,
}

#[derive(Debug, Clone)]
pub struct AssignExpression<'src> {
    pub lhs: Box<Expression<'src>>,
    pub rhs: Box<Expression<'src>>,
    pub op: Token<'src>,
}

#[derive(Debug, Clone)]
pub struct LogicalExpression<'src> {
    pub lhs: Box<Expression<'src>>,
    pub rhs: Box<Expression<'src>>,
    pub op: Token<'src>,
}

#[derive(Debug, Clone)]
pub struct UnaryExpression<'src> {
    pub expr: Box<Expression<'src>>,
    pub op: Token<'src>,
}

#[derive(Debug, Clone)]
pub struct StructExpression<'src> {
    pub name: &'src str,
    pub initializers: Vec<Expression<'src>>,
}

#[derive(Debug, Clone)]
pub struct StructInitializerExpression<'src> {
    pub member: Box<Expression<'src>>,
    pub value: Box<Expression<'src>>,
}

#[derive(Debug, Clone)]
pub struct GetExpression<'src> {
    pub expr: Box<Expression<'src>>,
    pub member: &'src str,
    pub op: Token<'src>,
}

#[derive(Debug, Clone)]
pub struct SubscriptExpression<'src> {
    pub expr: Box<Expression<'src>>,
    pub index: Box<Expression<'src>>,
}

#[derive(Debug, Clone)]
pub struct VecExpression<'src> {
    pub elements: Vec<Expression<'src>>,
}

#[derive(Debug, Clone)]
pub enum BinaryExpressionKind {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Equality(bool), /* negation */
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    BitwiseOr,
    BitwiseXor,
    BitwiseAnd,
    BitwiseShl,
    BitwiseShr,
    Strcat,
}

#[derive(Debug, Clone)]
pub enum Literal<'src> {
    Num(f64),
    String(&'src str),
    Bool(bool),
    Null,
}

impl<'src> From<f64> for Literal<'src> {
    fn from(value: f64) -> Self {
        Self::Num(value)
    }
}

impl<'src> From<&'src str> for Literal<'src> {
    fn from(value: &'src str) -> Self {
        Self::String(value)
    }
}
