use crate::errors::{FastAwkError, Result};
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Literals
    String(String),
    Number(f64),
    Regex(String),
    FieldRef(String), // $0, $1, $NF, etc.

    // Identifiers
    Identifier(String),

    // Keywords
    If,
    Else,
    While,
    For,
    Do,
    Break,
    Continue,
    Function,
    Return,
    Delete,
    Exit,
    Next,
    Print,
    Printf,
    Getline,
    Begin,
    End,
    In,

    // Operators
    Plus,
    Minus,
    Multiply,
    Divide,
    Modulo,
    Power,
    Assign,
    PlusAssign,
    MinusAssign,
    MultiplyAssign,
    DivideAssign,
    ModuloAssign,
    PowerAssign,
    
    // Comparison operators
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Match,
    NotMatch,

    // Logical operators
    And,
    Or,
    Not,

    // String operators
    Concatenate,

    // Increment/Decrement
    Increment,
    Decrement,

    // Punctuation
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    Semicolon,
    Comma,
    Question,
    Colon,
    Dollar,

    // Special
    Newline,
    Eof,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::String(s) => write!(f, "\"{}\"", s),
            Token::Number(n) => write!(f, "{}", n),
            Token::Regex(r) => write!(f, "/{}/", r),
            Token::FieldRef(r) => write!(f, "${}", r),
            Token::Identifier(id) => write!(f, "{}", id),
            Token::If => write!(f, "if"),
            Token::Else => write!(f, "else"),
            Token::While => write!(f, "while"),
            Token::For => write!(f, "for"),
            Token::Do => write!(f, "do"),
            Token::Break => write!(f, "break"),
            Token::Continue => write!(f, "continue"),
            Token::Function => write!(f, "function"),
            Token::Return => write!(f, "return"),
            Token::Delete => write!(f, "delete"),
            Token::Exit => write!(f, "exit"),
            Token::Next => write!(f, "next"),
            Token::Print => write!(f, "print"),
            Token::Printf => write!(f, "printf"),
            Token::Getline => write!(f, "getline"),
            Token::Begin => write!(f, "BEGIN"),
            Token::End => write!(f, "END"),
            Token::In => write!(f, "in"),
            Token::Plus => write!(f, "+"),
            Token::Minus => write!(f, "-"),
            Token::Multiply => write!(f, "*"),
            Token::Divide => write!(f, "/"),
            Token::Modulo => write!(f, "%"),
            Token::Power => write!(f, "^"),
            Token::Assign => write!(f, "="),
            Token::PlusAssign => write!(f, "+="),
            Token::MinusAssign => write!(f, "-="),
            Token::MultiplyAssign => write!(f, "*="),
            Token::DivideAssign => write!(f, "/="),
            Token::ModuloAssign => write!(f, "%="),
            Token::PowerAssign => write!(f, "^="),
            Token::Equal => write!(f, "=="),
            Token::NotEqual => write!(f, "!="),
            Token::Less => write!(f, "<"),
            Token::LessEqual => write!(f, "<="),
            Token::Greater => write!(f, ">"),
            Token::GreaterEqual => write!(f, ">="),
            Token::Match => write!(f, "~"),
            Token::NotMatch => write!(f, "!~"),
            Token::And => write!(f, "&&"),
            Token::Or => write!(f, "||"),
            Token::Not => write!(f, "!"),
            Token::Concatenate => write!(f, " "),
            Token::Increment => write!(f, "++"),
            Token::Decrement => write!(f, "--"),
            Token::LeftParen => write!(f, "("),
            Token::RightParen => write!(f, ")"),
            Token::LeftBrace => write!(f, "{{"),
            Token::RightBrace => write!(f, "}}"),
            Token::LeftBracket => write!(f, "["),
            Token::RightBracket => write!(f, "]"),
            Token::Semicolon => write!(f, ";"),
            Token::Comma => write!(f, ","),
            Token::Question => write!(f, "?"),
            Token::Colon => write!(f, ":"),
            Token::Dollar => write!(f, "$"),
            Token::Newline => write!(f, "\\n"),
            Token::Eof => write!(f, "EOF"),
        }
    }
}

pub struct Lexer {
    input: Vec<char>,
    position: usize,
    line: usize,
    column: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            position: 0,
            line: 1,
            column: 1,
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();
        
        loop {
            let token = self.next_token()?;
            let is_eof = matches!(token, Token::Eof);
            tokens.push(token);
            if is_eof {
                break;
            }
        }
        
        Ok(tokens)
    }

    fn next_token(&mut self) -> Result<Token> {
        self.skip_whitespace();
        
        if self.is_at_end() {
            return Ok(Token::Eof);
        }

        let ch = self.current_char();
        self.advance();

        match ch {
            '\n' => {
                self.line += 1;
                self.column = 1;
                Ok(Token::Newline)
            }
            '(' => Ok(Token::LeftParen),
            ')' => Ok(Token::RightParen),
            '{' => Ok(Token::LeftBrace),
            '}' => Ok(Token::RightBrace),
            '[' => Ok(Token::LeftBracket),
            ']' => Ok(Token::RightBracket),
            ';' => Ok(Token::Semicolon),
            ',' => Ok(Token::Comma),
            '?' => Ok(Token::Question),
            ':' => Ok(Token::Colon),
            '$' => Ok(Token::Dollar),
            '+' => {
                if self.match_char('=') {
                    Ok(Token::PlusAssign)
                } else if self.match_char('+') {
                    Ok(Token::Increment)
                } else {
                    Ok(Token::Plus)
                }
            }
            '-' => {
                if self.match_char('=') {
                    Ok(Token::MinusAssign)
                } else if self.match_char('-') {
                    Ok(Token::Decrement)
                } else {
                    Ok(Token::Minus)
                }
            }
            '*' => {
                if self.match_char('=') {
                    Ok(Token::MultiplyAssign)
                } else {
                    Ok(Token::Multiply)
                }
            }
            '/' => {
                if self.match_char('=') {
                    Ok(Token::DivideAssign)
                } else {
                    Ok(Token::Divide)
                }
            }
            '%' => {
                if self.match_char('=') {
                    Ok(Token::ModuloAssign)
                } else {
                    Ok(Token::Modulo)
                }
            }
            '^' => {
                if self.match_char('=') {
                    Ok(Token::PowerAssign)
                } else {
                    Ok(Token::Power)
                }
            }
            '=' => {
                if self.match_char('=') {
                    Ok(Token::Equal)
                } else {
                    Ok(Token::Assign)
                }
            }
            '!' => {
                if self.match_char('=') {
                    Ok(Token::NotEqual)
                } else if self.match_char('~') {
                    Ok(Token::NotMatch)
                } else {
                    Ok(Token::Not)
                }
            }
            '<' => {
                if self.match_char('=') {
                    Ok(Token::LessEqual)
                } else {
                    Ok(Token::Less)
                }
            }
            '>' => {
                if self.match_char('=') {
                    Ok(Token::GreaterEqual)
                } else {
                    Ok(Token::Greater)
                }
            }
            '~' => Ok(Token::Match),
            '&' => {
                if self.match_char('&') {
                    Ok(Token::And)
                } else {
                    Err(FastAwkError::parse_error(self.line, self.column, "Unexpected character '&'"))
                }
            }
            '|' => {
                if self.match_char('|') {
                    Ok(Token::Or)
                } else {
                    Err(FastAwkError::parse_error(self.line, self.column, "Unexpected character '|'"))
                }
            }
            '"' => self.read_string(),
            '\'' => self.read_string_single_quote(),
            _ if ch.is_ascii_digit() => {
                self.position -= 1; // Back up to read the number
                self.column -= 1;
                self.read_number()
            }
            _ if ch.is_ascii_alphabetic() || ch == '_' => {
                self.position -= 1; // Back up to read the identifier
                self.column -= 1;
                self.read_identifier()
            }
            '#' => {
                self.skip_comment();
                self.next_token()
            }
            _ => Err(FastAwkError::parse_error(
                self.line,
                self.column,
                format!("Unexpected character: '{}'", ch),
            )),
        }
    }

    fn current_char(&self) -> char {
        self.input.get(self.position).copied().unwrap_or('\0')
    }

    fn peek_char(&self) -> char {
        self.input.get(self.position + 1).copied().unwrap_or('\0')
    }

    fn advance(&mut self) {
        if !self.is_at_end() {
            self.position += 1;
            self.column += 1;
        }
    }

    fn is_at_end(&self) -> bool {
        self.position >= self.input.len()
    }

    fn match_char(&mut self, expected: char) -> bool {
        if self.is_at_end() || self.current_char() != expected {
            false
        } else {
            self.advance();
            true
        }
    }

    fn skip_whitespace(&mut self) {
        while !self.is_at_end() {
            match self.current_char() {
                ' ' | '\t' | '\r' => self.advance(),
                _ => break,
            }
        }
    }

    fn skip_comment(&mut self) {
        while !self.is_at_end() && self.current_char() != '\n' {
            self.advance();
        }
    }

    fn read_string(&mut self) -> Result<Token> {
        let mut value = String::new();
        
        while !self.is_at_end() && self.current_char() != '"' {
            if self.current_char() == '\\' {
                self.advance();
                if self.is_at_end() {
                    return Err(FastAwkError::parse_error(
                        self.line,
                        self.column,
                        "Unterminated string literal",
                    ));
                }
                
                match self.current_char() {
                    'n' => value.push('\n'),
                    't' => value.push('\t'),
                    'r' => value.push('\r'),
                    '\\' => value.push('\\'),
                    '"' => value.push('"'),
                    '\'' => value.push('\''),
                    c => value.push(c),
                }
            } else {
                value.push(self.current_char());
            }
            self.advance();
        }
        
        if self.is_at_end() {
            return Err(FastAwkError::parse_error(
                self.line,
                self.column,
                "Unterminated string literal",
            ));
        }
        
        self.advance(); // Skip closing quote
        Ok(Token::String(value))
    }

    fn read_string_single_quote(&mut self) -> Result<Token> {
        let mut value = String::new();
        
        while !self.is_at_end() && self.current_char() != '\'' {
            value.push(self.current_char());
            self.advance();
        }
        
        if self.is_at_end() {
            return Err(FastAwkError::parse_error(
                self.line,
                self.column,
                "Unterminated string literal",
            ));
        }
        
        self.advance(); // Skip closing quote
        Ok(Token::String(value))
    }

    fn read_regex(&mut self) -> Result<Token> {
        let mut pattern = String::new();
        
        while !self.is_at_end() && self.current_char() != '/' {
            if self.current_char() == '\\' {
                pattern.push(self.current_char());
                self.advance();
                if !self.is_at_end() {
                    pattern.push(self.current_char());
                    self.advance();
                }
            } else {
                pattern.push(self.current_char());
                self.advance();
            }
        }
        
        if self.is_at_end() {
            return Err(FastAwkError::parse_error(
                self.line,
                self.column,
                "Unterminated regex literal",
            ));
        }
        
        self.advance(); // Skip closing /
        Ok(Token::Regex(pattern))
    }

    fn read_number(&mut self) -> Result<Token> {
        let mut value = String::new();
        
        while !self.is_at_end() && (self.current_char().is_ascii_digit() || self.current_char() == '.') {
            value.push(self.current_char());
            self.advance();
        }
        
        // Handle scientific notation
        if !self.is_at_end() && (self.current_char() == 'e' || self.current_char() == 'E') {
            value.push(self.current_char());
            self.advance();
            
            if !self.is_at_end() && (self.current_char() == '+' || self.current_char() == '-') {
                value.push(self.current_char());
                self.advance();
            }
            
            while !self.is_at_end() && self.current_char().is_ascii_digit() {
                value.push(self.current_char());
                self.advance();
            }
        }
        
        let number = value.parse::<f64>().map_err(|_| {
            FastAwkError::parse_error(self.line, self.column, format!("Invalid number: {}", value))
        })?;
        
        Ok(Token::Number(number))
    }

    fn read_identifier(&mut self) -> Result<Token> {
        let mut value = String::new();
        
        while !self.is_at_end() && (self.current_char().is_ascii_alphanumeric() || self.current_char() == '_') {
            value.push(self.current_char());
            self.advance();
        }
        
        let token = match value.as_str() {
            "if" => Token::If,
            "else" => Token::Else,
            "while" => Token::While,
            "for" => Token::For,
            "do" => Token::Do,
            "break" => Token::Break,
            "continue" => Token::Continue,
            "function" => Token::Function,
            "return" => Token::Return,
            "delete" => Token::Delete,
            "exit" => Token::Exit,
            "next" => Token::Next,
            "print" => Token::Print,
            "printf" => Token::Printf,
            "getline" => Token::Getline,
            "BEGIN" => Token::Begin,
            "END" => Token::End,
            "in" => Token::In,
            _ => Token::Identifier(value),
        };
        
        Ok(token)
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_tokens() {
        let mut lexer = Lexer::new("+ - * / % ^ = == != < <= > >= ~ !~");
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens[0], Token::Plus);
        assert_eq!(tokens[1], Token::Minus);
        assert_eq!(tokens[2], Token::Multiply);
        assert_eq!(tokens[3], Token::Divide);
        assert_eq!(tokens[4], Token::Modulo);
        assert_eq!(tokens[5], Token::Power);
        assert_eq!(tokens[6], Token::Assign);
        assert_eq!(tokens[7], Token::Equal);
        assert_eq!(tokens[8], Token::NotEqual);
        assert_eq!(tokens[9], Token::Less);
        assert_eq!(tokens[10], Token::LessEqual);
        assert_eq!(tokens[11], Token::Greater);
        assert_eq!(tokens[12], Token::GreaterEqual);
        assert_eq!(tokens[13], Token::Match);
        assert_eq!(tokens[14], Token::NotMatch);
    }

    #[test]
    fn test_keywords() {
        let mut lexer = Lexer::new("if else while for BEGIN END");
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens[0], Token::If);
        assert_eq!(tokens[1], Token::Else);
        assert_eq!(tokens[2], Token::While);
        assert_eq!(tokens[3], Token::For);
        assert_eq!(tokens[4], Token::Begin);
        assert_eq!(tokens[5], Token::End);
    }

    #[test]
    fn test_string_literals() {
        let mut lexer = Lexer::new(r#""hello world" "escaped\nstring""#);
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens[0], Token::String("hello world".to_string()));
        assert_eq!(tokens[1], Token::String("escaped\nstring".to_string()));
    }

    #[test]
    fn test_numbers() {
        let mut lexer = Lexer::new("42 3.14 1.23e-4");
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens[0], Token::Number(42.0));
        assert_eq!(tokens[1], Token::Number(3.14));
        assert_eq!(tokens[2], Token::Number(1.23e-4));
    }

    #[test]
    fn test_identifiers() {
        let mut lexer = Lexer::new("variable_name _private func123");
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens[0], Token::Identifier("variable_name".to_string()));
        assert_eq!(tokens[1], Token::Identifier("_private".to_string()));
        assert_eq!(tokens[2], Token::Identifier("func123".to_string()));
    }
}