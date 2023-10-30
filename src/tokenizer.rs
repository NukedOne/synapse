use regex::Regex;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TokenKind {
    Print,
    Fn,
    Return,
    If,
    Else,
    Identifier,
    Number,
    True,
    False,
    Null,
    Plus,
    Minus,
    Star,
    Slash,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    BangEqual,
    DoubleEqual,
    String,
    Semicolon,
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Comma,
}

#[derive(Debug, Clone, Copy)]
pub struct Token<'source> {
    pub kind: TokenKind,
    pub value: &'source str,
}

impl<'source> Token<'source> {
    fn new(kind: TokenKind, value: &'source str) -> Token<'source> {
        Token { kind, value }
    }
}

pub struct Tokenizer<'source> {
    src: &'source str,
    start: usize,
}

impl<'source> Iterator for Tokenizer<'source> {
    type Item = Token<'source>;

    fn next(&mut self) -> Option<Self::Item> {
        let re_keyword = r"?P<keyword>print|fn|return|if|else";
        let re_literal = r"?P<literal>true|false|null";
        let re_identifier = r"?P<identifier>[a-zA-Z_][a-zA-Z0-9_]*";
        let re_individual = r"?P<individual>[-+*/<>;(){},]";
        let re_double = r"?P<double>==|!=|<=|>=|\+\+";
        let re_number = r"?P<number>[-+]?\d+(\.\d+)?";
        let re_string = r#""(?P<string>[^\n"]*)""#;

        let r = Regex::new(
            format!(
                "({})|({})|({})|({})|({})|({}|({}))",
                re_keyword,
                re_literal,
                re_identifier,
                re_double,
                re_individual,
                re_number,
                re_string,
            )
            .as_str(),
        )
        .unwrap();

        let token = match r.captures_at(self.src, self.start) {
            Some(captures) => {
                if let Some(m) = captures.name("keyword") {
                    self.start = m.end();
                    match m.as_str() {
                        "print" => Token::new(TokenKind::Print, "print"),
                        "fn" => Token::new(TokenKind::Fn, "fn"),
                        "return" => Token::new(TokenKind::Return, "return"),
                        "if" => Token::new(TokenKind::If, "if"),
                        "else" => Token::new(TokenKind::Else, "else"),
                        _ => unreachable!(),
                    }
                } else if let Some(m) = captures.name("literal") {
                    self.start = m.end();
                    match m.as_str() {
                        "true" => Token::new(TokenKind::True, "true"),
                        "false" => Token::new(TokenKind::False, "false"),
                        "null" => Token::new(TokenKind::Null, "null"),
                        _ => unreachable!(),
                    }
                } else if let Some(m) = captures.name("identifier") {
                    self.start = m.end();
                    Token::new(TokenKind::Identifier, m.as_str())
                } else if let Some(m) = captures.name("individual") {
                    self.start = m.end();
                    match m.as_str() {
                        "+" => Token::new(TokenKind::Plus, "+"),
                        "-" => Token::new(TokenKind::Minus, "-"),
                        "*" => Token::new(TokenKind::Star, "*"),
                        "/" => Token::new(TokenKind::Slash, "/"),
                        "<" => Token::new(TokenKind::Less, "<"),
                        ">" => Token::new(TokenKind::Greater, ">"),
                        ";" => Token::new(TokenKind::Semicolon, ";"),
                        "(" => Token::new(TokenKind::LeftParen, "("),
                        ")" => Token::new(TokenKind::RightParen, ")"),
                        "{" => Token::new(TokenKind::LeftBrace, "{"),
                        "}" => Token::new(TokenKind::RightBrace, "}"),
                        "," => Token::new(TokenKind::Comma, ","),
                        _ => unreachable!(),
                    }
                } else if let Some(m) = captures.name("double") {
                    self.start = m.end();
                    match m.as_str() {
                        "==" => Token::new(TokenKind::DoubleEqual, "=="),
                        "!=" => Token::new(TokenKind::BangEqual, "!="),
                        "<=" => Token::new(TokenKind::LessEqual, "<="),
                        ">=" => Token::new(TokenKind::GreaterEqual, ">="),
                        _ => unreachable!(),
                    }
                } else if let Some(m) = captures.name("number") {
                    self.start = m.end();
                    Token::new(TokenKind::Number, m.as_str())
                } else if let Some(m) = captures.name("string") {
                    self.start = m.end();
                    Token::new(TokenKind::String, m.as_str())
                } else {
                    return None;
                }
            }
            None => return None,
        };

        Some(token)
    }
}

impl<'source> Tokenizer<'source> {
    pub fn new(src: &'source str) -> Tokenizer<'source> {
        Tokenizer { src, start: 0 }
    }
}
