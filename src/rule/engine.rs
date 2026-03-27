#![allow(dead_code)]

use regex::Regex;
use std::collections::HashMap;


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

pub fn parse_rule(expr: &str) -> Option<Expr> {
    let chars: Vec<char> = expr.chars().collect();
    let mut parser = ParserState { chars, pos: 0 };
    parser.parse_or()
}

struct ParserState {
    chars: Vec<char>,
    pos: usize,
}

impl ParserState {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn parse_or(&mut self) -> Option<Expr> {
        let mut left = self.parse_and()?;
        while let Some(c) = self.peek() {
            if c == '|' {
                self.advance();
                let right = self.parse_and()?;
                left = Expr::Or(Box::new(left), Box::new(right));
            } else {
                break;
            }
        }
        Some(left)
    }

    fn parse_and(&mut self) -> Option<Expr> {
        let mut left = self.parse_unary()?;
        while let Some(c) = self.peek() {
            if c == '&' {
                self.advance();
                let right = self.parse_unary()?;
                left = Expr::And(Box::new(left), Box::new(right));
            } else {
                break;
            }
        }
        Some(left)
    }

    fn parse_unary(&mut self) -> Option<Expr> {
        self.skip_whitespace();
        if let Some(c) = self.peek() {
            if c == '!' {
                self.advance();
                let expr = self.parse_unary()?;
                return Some(Expr::Not(Box::new(expr)));
            }
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Option<Expr> {
        self.skip_whitespace();
        let c = self.peek()?;
        if c == '(' {
            self.advance();
            let expr = self.parse_or()?;
            self.skip_whitespace();
            if self.peek() == Some(')') {
                self.advance();
            }
            Some(expr)
        } else if c == '"' {
            self.advance();
            let mut content = String::new();
            while let Some(c) = self.peek() {
                if c == '"' {
                    self.advance();
                    return Some(Expr::Literal(content));
                }
                content.push(c);
                self.advance();
            }
            Some(Expr::Literal(content))
        } else if self.chars[self.pos..]
            .iter()
            .collect::<String>()
            .starts_with("contains(")
        {
            self.pos += 9;
            self.skip_whitespace();
            if self.peek() == Some('"') {
                self.advance();
                let mut content = String::new();
                while let Some(c) = self.peek() {
                    if c == '"' {
                        self.advance();
                        self.skip_whitespace();
                        if self.peek() == Some(')') {
                            self.advance();
                        }
                        return Some(Expr::Contains(content));
                    }
                    content.push(c);
                    self.advance();
                }
            }
            None
        } else if self.chars[self.pos..]
            .iter()
            .collect::<String>()
            .starts_with("regex(")
        {
            self.pos += 6;
            let mut pattern = String::new();
            while let Some(c) = self.peek() {
                if c == ')' {
                    self.advance();
                    match Regex::new(&pattern) {
                        Ok(re) => return Some(Expr::Regex(re)),
                        Err(_) => return Some(Expr::Literal(pattern)),
                    }
                }
                pattern.push(c);
                self.advance();
            }
            None
        } else {
            None
        }
    }
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
