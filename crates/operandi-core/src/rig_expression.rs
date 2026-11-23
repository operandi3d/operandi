//! The expression language for rig modifiers (RG-60).

use std::collections::BTreeSet;
use std::f64::consts::{E, PI};

const MAX_DEPTH: usize = 256;

/// A parsed expression.
#[derive(Clone, Debug, PartialEq)]
pub struct Expr {
    root: Node,
}

/// A syntax error with a zero-based character index into the source.
#[derive(Clone, Debug, PartialEq)]
pub struct ParseError {
    pub position: usize,
    pub message: String,
}

impl Expr {
    /// Parses `source`. Bare identifiers other than constants are variables.
    pub fn parse(source: &str) -> Result<Self, ParseError> {
        let mut parser = Parser::new(source);
        let root = parser.expression(0, 0)?;
        parser.expect(Token::End, "Unexpected token")?;
        Ok(Self { root })
    }

    /// Returns all variable names in sorted order without duplicates.
    pub fn variables(&self) -> Vec<String> {
        let mut names = BTreeSet::new();
        self.root.collect_variables(&mut names);
        names.into_iter().map(str::to_owned).collect()
    }

    /// Evaluates the expression. Variables missing from `var` read as zero.
    pub fn eval(&self, var: &dyn Fn(&str) -> Option<f64>) -> f64 {
        self.root.eval(var)
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Node {
    Number(f64),
    Variable(String),
    Unary(Unary, Box<Node>),
    // Keep left-associative chains flat so their length doesn't grow the stack.
    Infix(Box<Node>, Vec<(Binary, Node)>),
    Conditional(Box<Node>, Box<Node>, Box<Node>),
    Call(Function, Vec<Node>),
}

impl Node {
    fn collect_variables<'a>(&'a self, names: &mut BTreeSet<&'a str>) {
        match self {
            Self::Number(_) => {}
            Self::Variable(name) => {
                names.insert(name);
            }
            Self::Unary(_, operand) => operand.collect_variables(names),
            Self::Infix(first, rest) => {
                first.collect_variables(names);
                for (_, operand) in rest {
                    operand.collect_variables(names);
                }
            }
            Self::Conditional(condition, yes, no) => {
                condition.collect_variables(names);
                yes.collect_variables(names);
                no.collect_variables(names);
            }
            Self::Call(_, args) => {
                for arg in args {
                    arg.collect_variables(names);
                }
            }
        }
    }

    fn eval(&self, var: &dyn Fn(&str) -> Option<f64>) -> f64 {
        match self {
            Self::Number(value) => *value,
            Self::Variable(name) => var(name).unwrap_or(0.0),
            Self::Unary(op, operand) => {
                let value = operand.eval(var);
                match op {
                    Unary::Plus => value,
                    Unary::Minus => -value,
                    Unary::Not => flag(value == 0.0),
                }
            }
            Self::Infix(first, rest) => {
                let mut value = first.eval(var);
                for (op, operand) in rest {
                    value = op.eval(value, || operand.eval(var));
                }
                value
            }
            Self::Conditional(condition, yes, no) => {
                if condition.eval(var) != 0.0 {
                    yes.eval(var)
                } else {
                    no.eval(var)
                }
            }
            Self::Call(function, args) => function.eval(args.iter().map(|arg| arg.eval(var))),
        }
    }
}

fn flag(value: bool) -> f64 {
    f64::from(u8::from(value))
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Unary {
    Plus,
    Minus,
    Not,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Binary {
    Or,
    And,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Add,
    Subtract,
    Multiply,
    Divide,
    Power,
}

const UNARY_PRECEDENCE: u8 = 7;

impl Binary {
    fn precedence(self) -> u8 {
        match self {
            Self::Or => 1,
            Self::And => 2,
            Self::Equal | Self::NotEqual => 3,
            Self::Less | Self::LessEqual | Self::Greater | Self::GreaterEqual => 4,
            Self::Add | Self::Subtract => 5,
            Self::Multiply | Self::Divide => 6,
            Self::Power => 8,
        }
    }

    fn eval(self, left: f64, right: impl FnOnce() -> f64) -> f64 {
        match self {
            Self::Or => flag(left != 0.0 || right() != 0.0),
            Self::And => flag(left != 0.0 && right() != 0.0),
            Self::Equal => flag(left == right()),
            Self::NotEqual => flag(left != right()),
            Self::Less => flag(left < right()),
            Self::LessEqual => flag(left <= right()),
            Self::Greater => flag(left > right()),
            Self::GreaterEqual => flag(left >= right()),
            Self::Add => left + right(),
            Self::Subtract => left - right(),
            Self::Multiply => left * right(),
            Self::Divide => left / right(),
            Self::Power => left.powf(right()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Function {
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    Sinh,
    Cosh,
    Tanh,
    Asinh,
    Acosh,
    Atanh,
    Log2,
    Log10,
    Ln,
    Exp,
    Sqrt,
    Sign,
    Rint,
    Abs,
    Floor,
    Ceil,
    Min,
    Max,
    Sum,
    Avg,
    Mod,
    Clamp,
    Noise,
}

impl Function {
    fn named(name: &str) -> Option<Self> {
        Some(match name {
            "sin" => Self::Sin,
            "cos" => Self::Cos,
            "tan" => Self::Tan,
            "asin" => Self::Asin,
            "acos" => Self::Acos,
            "atan" => Self::Atan,
            "sinh" => Self::Sinh,
            "cosh" => Self::Cosh,
            "tanh" => Self::Tanh,
            "asinh" => Self::Asinh,
            "acosh" => Self::Acosh,
            "atanh" => Self::Atanh,
            "log2" => Self::Log2,
            "log10" | "log" => Self::Log10,
            "ln" => Self::Ln,
            "exp" => Self::Exp,
            "sqrt" => Self::Sqrt,
            "sign" => Self::Sign,
            "rint" => Self::Rint,
            "abs" => Self::Abs,
            "floor" => Self::Floor,
            "ceil" => Self::Ceil,
            "min" => Self::Min,
            "max" => Self::Max,
            "sum" => Self::Sum,
            "avg" => Self::Avg,
            "mod" => Self::Mod,
            "clamp" => Self::Clamp,
            "noise" => Self::Noise,
            _ => return None,
        })
    }

    fn accepts(self, count: usize) -> bool {
        match self {
            Self::Min | Self::Max | Self::Sum | Self::Avg => count >= 1,
            Self::Mod => count == 2,
            Self::Clamp => count == 3,
            Self::Noise => (1..=3).contains(&count),
            _ => count == 1,
        }
    }

    fn eval(self, mut args: impl ExactSizeIterator<Item = f64>) -> f64 {
        let count = args.len();
        let a = args.next().unwrap_or(f64::NAN);
        match self {
            Self::Sin => a.sin(),
            Self::Cos => a.cos(),
            Self::Tan => a.tan(),
            Self::Asin => a.asin(),
            Self::Acos => a.acos(),
            Self::Atan => a.atan(),
            Self::Sinh => a.sinh(),
            Self::Cosh => a.cosh(),
            Self::Tanh => a.tanh(),
            Self::Asinh => a.asinh(),
            Self::Acosh => a.acosh(),
            Self::Atanh => a.atanh(),
            Self::Log2 => a.log2(),
            Self::Log10 => a.log10(),
            Self::Ln => a.ln(),
            Self::Exp => a.exp(),
            Self::Sqrt => a.sqrt(),
            Self::Sign => {
                if a == 0.0 {
                    0.0
                } else {
                    a.signum()
                }
            }
            Self::Rint => a.round_ties_even(),
            Self::Abs => a.abs(),
            Self::Floor => a.floor(),
            Self::Ceil => a.ceil(),
            Self::Min => args.fold(a, f64::min),
            Self::Max => args.fold(a, f64::max),
            Self::Sum => args.fold(a, |sum, value| sum + value),
            Self::Avg => args.fold(a, |sum, value| sum + value) / count as f64,
            Self::Mod => {
                let b = args.next().unwrap_or(f64::NAN);
                a - b * (a / b).floor()
            }
            Self::Clamp => {
                let lo = args.next().unwrap_or(f64::NAN);
                let hi = args.next().unwrap_or(f64::NAN);
                if lo <= hi { a.clamp(lo, hi) } else { f64::NAN }
            }
            Self::Noise => {
                let mut coordinates = [a, 0.0, 0.0];
                for (coordinate, value) in coordinates[1..].iter_mut().zip(args) {
                    *coordinate = value;
                }
                noise(&coordinates[..count])
            }
        }
    }
}

fn noise(coordinates: &[f64]) -> f64 {
    if coordinates.iter().any(|coordinate| !coordinate.is_finite()) {
        return f64::NAN;
    }
    lattice_noise(coordinates, 0x92ea_853b_54c1_f627)
}

fn lattice_noise(coordinates: &[f64], hash: u64) -> f64 {
    let Some((&coordinate, rest)) = coordinates.split_first() else {
        return (hash >> 11) as f64 * (2.0 / (1_u64 << 53) as f64) - 1.0;
    };
    let lower = coordinate.floor();
    let fraction = coordinate - lower;
    // Smoothstep gives matching, flat slopes on both sides of each lattice boundary.
    let weight = fraction * fraction * (3.0 - 2.0 * fraction);
    let a = lattice_noise(rest, hash_coordinate(hash, lower));
    let b = lattice_noise(rest, hash_coordinate(hash, lower + 1.0));
    a + (b - a) * weight
}

fn hash_coordinate(hash: u64, coordinate: f64) -> u64 {
    // Canonicalize signed zero so adjacent cells share exactly the same corner.
    let bits = if coordinate == 0.0 {
        0
    } else {
        coordinate.to_bits()
    };
    let mut mixed = (hash ^ bits).wrapping_mul(0xd6e8_feb8_6659_fd93);
    mixed ^= mixed >> 32;
    mixed = mixed.wrapping_mul(0xa5a3_568b_34e9_c257);
    mixed ^ (mixed >> 32)
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Token<'a> {
    Number(f64),
    Identifier(&'a str),
    Binary(Binary),
    Bang,
    Open,
    Close,
    Comma,
    Question,
    Colon,
    End,
    Invalid(&'static str),
}

struct Lexer<'a> {
    remaining: &'a str,
    position: usize,
}

impl<'a> Lexer<'a> {
    fn peek(&self) -> Option<char> {
        self.remaining.chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.remaining = &self.remaining[ch.len_utf8()..];
        self.position += 1;
        Some(ch)
    }

    fn eat(&mut self, ch: char) -> bool {
        if self.peek() == Some(ch) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn skip_while(&mut self, predicate: impl Fn(char) -> bool) {
        while self.peek().is_some_and(&predicate) {
            self.bump();
        }
    }

    fn next(&mut self) -> (Token<'a>, usize) {
        self.skip_while(char::is_whitespace);
        let position = self.position;
        let source = self.remaining;
        let token = match self.bump() {
            Some(ch) if ch.is_ascii_digit() || (ch == '.' && self.peek().is_some_and(is_digit)) => {
                return self.number(source, position, ch);
            }
            Some(ch) if ch.is_alphabetic() || ch == '_' => {
                self.skip_while(|ch| ch.is_alphanumeric() || ch == '_');
                Token::Identifier(&source[..source.len() - self.remaining.len()])
            }
            Some('(') => Token::Open,
            Some(')') => Token::Close,
            Some(',') => Token::Comma,
            Some('?') => Token::Question,
            Some(':') => Token::Colon,
            Some('+') => Token::Binary(Binary::Add),
            Some('-') => Token::Binary(Binary::Subtract),
            Some('*') => Token::Binary(Binary::Multiply),
            Some('/') => Token::Binary(Binary::Divide),
            Some('^') => Token::Binary(Binary::Power),
            Some('=') if self.eat('=') => Token::Binary(Binary::Equal),
            Some('!') if self.eat('=') => Token::Binary(Binary::NotEqual),
            Some('!') => Token::Bang,
            Some('<') if self.eat('=') => Token::Binary(Binary::LessEqual),
            Some('<') => Token::Binary(Binary::Less),
            Some('>') if self.eat('=') => Token::Binary(Binary::GreaterEqual),
            Some('>') => Token::Binary(Binary::Greater),
            Some('&') if self.eat('&') => Token::Binary(Binary::And),
            Some('|') if self.eat('|') => Token::Binary(Binary::Or),
            None => Token::End,
            _ => Token::Invalid("Unknown character"),
        };
        (token, position)
    }

    fn number(&mut self, source: &str, position: usize, first: char) -> (Token<'a>, usize) {
        self.skip_while(is_digit);
        if first != '.' && self.eat('.') {
            self.skip_while(is_digit);
        }
        if self.eat('e') || self.eat('E') {
            if matches!(self.peek(), Some('+' | '-')) {
                self.bump();
            }
            if !self.peek().is_some_and(is_digit) {
                return (Token::Invalid("Expected an exponent digit"), self.position);
            }
            self.skip_while(is_digit);
        }
        let spelling = &source[..source.len() - self.remaining.len()];
        let token = match spelling.parse() {
            Ok(value) => Token::Number(value),
            Err(_) => Token::Invalid("Invalid number"),
        };
        (token, position)
    }
}

fn is_digit(ch: char) -> bool {
    ch.is_ascii_digit()
}

struct Parser<'a> {
    lexer: Lexer<'a>,
    token: Token<'a>,
    position: usize,
}

impl<'a> Parser<'a> {
    fn new(source: &'a str) -> Self {
        let mut lexer = Lexer {
            remaining: source,
            position: 0,
        };
        let (token, position) = lexer.next();
        Self {
            lexer,
            token,
            position,
        }
    }

    fn advance(&mut self) {
        (self.token, self.position) = self.lexer.next();
    }

    fn eat(&mut self, token: Token<'a>) -> bool {
        if self.token == token {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, token: Token<'a>, message: &str) -> Result<(), ParseError> {
        if self.eat(token) {
            Ok(())
        } else {
            Err(self.error(message))
        }
    }

    fn error(&self, message: &str) -> ParseError {
        let message = match self.token {
            Token::Invalid(message) => message,
            _ => message,
        };
        ParseError {
            position: self.position,
            message: message.to_owned(),
        }
    }

    fn expression(&mut self, minimum: u8, depth: usize) -> Result<Node, ParseError> {
        if depth > MAX_DEPTH {
            return Err(self.error("Expression is nested too deeply"));
        }
        let first = self.prefix(depth)?;
        let mut rest = Vec::new();
        while let Token::Binary(op) = self.token {
            let precedence = op.precedence();
            if precedence < minimum {
                break;
            }
            self.advance();
            // Power can consume another power on its right at the same precedence.
            let next_minimum = precedence + u8::from(op != Binary::Power);
            rest.push((op, self.expression(next_minimum, depth + 1)?));
        }
        let value = if rest.is_empty() {
            first
        } else {
            Node::Infix(Box::new(first), rest)
        };
        if minimum == 0 && self.eat(Token::Question) {
            let yes = self.expression(0, depth + 1)?;
            self.expect(Token::Colon, "Missing :")?;
            let no = self.expression(0, depth + 1)?;
            Ok(Node::Conditional(
                Box::new(value),
                Box::new(yes),
                Box::new(no),
            ))
        } else {
            Ok(value)
        }
    }

    fn prefix(&mut self, depth: usize) -> Result<Node, ParseError> {
        let unary = match self.token {
            Token::Binary(Binary::Add) => Unary::Plus,
            Token::Binary(Binary::Subtract) => Unary::Minus,
            Token::Bang => Unary::Not,
            _ => return self.primary(depth),
        };
        self.advance();
        let operand = self.expression(UNARY_PRECEDENCE, depth + 1)?;
        Ok(Node::Unary(unary, Box::new(operand)))
    }

    fn primary(&mut self, depth: usize) -> Result<Node, ParseError> {
        match self.token {
            Token::Number(value) => {
                self.advance();
                Ok(Node::Number(value))
            }
            Token::Identifier(name) => {
                let position = self.position;
                self.advance();
                if self.eat(Token::Open) {
                    self.call(name, position, depth)
                } else {
                    Ok(match name {
                        "pi" => Node::Number(PI),
                        "e" => Node::Number(E),
                        _ => Node::Variable(name.to_owned()),
                    })
                }
            }
            Token::Open => {
                self.advance();
                let value = self.expression(0, depth + 1)?;
                self.expect(Token::Close, "Missing )")?;
                Ok(value)
            }
            _ => Err(self.error("Expected a value")),
        }
    }

    fn call(&mut self, name: &str, position: usize, depth: usize) -> Result<Node, ParseError> {
        let function = Function::named(name).ok_or_else(|| ParseError {
            position,
            message: format!("Unknown function {name}"),
        })?;
        let mut args = Vec::new();
        if !self.eat(Token::Close) {
            loop {
                args.push(self.expression(0, depth + 1)?);
                if !self.eat(Token::Comma) {
                    break;
                }
            }
            self.expect(Token::Close, "Missing )")?;
        }
        if !function.accepts(args.len()) {
            return Err(ParseError {
                position,
                message: format!("Wrong number of arguments for {name}"),
            });
        }
        Ok(Node::Call(function, args))
    }
}
