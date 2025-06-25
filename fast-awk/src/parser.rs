use crate::ast::*;
use crate::errors::{FastAwkError, Result};
use crate::lexer::{Lexer, Token};
use crate::value::Value;

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    pub fn new(input: &str) -> Result<Self> {
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize()?;
        Ok(Self { tokens, current: 0 })
    }

    pub fn parse(&mut self) -> Result<Program> {
        let mut program = Program::new();

        while !self.is_at_end() {
            self.skip_newlines();
            if self.is_at_end() {
                break;
            }

            if self.check(&Token::Function) {
                let function = self.parse_function()?;
                program.add_function(function);
            } else {
                let rule = self.parse_rule()?;
                program.add_rule(rule);
            }
        }

        Ok(program)
    }

    fn parse_function(&mut self) -> Result<Function> {
        self.consume(Token::Function, "Expected 'function'")?;
        
        let name = if let Token::Identifier(name) = self.advance() {
            name.clone()
        } else {
            return Err(FastAwkError::syntax_error("Expected function name"));
        };

        self.consume(Token::LeftParen, "Expected '(' after function name")?;
        
        let mut parameters = Vec::new();
        if !self.check(&Token::RightParen) {
            loop {
                if let Token::Identifier(param) = self.advance() {
                    parameters.push(param.clone());
                } else {
                    return Err(FastAwkError::syntax_error("Expected parameter name"));
                }
                
                if !self.match_token(&Token::Comma) {
                    break;
                }
            }
        }
        
        self.consume(Token::RightParen, "Expected ')' after parameters")?;
        self.skip_newlines();
        
        let body = self.parse_action()?;
        
        Ok(Function {
            name,
            parameters,
            body,
        })
    }

    fn parse_rule(&mut self) -> Result<Rule> {
        let pattern = self.parse_pattern()?;
        self.skip_newlines();
        
        let action = if self.check(&Token::LeftBrace) {
            self.parse_action()?
        } else if pattern.is_some() {
            // Pattern without action - default action is print
            let mut action = Action::new();
            action.add_statement(Statement::Print(PrintStatement::new()));
            action
        } else {
            return Err(FastAwkError::syntax_error("Expected pattern or action"));
        };

        Ok(Rule { pattern, action })
    }

    fn parse_pattern(&mut self) -> Result<Option<Pattern>> {
        if self.match_token(&Token::Begin) {
            Ok(Some(Pattern::Begin))
        } else if self.match_token(&Token::End) {
            Ok(Some(Pattern::End))
        } else if self.check(&Token::LeftBrace) {
            // No pattern, just action
            Ok(None)
        } else {
            let expr = self.parse_expression()?;
            
            // Check for range pattern
            if self.match_token(&Token::Comma) {
                let end_expr = self.parse_expression()?;
                Ok(Some(Pattern::Range(
                    Box::new(Pattern::Expression(expr)),
                    Box::new(Pattern::Expression(end_expr)),
                )))
            } else {
                Ok(Some(Pattern::Expression(expr)))
            }
        }
    }

    fn parse_action(&mut self) -> Result<Action> {
        self.consume(Token::LeftBrace, "Expected '{'")?;
        self.skip_newlines();
        
        let mut action = Action::new();
        
        while !self.check(&Token::RightBrace) && !self.is_at_end() {
            let stmt = self.parse_statement()?;
            action.add_statement(stmt);
            self.skip_newlines();
        }
        
        self.consume(Token::RightBrace, "Expected '}'")?;
        Ok(action)
    }

    fn parse_statement(&mut self) -> Result<Statement> {
        match self.peek() {
            Token::If => self.parse_if_statement(),
            Token::While => self.parse_while_statement(),
            Token::For => self.parse_for_statement(),
            Token::Do => self.parse_do_while_statement(),
            Token::Break => {
                self.advance();
                self.consume_statement_terminator()?;
                Ok(Statement::Break)
            }
            Token::Continue => {
                self.advance();
                self.consume_statement_terminator()?;
                Ok(Statement::Continue)
            }
            Token::Next => {
                self.advance();
                self.consume_statement_terminator()?;
                Ok(Statement::Next)
            }
            Token::Exit => {
                self.advance();
                let expr = if self.check_statement_terminator() {
                    None
                } else {
                    Some(self.parse_expression()?)
                };
                self.consume_statement_terminator()?;
                Ok(Statement::Exit(expr))
            }
            Token::Return => {
                self.advance();
                let expr = if self.check_statement_terminator() {
                    None
                } else {
                    Some(self.parse_expression()?)
                };
                self.consume_statement_terminator()?;
                Ok(Statement::Return(expr))
            }
            Token::Delete => {
                self.advance();
                let expr = self.parse_expression()?;
                self.consume_statement_terminator()?;
                Ok(Statement::Delete(expr))
            }
            Token::Print => self.parse_print_statement(),
            Token::Printf => self.parse_printf_statement(),
            Token::LeftBrace => {
                let action = self.parse_action()?;
                Ok(Statement::Block(action.statements))
            }
            _ => {
                let expr = self.parse_expression()?;
                self.consume_statement_terminator()?;
                Ok(Statement::Expression(expr))
            }
        }
    }

    fn parse_if_statement(&mut self) -> Result<Statement> {
        self.consume(Token::If, "Expected 'if'")?;
        self.consume(Token::LeftParen, "Expected '(' after 'if'")?;
        let condition = self.parse_expression()?;
        self.consume(Token::RightParen, "Expected ')' after if condition")?;
        self.skip_newlines();
        
        let then_stmt = Box::new(self.parse_statement()?);
        
        let else_stmt = if self.match_token(&Token::Else) {
            self.skip_newlines();
            Some(Box::new(self.parse_statement()?))
        } else {
            None
        };
        
        Ok(Statement::If {
            condition,
            then_stmt,
            else_stmt,
        })
    }

    fn parse_while_statement(&mut self) -> Result<Statement> {
        self.consume(Token::While, "Expected 'while'")?;
        self.consume(Token::LeftParen, "Expected '(' after 'while'")?;
        let condition = self.parse_expression()?;
        self.consume(Token::RightParen, "Expected ')' after while condition")?;
        self.skip_newlines();
        
        let body = Box::new(self.parse_statement()?);
        
        Ok(Statement::While { condition, body })
    }

    fn parse_for_statement(&mut self) -> Result<Statement> {
        self.consume(Token::For, "Expected 'for'")?;
        self.consume(Token::LeftParen, "Expected '(' after 'for'")?;
        
        // Check for for-in loop
        if matches!(self.peek(), Token::Identifier(_)) && self.peek_ahead(1) == &Token::In {
            let variable = if let Token::Identifier(name) = self.advance() {
                name.clone()
            } else {
                return Err(FastAwkError::syntax_error("Expected variable name"));
            };
            
            self.consume(Token::In, "Expected 'in'")?;
            let array = self.parse_expression()?;
            self.consume(Token::RightParen, "Expected ')' after for-in")?;
            self.skip_newlines();
            
            let body = Box::new(self.parse_statement()?);
            
            Ok(Statement::ForIn {
                variable,
                array,
                body,
            })
        } else {
            // Regular for loop
            let init = if self.check(&Token::Semicolon) {
                None
            } else {
                Some(self.parse_expression()?)
            };
            
            self.consume(Token::Semicolon, "Expected ';' after for loop initializer")?;
            
            let condition = if self.check(&Token::Semicolon) {
                None
            } else {
                Some(self.parse_expression()?)
            };
            
            self.consume(Token::Semicolon, "Expected ';' after for loop condition")?;
            
            let update = if self.check(&Token::RightParen) {
                None
            } else {
                Some(self.parse_expression()?)
            };
            
            self.consume(Token::RightParen, "Expected ')' after for loop")?;
            self.skip_newlines();
            
            let body = Box::new(self.parse_statement()?);
            
            Ok(Statement::For {
                init,
                condition,
                update,
                body,
            })
        }
    }

    fn parse_do_while_statement(&mut self) -> Result<Statement> {
        self.consume(Token::Do, "Expected 'do'")?;
        self.skip_newlines();
        
        let body = Box::new(self.parse_statement()?);
        
        self.consume(Token::While, "Expected 'while' after do body")?;
        self.consume(Token::LeftParen, "Expected '(' after 'while'")?;
        let condition = self.parse_expression()?;
        self.consume(Token::RightParen, "Expected ')' after while condition")?;
        self.consume_statement_terminator()?;
        
        // Transform do-while into while loop with initial execution
        Ok(Statement::Block(vec![
            *body.clone(),
            Statement::While { condition, body },
        ]))
    }

    fn parse_print_statement(&mut self) -> Result<Statement> {
        self.consume(Token::Print, "Expected 'print'")?;
        
        let mut expressions = Vec::new();
        
        if !self.check_statement_terminator() && !self.check(&Token::Greater) && !self.check(&Token::Or) {
            expressions.push(self.parse_expression()?);
            
            while self.match_token(&Token::Comma) {
                expressions.push(self.parse_expression()?);
            }
        }
        
        let output_target = self.parse_output_target()?;
        self.consume_statement_terminator()?;
        
        let mut print_stmt = PrintStatement::with_expressions(expressions);
        if let Some(target) = output_target {
            print_stmt = print_stmt.with_target(target);
        }
        
        Ok(Statement::Print(print_stmt))
    }

    fn parse_printf_statement(&mut self) -> Result<Statement> {
        self.consume(Token::Printf, "Expected 'printf'")?;
        
        let format = self.parse_expression()?;
        let mut arguments = Vec::new();
        
        while self.match_token(&Token::Comma) {
            arguments.push(self.parse_expression()?);
        }
        
        let output_target = self.parse_output_target()?;
        self.consume_statement_terminator()?;
        
        let mut printf_stmt = PrintfStatement::new(format).with_arguments(arguments);
        if let Some(target) = output_target {
            printf_stmt = printf_stmt.with_target(target);
        }
        
        Ok(Statement::Printf(printf_stmt))
    }

    fn parse_output_target(&mut self) -> Result<Option<OutputTarget>> {
        if self.match_token(&Token::Greater) {
            let expr = self.parse_expression()?;
            Ok(Some(OutputTarget::File(expr)))
        } else if self.match_token(&Token::Or) {
            let expr = self.parse_expression()?;
            Ok(Some(OutputTarget::Pipe(expr)))
        } else {
            Ok(None)
        }
    }

    fn parse_expression(&mut self) -> Result<Expression> {
        self.parse_ternary()
    }

    fn parse_ternary(&mut self) -> Result<Expression> {
        let expr = self.parse_logical_or()?;
        
        if self.match_token(&Token::Question) {
            let true_expr = self.parse_expression()?;
            self.consume(Token::Colon, "Expected ':' in ternary expression")?;
            let false_expr = self.parse_expression()?;
            
            Ok(Expression::Ternary {
                condition: Box::new(expr),
                true_expr: Box::new(true_expr),
                false_expr: Box::new(false_expr),
            })
        } else {
            Ok(expr)
        }
    }

    fn parse_logical_or(&mut self) -> Result<Expression> {
        let mut expr = self.parse_logical_and()?;
        
        while self.match_token(&Token::Or) {
            let right = self.parse_logical_and()?;
            expr = Expression::Or(Box::new(expr), Box::new(right));
        }
        
        Ok(expr)
    }

    fn parse_logical_and(&mut self) -> Result<Expression> {
        let mut expr = self.parse_in()?;
        
        while self.match_token(&Token::And) {
            let right = self.parse_in()?;
            expr = Expression::And(Box::new(expr), Box::new(right));
        }
        
        Ok(expr)
    }

    fn parse_in(&mut self) -> Result<Expression> {
        let mut expr = self.parse_regex_match()?;
        
        while self.match_token(&Token::In) {
            let right = self.parse_regex_match()?;
            expr = Expression::In(Box::new(expr), Box::new(right));
        }
        
        Ok(expr)
    }

    fn parse_regex_match(&mut self) -> Result<Expression> {
        let mut expr = self.parse_relational()?;
        
        while self.match_token(&Token::Match) || self.match_token(&Token::NotMatch) {
            let is_match = self.previous() == &Token::Match;
            let right = self.parse_relational()?;
            
            expr = if is_match {
                Expression::Match(Box::new(expr), Box::new(right))
            } else {
                Expression::NotMatch(Box::new(expr), Box::new(right))
            };
        }
        
        Ok(expr)
    }

    fn parse_relational(&mut self) -> Result<Expression> {
        let mut expr = self.parse_concatenation()?;
        
        while self.check(&Token::Less) || self.check(&Token::LessEqual) ||
              self.check(&Token::Greater) || self.check(&Token::GreaterEqual) ||
              self.check(&Token::Equal) || self.check(&Token::NotEqual) {
            
            let op = self.advance().clone();
            let right = self.parse_concatenation()?;
            
            expr = match op {
                Token::Less => Expression::Less(Box::new(expr), Box::new(right)),
                Token::LessEqual => Expression::LessEqual(Box::new(expr), Box::new(right)),
                Token::Greater => Expression::Greater(Box::new(expr), Box::new(right)),
                Token::GreaterEqual => Expression::GreaterEqual(Box::new(expr), Box::new(right)),
                Token::Equal => Expression::Equal(Box::new(expr), Box::new(right)),
                Token::NotEqual => Expression::NotEqual(Box::new(expr), Box::new(right)),
                _ => unreachable!(),
            };
        }
        
        Ok(expr)
    }

    fn parse_concatenation(&mut self) -> Result<Expression> {
        let mut expr = self.parse_additive()?;
        
        // String concatenation is implicit (whitespace)
        while self.is_concatenation_context() {
            let right = self.parse_additive()?;
            expr = Expression::Concatenate(Box::new(expr), Box::new(right));
        }
        
        Ok(expr)
    }

    fn parse_additive(&mut self) -> Result<Expression> {
        let mut expr = self.parse_multiplicative()?;
        
        while self.match_token(&Token::Plus) || self.match_token(&Token::Minus) {
            let is_add = self.previous() == &Token::Plus;
            let right = self.parse_multiplicative()?;
            
            expr = if is_add {
                Expression::Add(Box::new(expr), Box::new(right))
            } else {
                Expression::Subtract(Box::new(expr), Box::new(right))
            };
        }
        
        Ok(expr)
    }

    fn parse_multiplicative(&mut self) -> Result<Expression> {
        let mut expr = self.parse_power()?;
        
        while self.match_token(&Token::Multiply) || self.match_token(&Token::Divide) || 
              self.match_token(&Token::Modulo) {
            
            let op = self.previous().clone();
            let right = self.parse_power()?;
            
            expr = match op {
                Token::Multiply => Expression::Multiply(Box::new(expr), Box::new(right)),
                Token::Divide => Expression::Divide(Box::new(expr), Box::new(right)),
                Token::Modulo => Expression::Modulo(Box::new(expr), Box::new(right)),
                _ => unreachable!(),
            };
        }
        
        Ok(expr)
    }

    fn parse_power(&mut self) -> Result<Expression> {
        let mut expr = self.parse_unary()?;
        
        if self.match_token(&Token::Power) {
            let right = self.parse_power()?; // Right associative
            expr = Expression::Power(Box::new(expr), Box::new(right));
        }
        
        Ok(expr)
    }

    fn parse_unary(&mut self) -> Result<Expression> {
        if self.match_token(&Token::Not) {
            let expr = self.parse_unary()?;
            Ok(Expression::Not(Box::new(expr)))
        } else if self.match_token(&Token::Minus) {
            let expr = self.parse_unary()?;
            Ok(Expression::UnaryMinus(Box::new(expr)))
        } else if self.match_token(&Token::Plus) {
            let expr = self.parse_unary()?;
            Ok(Expression::UnaryPlus(Box::new(expr)))
        } else if self.match_token(&Token::Increment) {
            let expr = self.parse_postfix()?;
            Ok(Expression::PreIncrement(Box::new(expr)))
        } else if self.match_token(&Token::Decrement) {
            let expr = self.parse_postfix()?;
            Ok(Expression::PreDecrement(Box::new(expr)))
        } else {
            self.parse_postfix()
        }
    }

    fn parse_postfix(&mut self) -> Result<Expression> {
        let mut expr = self.parse_primary()?;
        
        loop {
            if self.match_token(&Token::Increment) {
                expr = Expression::PostIncrement(Box::new(expr));
            } else if self.match_token(&Token::Decrement) {
                expr = Expression::PostDecrement(Box::new(expr));
            } else if self.match_token(&Token::LeftBracket) {
                let index = self.parse_expression()?;
                self.consume(Token::RightBracket, "Expected ']' after array index")?;
                expr = Expression::ArrayRef {
                    array: Box::new(expr),
                    index: Box::new(index),
                };
            } else if self.match_token(&Token::LeftParen) && matches!(expr, Expression::Identifier(_)) {
                // Function call
                if let Expression::Identifier(name) = expr {
                    let mut arguments = Vec::new();
                    
                    if !self.check(&Token::RightParen) {
                        arguments.push(self.parse_expression()?);
                        while self.match_token(&Token::Comma) {
                            arguments.push(self.parse_expression()?);
                        }
                    }
                    
                    self.consume(Token::RightParen, "Expected ')' after function arguments")?;
                    expr = Expression::FunctionCall { name, arguments };
                } else {
                    return Err(FastAwkError::syntax_error("Invalid function call"));
                }
            } else {
                break;
            }
        }
        
        // Handle assignment operators
        if self.check_assignment_operator() {
            let op = self.advance().clone();
            let right = self.parse_expression()?;
            
            expr = match op {
                Token::Assign => Expression::Assign(Box::new(expr), Box::new(right)),
                Token::PlusAssign => Expression::AddAssign(Box::new(expr), Box::new(right)),
                Token::MinusAssign => Expression::SubtractAssign(Box::new(expr), Box::new(right)),
                Token::MultiplyAssign => Expression::MultiplyAssign(Box::new(expr), Box::new(right)),
                Token::DivideAssign => Expression::DivideAssign(Box::new(expr), Box::new(right)),
                Token::ModuloAssign => Expression::ModuloAssign(Box::new(expr), Box::new(right)),
                Token::PowerAssign => Expression::PowerAssign(Box::new(expr), Box::new(right)),
                _ => unreachable!(),
            };
        }
        
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expression> {
        match self.peek() {
            Token::Number(n) => {
                let value = *n;
                self.advance();
                Ok(Expression::Literal(Value::Number(value)))
            }
            Token::String(s) => {
                let value = s.clone();
                self.advance();
                Ok(Expression::Literal(Value::String(value)))
            }
            Token::Regex(pattern) => {
                let pattern = pattern.clone();
                self.advance();
                Ok(Expression::Regex(pattern))
            }
            Token::Identifier(name) => {
                let name = name.clone();
                self.advance();
                Ok(Expression::Identifier(name))
            }
            Token::Dollar => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expression::FieldRef(Box::new(expr)))
            }
            Token::LeftParen => {
                self.advance();
                let expr = self.parse_expression()?;
                self.consume(Token::RightParen, "Expected ')' after expression")?;
                Ok(expr)
            }
            Token::Getline => {
                self.advance();
                // Simplified getline parsing
                Ok(Expression::Getline {
                    target: None,
                    source: None,
                })
            }
            Token::Divide => {
                // This could be the start of a regex literal /pattern/
                self.advance(); // consume the /
                let mut pattern = String::new();
                
                while !self.is_at_end() && !self.check(&Token::Divide) {
                    match self.peek() {
                        Token::String(s) => {
                            pattern.push_str(s);
                            self.advance();
                        }
                        Token::Identifier(id) => {
                            pattern.push_str(id);
                            self.advance();
                        }
                        Token::Number(n) => {
                            pattern.push_str(&n.to_string());
                            self.advance();
                        }
                        _ => {
                            pattern.push_str(&self.advance().to_string());
                        }
                    }
                }
                
                if !self.check(&Token::Divide) {
                    return Err(FastAwkError::syntax_error("Unterminated regex literal"));
                }
                self.advance(); // consume closing /
                
                Ok(Expression::Regex(pattern))
            }
            _ => Err(FastAwkError::syntax_error(format!(
                "Unexpected token: {}",
                self.peek()
            ))),
        }
    }

    // Helper methods
    fn is_at_end(&self) -> bool {
        matches!(self.peek(), Token::Eof)
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.current).unwrap_or(&Token::Eof)
    }

    fn peek_ahead(&self, offset: usize) -> &Token {
        self.tokens.get(self.current + offset).unwrap_or(&Token::Eof)
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }

    fn check(&self, token_type: &Token) -> bool {
        std::mem::discriminant(self.peek()) == std::mem::discriminant(token_type)
    }

    fn match_token(&mut self, token_type: &Token) -> bool {
        if self.check(token_type) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn consume(&mut self, token_type: Token, message: &str) -> Result<&Token> {
        if self.check(&token_type) {
            Ok(self.advance())
        } else {
            Err(FastAwkError::syntax_error(format!(
                "{} - found {}",
                message,
                self.peek()
            )))
        }
    }

    fn skip_newlines(&mut self) {
        while self.match_token(&Token::Newline) {
            // Skip newlines
        }
    }

    fn check_statement_terminator(&self) -> bool {
        matches!(self.peek(), Token::Newline | Token::Semicolon | Token::RightBrace | Token::Eof)
    }

    fn consume_statement_terminator(&mut self) -> Result<()> {
        if self.match_token(&Token::Semicolon) || self.match_token(&Token::Newline) {
            Ok(())
        } else if matches!(self.peek(), Token::RightBrace | Token::Eof) {
            Ok(())
        } else {
            Err(FastAwkError::syntax_error("Expected ';' or newline"))
        }
    }

    fn check_assignment_operator(&self) -> bool {
        matches!(
            self.peek(),
            Token::Assign | Token::PlusAssign | Token::MinusAssign |
            Token::MultiplyAssign | Token::DivideAssign | Token::ModuloAssign |
            Token::PowerAssign
        )
    }

    fn is_concatenation_context(&self) -> bool {
        // Simplified concatenation detection
        matches!(
            self.peek(),
            Token::Identifier(_) | Token::String(_) | Token::Number(_) |
            Token::Dollar | Token::LeftParen
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_expression() {
        let mut parser = Parser::new("1 + 2").unwrap();
        let expr = parser.parse_expression().unwrap();
        
        match expr {
            Expression::Add(left, right) => {
                assert_eq!(*left, Expression::Literal(Value::Number(1.0)));
                assert_eq!(*right, Expression::Literal(Value::Number(2.0)));
            }
            _ => panic!("Expected Add expression"),
        }
    }

    #[test]
    fn test_begin_rule() {
        let mut parser = Parser::new("BEGIN { print \"hello\" }").unwrap();
        let program = parser.parse().unwrap();
        
        assert_eq!(program.rules.len(), 1);
        assert!(matches!(program.rules[0].pattern, Some(Pattern::Begin)));
    }

    #[test]
    fn test_field_reference() {
        let mut parser = Parser::new("$1").unwrap();
        let expr = parser.parse_expression().unwrap();
        
        match expr {
            Expression::FieldRef(field_expr) => {
                assert_eq!(*field_expr, Expression::Literal(Value::Number(1.0)));
            }
            _ => panic!("Expected FieldRef expression"),
        }
    }

    #[test]
    fn test_function_call() {
        let mut parser = Parser::new("substr(\"hello\", 1, 3)").unwrap();
        let expr = parser.parse_expression().unwrap();
        
        match expr {
            Expression::FunctionCall { name, arguments } => {
                assert_eq!(name, "substr");
                assert_eq!(arguments.len(), 3);
            }
            _ => panic!("Expected FunctionCall expression"),
        }
    }
}