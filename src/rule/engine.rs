use regex::Regex;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Literal(String),
    And,
    Or,
    Not,
    LParen,
    RParen,
    Regex(String),
    Contains,
}

pub struct Lexer {
    pos: usize,
    input: Vec<char>,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Lexer {
            pos: 0,
            input: input.chars().collect(),
        }
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        while self.pos < self.input.len() {
            self.skip_whitespace();
            if self.pos >= self.input.len() {
                break;
            }
            let ch = self.input[self.pos];
            match ch {
                '&' => {
                    tokens.push(Token::And);
                    self.pos += 1;
                }
                '|' => {
                    tokens.push(Token::Or);
                    self.pos += 1;
                }
                '!' => {
                    tokens.push(Token::Not);
                    self.pos += 1;
                }
                '(' => {
                    tokens.push(Token::LParen);
                    self.pos += 1;
                }
                ')' => {
                    tokens.push(Token::RParen);
                    self.pos += 1;
                }
                '"' => {
                    if let Some(lit) = self.read_string_literal() {
                        tokens.push(Token::Literal(lit));
                    }
                }
                'r' => {
                    if self.input[self.pos..]
                        .iter()
                        .collect::<String>()
                        .starts_with("regex(")
                    {
                        if let Some(re) = self.read_regex() {
                            tokens.push(Token::Regex(re));
                        }
                    } else {
                        self.pos += 1;
                    }
                }
                'c' => {
                    let rest: String = self.input[self.pos..].iter().collect();
                    if rest.starts_with("contains(") {
                        if let Some(s) = self.read_contains() {
                            tokens.push(Token::Contains);
                            tokens.push(Token::Literal(s));
                        }
                    } else {
                        self.pos += 1;
                    }
                }
                _ => {
                    self.pos += 1;
                }
            }
        }
        tokens
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() && self.input[self.pos].is_whitespace() {
            self.pos += 1;
        }
    }

    fn read_string_literal(&mut self) -> Option<String> {
        if self.input[self.pos] != '"' {
            return None;
        }
        self.pos += 1;
        let mut result = String::new();
        while self.pos < self.input.len() && self.input[self.pos] != '"' {
            let ch = self.input[self.pos];
            if ch == '\\' && self.pos + 1 < self.input.len() {
                self.pos += 1;
                if self.pos < self.input.len() {
                    result.push(self.input[self.pos]);
                    self.pos += 1;
                }
            } else {
                result.push(ch);
                self.pos += 1;
            }
        }
        if self.pos < self.input.len() && self.input[self.pos] == '"' {
            self.pos += 1;
        }
        Some(result)
    }

    fn read_regex(&mut self) -> Option<String> {
        let rest: String = self.input[self.pos..].iter().collect();
        if !rest.starts_with("regex(") {
            return None;
        }
        self.pos += 6;
        self.skip_whitespace();
        let mut pattern = String::new();
        let mut paren_depth = 0;
        let mut in_pattern = false;
        while self.pos < self.input.len() {
            let ch = self.input[self.pos];
            if ch == '(' {
                paren_depth += 1;
                in_pattern = true;
            } else if ch == ')' && paren_depth > 0 {
                paren_depth -= 1;
                if paren_depth == 0 {
                    self.pos += 1;
                    break;
                }
            }
            if in_pattern {
                pattern.push(ch);
            }
            self.pos += 1;
        }
        Some(pattern)
    }

    fn read_contains(&mut self) -> Option<String> {
        let rest: String = self.input[self.pos..].iter().collect();
        if !rest.starts_with("contains(") {
            return None;
        }
        self.pos += 9;
        self.skip_whitespace();
        self.read_string_literal()
    }
}

#[derive(Debug, Clone)]
pub enum Expr {
    Literal(String),
    Regex(Regex),
    Contains(String),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Not(Box<Expr>),
}

impl Expr {
    pub fn evaluate(&self, text: &str) -> bool {
        match self {
            Expr::Literal(s) => text.contains(s),
            Expr::Regex(re) => re.is_match(text),
            Expr::Contains(s) => text.contains(s),
            Expr::And(a, b) => a.evaluate(text) && b.evaluate(text),
            Expr::Or(a, b) => a.evaluate(text) || b.evaluate(text),
            Expr::Not(e) => !e.evaluate(text),
        }
    }
}

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    pub fn parse(&mut self) -> Option<Expr> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Option<Expr> {
        let mut left = self.parse_and()?;
        while self.peek() == Some(&Token::Or) {
            self.advance();
            let right = self.parse_and()?;
            left = Expr::Or(Box::new(left), Box::new(right));
        }
        Some(left)
    }

    fn parse_and(&mut self) -> Option<Expr> {
        let mut left = self.parse_unary()?;
        while self.peek() == Some(&Token::And) {
            self.advance();
            let right = self.parse_unary()?;
            left = Expr::And(Box::new(left), Box::new(right));
        }
        Some(left)
    }

    fn parse_unary(&mut self) -> Option<Expr> {
        if self.peek() == Some(&Token::Not) {
            self.advance();
            let expr = self.parse_unary()?;
            return Some(Expr::Not(Box::new(expr)));
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Option<Expr> {
        match self.peek() {
            Some(Token::Literal(s)) => {
                let s = s.clone();
                self.advance();
                Some(Expr::Literal(s))
            }
            Some(Token::Regex(pattern)) => {
                let pattern = pattern.clone();
                self.advance();
                match Regex::new(&pattern) {
                    Ok(re) => Some(Expr::Regex(re)),
                    Err(_) => None,
                }
            }
            Some(Token::Contains) => {
                self.advance();
                if let Some(Token::Literal(s)) = self.peek() {
                    let s = s.clone();
                    self.advance();
                    Some(Expr::Contains(s))
                } else {
                    None
                }
            }
            Some(Token::LParen) => {
                self.advance();
                let expr = self.parse_or()?;
                if self.peek() == Some(&Token::RParen) {
                    self.advance();
                }
                Some(expr)
            }
            _ => None,
        }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) {
        self.pos += 1;
    }
}

pub fn parse_rule(expr: &str) -> Option<Expr> {
    let mut lexer = Lexer::new(expr);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    parser.parse()
}

pub struct RuleEngine {
    hooks: HashMap<String, Box<dyn Fn(&str) -> String + Send + Sync>>,
}

impl RuleEngine {
    pub fn new() -> Self {
        RuleEngine {
            hooks: HashMap::new(),
        }
    }

    pub fn register_hook<F>(&mut self, name: &str, hook: F)
    where
        F: Fn(&str) -> String + Send + Sync + 'static,
    {
        self.hooks.insert(name.to_string(), Box::new(hook));
    }

    pub fn execute_hook(&self, hook_call: &str) -> Option<String> {
        let parts: Vec<&str> = hook_call.splitn(2, '(').collect();
        if parts.len() != 2 {
            return None;
        }
        let name = parts[0];
        let mut args = parts[1].to_string();
        if args.ends_with(')') {
            args.pop();
        }
        let args = args.trim();

        if let Some(hook) = self.hooks.get(name) {
            Some(hook(args))
        } else {
            None
        }
    }

    pub fn evaluate_rule(&self, expr: &str, text: &str) -> bool {
        if let Some(expr) = parse_rule(expr) {
            expr.evaluate(text)
        } else {
            false
        }
    }

    pub fn evaluate_and_inject(&self, expr: &str, hook: &str, text: &str) -> Option<String> {
        if !self.evaluate_rule(expr, text) {
            return None;
        }
        self.execute_hook(hook)
    }
}

impl Default for RuleEngine {
    fn default() -> Self {
        Self::new()
    }
}
