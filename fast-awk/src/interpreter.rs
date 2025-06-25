use crate::ast::*;
use crate::errors::{FastAwkError, Result};
use crate::runtime::{RuntimeContext, ControlFlow};
use crate::value::Value;
use std::collections::HashMap;

pub struct Interpreter {
    pub context: RuntimeContext,
    functions: HashMap<String, Function>,
}

impl Interpreter {
    pub fn new() -> Self {
        Self {
            context: RuntimeContext::new(),
            functions: HashMap::new(),
        }
    }

    pub fn execute_program(&mut self, program: &Program) -> Result<()> {
        // Store user-defined functions
        self.functions = program.functions.clone();

        // Execute BEGIN rules
        for rule in program.get_begin_rules() {
            self.execute_action(&rule.action)?;
            if self.context.has_control_flow() {
                match &self.context.control_flow {
                    ControlFlow::Exit(_) => return Ok(()),
                    _ => self.context.clear_control_flow(),
                }
            }
        }

        Ok(())
    }

    pub fn execute_main_rules(&mut self, program: &Program, record: &str) -> Result<bool> {
        if self.context.has_control_flow() {
            if matches!(self.context.control_flow, ControlFlow::Exit(_)) {
                return Ok(false);
            }
        }

        self.context.set_current_record(record);
        let mut any_matched = false;

        for rule in program.get_main_rules() {
            let matches = if let Some(ref pattern) = rule.pattern {
                self.evaluate_pattern(pattern)?
            } else {
                true // No pattern means always match
            };

            if matches {
                any_matched = true;
                self.execute_action(&rule.action)?;
                
                match &self.context.control_flow {
                    ControlFlow::Next => {
                        self.context.clear_control_flow();
                        break;
                    }
                    ControlFlow::Exit(_) => return Ok(false),
                    _ => {}
                }
            }
        }

        Ok(any_matched)
    }

    pub fn execute_end_rules(&mut self, program: &Program) -> Result<()> {
        for rule in program.get_end_rules() {
            self.execute_action(&rule.action)?;
            if matches!(self.context.control_flow, ControlFlow::Exit(_)) {
                break;
            }
        }
        Ok(())
    }

    fn evaluate_pattern(&mut self, pattern: &Pattern) -> Result<bool> {
        match pattern {
            Pattern::Begin | Pattern::End => Ok(false), // Should not be called for these
            Pattern::Expression(expr) => {
                let value = self.evaluate_expression(expr)?;
                Ok(value.to_bool())
            }
            Pattern::Range(start, end) => {
                // Range patterns are more complex and would need state tracking
                // For now, simplified implementation
                let start_matches = self.evaluate_pattern(start)?;
                let end_matches = self.evaluate_pattern(end)?;
                Ok(start_matches || end_matches)
            }
        }
    }

    fn execute_action(&mut self, action: &Action) -> Result<()> {
        for statement in &action.statements {
            self.execute_statement(statement)?;
            
            match &self.context.control_flow {
                ControlFlow::Break | ControlFlow::Continue => break,
                ControlFlow::Next | ControlFlow::Exit(_) | ControlFlow::Return(_) => break,
                ControlFlow::None => {}
            }
        }
        Ok(())
    }

    fn execute_statement(&mut self, statement: &Statement) -> Result<()> {
        match statement {
            Statement::Expression(expr) => {
                self.evaluate_expression(expr)?;
            }
            Statement::Block(statements) => {
                for stmt in statements {
                    self.execute_statement(stmt)?;
                    if self.context.has_control_flow() {
                        break;
                    }
                }
            }
            Statement::If { condition, then_stmt, else_stmt } => {
                let condition_value = self.evaluate_expression(condition)?;
                if condition_value.to_bool() {
                    self.execute_statement(then_stmt)?;
                } else if let Some(else_stmt) = else_stmt {
                    self.execute_statement(else_stmt)?;
                }
            }
            Statement::While { condition, body } => {
                while !self.context.has_control_flow() {
                    let condition_value = self.evaluate_expression(condition)?;
                    if !condition_value.to_bool() {
                        break;
                    }
                    
                    self.execute_statement(body)?;
                    
                    match &self.context.control_flow {
                        ControlFlow::Break => {
                            self.context.clear_control_flow();
                            break;
                        }
                        ControlFlow::Continue => {
                            self.context.clear_control_flow();
                            continue;
                        }
                        ControlFlow::Next | ControlFlow::Exit(_) | ControlFlow::Return(_) => break,
                        ControlFlow::None => {}
                    }
                }
            }
            Statement::For { init, condition, update, body } => {
                if let Some(init) = init {
                    self.evaluate_expression(init)?;
                }
                
                while !self.context.has_control_flow() {
                    if let Some(condition) = condition {
                        let condition_value = self.evaluate_expression(condition)?;
                        if !condition_value.to_bool() {
                            break;
                        }
                    }
                    
                    self.execute_statement(body)?;
                    
                    match &self.context.control_flow {
                        ControlFlow::Break => {
                            self.context.clear_control_flow();
                            break;
                        }
                        ControlFlow::Continue => {
                            self.context.clear_control_flow();
                            if let Some(update) = update {
                                self.evaluate_expression(update)?;
                            }
                            continue;
                        }
                        ControlFlow::Next | ControlFlow::Exit(_) | ControlFlow::Return(_) => break,
                        ControlFlow::None => {}
                    }
                    
                    if let Some(update) = update {
                        self.evaluate_expression(update)?;
                    }
                }
            }
            Statement::ForIn { variable, array, body } => {
                let array_value = self.evaluate_expression(array)?;
                if let Value::Array(_) = array_value {
                    let keys = array_value.array_keys();
                    for key in keys {
                        if self.context.has_control_flow() {
                            break;
                        }
                        
                        self.context.set_variable(variable, Value::String(key));
                        self.execute_statement(body)?;
                        
                        match &self.context.control_flow {
                            ControlFlow::Break => {
                                self.context.clear_control_flow();
                                break;
                            }
                            ControlFlow::Continue => {
                                self.context.clear_control_flow();
                                continue;
                            }
                            ControlFlow::Next | ControlFlow::Exit(_) | ControlFlow::Return(_) => break,
                            ControlFlow::None => {}
                        }
                    }
                }
            }
            Statement::Break => {
                self.context.set_control_flow(ControlFlow::Break);
            }
            Statement::Continue => {
                self.context.set_control_flow(ControlFlow::Continue);
            }
            Statement::Next => {
                self.context.set_control_flow(ControlFlow::Next);
            }
            Statement::Exit(expr) => {
                let exit_code = if let Some(expr) = expr {
                    self.evaluate_expression(expr)?.to_number() as i32
                } else {
                    0
                };
                self.context.set_exit_code(exit_code);
            }
            Statement::Return(expr) => {
                let return_value = if let Some(expr) = expr {
                    self.evaluate_expression(expr)?
                } else {
                    Value::Undefined
                };
                self.context.set_control_flow(ControlFlow::Return(return_value));
            }
            Statement::Delete(expr) => {
                // Simplified delete implementation
                match expr {
                    Expression::Identifier(name) => {
                        self.context.set_variable(name, Value::Undefined);
                    }
                    Expression::ArrayRef { array, index } => {
                        let _array_value = self.evaluate_expression(array)?;
                        let _index_value = self.evaluate_expression(index)?;
                        // In a full implementation, this would remove the array element
                    }
                    _ => return Err(FastAwkError::runtime_error("Invalid delete target")),
                }
            }
            Statement::Print(print_stmt) => {
                let mut values = Vec::new();
                for expr in &print_stmt.expressions {
                    values.push(self.evaluate_expression(expr)?);
                }
                
                // Handle output redirection in a full implementation
                self.context.print_values(&values)?;
            }
            Statement::Printf(printf_stmt) => {
                let format = self.evaluate_expression(&printf_stmt.format)?;
                let mut args = Vec::new();
                for expr in &printf_stmt.arguments {
                    args.push(self.evaluate_expression(expr)?);
                }
                
                self.context.printf_format(&format, &args)?;
            }
        }
        
        Ok(())
    }

    fn evaluate_expression(&mut self, expression: &Expression) -> Result<Value> {
        match expression {
            Expression::Literal(value) => Ok(value.clone()),
            
            Expression::Identifier(name) => Ok(self.context.get_variable(name)),
            
            Expression::FieldRef(expr) => {
                let index_value = self.evaluate_expression(expr)?;
                let index = index_value.to_number() as usize;
                Ok(Value::String(self.context.get_field(index)))
            }
            
            Expression::ArrayRef { array, index } => {
                let mut array_value = self.evaluate_expression(array)?;
                let index_value = self.evaluate_expression(index)?;
                let index_str = index_value.to_string();
                
                let element = array_value.get_array_element(&index_str);
                Ok(element.clone())
            }
            
            // Arithmetic operations
            Expression::Add(left, right) => {
                let left_val = self.evaluate_expression(left)?;
                let right_val = self.evaluate_expression(right)?;
                left_val.add(&right_val)
            }
            
            Expression::Subtract(left, right) => {
                let left_val = self.evaluate_expression(left)?;
                let right_val = self.evaluate_expression(right)?;
                left_val.subtract(&right_val)
            }
            
            Expression::Multiply(left, right) => {
                let left_val = self.evaluate_expression(left)?;
                let right_val = self.evaluate_expression(right)?;
                left_val.multiply(&right_val)
            }
            
            Expression::Divide(left, right) => {
                let left_val = self.evaluate_expression(left)?;
                let right_val = self.evaluate_expression(right)?;
                left_val.divide(&right_val)
            }
            
            Expression::Modulo(left, right) => {
                let left_val = self.evaluate_expression(left)?;
                let right_val = self.evaluate_expression(right)?;
                left_val.modulo(&right_val)
            }
            
            Expression::Power(left, right) => {
                let left_val = self.evaluate_expression(left)?;
                let right_val = self.evaluate_expression(right)?;
                left_val.power(&right_val)
            }
            
            Expression::UnaryMinus(expr) => {
                let value = self.evaluate_expression(expr)?;
                Ok(Value::Number(-value.to_number()))
            }
            
            Expression::UnaryPlus(expr) => {
                let value = self.evaluate_expression(expr)?;
                Ok(Value::Number(value.to_number()))
            }
            
            // Comparison operations
            Expression::Equal(left, right) => {
                let left_val = self.evaluate_expression(left)?;
                let right_val = self.evaluate_expression(right)?;
                Ok(Value::Number(if left_val.compare(&right_val) == std::cmp::Ordering::Equal { 1.0 } else { 0.0 }))
            }
            
            Expression::NotEqual(left, right) => {
                let left_val = self.evaluate_expression(left)?;
                let right_val = self.evaluate_expression(right)?;
                Ok(Value::Number(if left_val.compare(&right_val) != std::cmp::Ordering::Equal { 1.0 } else { 0.0 }))
            }
            
            Expression::Less(left, right) => {
                let left_val = self.evaluate_expression(left)?;
                let right_val = self.evaluate_expression(right)?;
                Ok(Value::Number(if left_val.compare(&right_val) == std::cmp::Ordering::Less { 1.0 } else { 0.0 }))
            }
            
            Expression::LessEqual(left, right) => {
                let left_val = self.evaluate_expression(left)?;
                let right_val = self.evaluate_expression(right)?;
                Ok(Value::Number(if left_val.compare(&right_val) != std::cmp::Ordering::Greater { 1.0 } else { 0.0 }))
            }
            
            Expression::Greater(left, right) => {
                let left_val = self.evaluate_expression(left)?;
                let right_val = self.evaluate_expression(right)?;
                Ok(Value::Number(if left_val.compare(&right_val) == std::cmp::Ordering::Greater { 1.0 } else { 0.0 }))
            }
            
            Expression::GreaterEqual(left, right) => {
                let left_val = self.evaluate_expression(left)?;
                let right_val = self.evaluate_expression(right)?;
                Ok(Value::Number(if left_val.compare(&right_val) != std::cmp::Ordering::Less { 1.0 } else { 0.0 }))
            }
            
            Expression::Match(left, right) => {
                let string_val = self.evaluate_expression(left)?;
                let pattern_val = self.evaluate_expression(right)?;
                let pattern = pattern_val.to_string();
                
                let regex = self.context.get_regex(&pattern)?;
                Ok(Value::Number(if string_val.regex_match(&regex) { 1.0 } else { 0.0 }))
            }
            
            Expression::NotMatch(left, right) => {
                let string_val = self.evaluate_expression(left)?;
                let pattern_val = self.evaluate_expression(right)?;
                let pattern = pattern_val.to_string();
                
                let regex = self.context.get_regex(&pattern)?;
                Ok(Value::Number(if !string_val.regex_match(&regex) { 1.0 } else { 0.0 }))
            }
            
            // Logical operations
            Expression::And(left, right) => {
                let left_val = self.evaluate_expression(left)?;
                if !left_val.to_bool() {
                    Ok(Value::Number(0.0))
                } else {
                    let right_val = self.evaluate_expression(right)?;
                    Ok(Value::Number(if right_val.to_bool() { 1.0 } else { 0.0 }))
                }
            }
            
            Expression::Or(left, right) => {
                let left_val = self.evaluate_expression(left)?;
                if left_val.to_bool() {
                    Ok(Value::Number(1.0))
                } else {
                    let right_val = self.evaluate_expression(right)?;
                    Ok(Value::Number(if right_val.to_bool() { 1.0 } else { 0.0 }))
                }
            }
            
            Expression::Not(expr) => {
                let value = self.evaluate_expression(expr)?;
                Ok(Value::Number(if !value.to_bool() { 1.0 } else { 0.0 }))
            }
            
            // String operations
            Expression::Concatenate(left, right) => {
                let left_val = self.evaluate_expression(left)?;
                let right_val = self.evaluate_expression(right)?;
                Ok(left_val.concatenate(&right_val))
            }
            
            Expression::In(left, right) => {
                let key_val = self.evaluate_expression(left)?;
                let array_val = self.evaluate_expression(right)?;
                let key_str = key_val.to_string();
                
                Ok(Value::Number(if array_val.has_array_key(&key_str) { 1.0 } else { 0.0 }))
            }
            
            // Assignment operations
            Expression::Assign(left, right) => {
                let value = self.evaluate_expression(right)?;
                self.assign_to_lvalue(left, value.clone())?;
                Ok(value)
            }
            
            Expression::AddAssign(left, right) => {
                let left_val = self.evaluate_lvalue(left)?;
                let right_val = self.evaluate_expression(right)?;
                let result = left_val.add(&right_val)?;
                self.assign_to_lvalue(left, result.clone())?;
                Ok(result)
            }
            
            Expression::SubtractAssign(left, right) => {
                let left_val = self.evaluate_lvalue(left)?;
                let right_val = self.evaluate_expression(right)?;
                let result = left_val.subtract(&right_val)?;
                self.assign_to_lvalue(left, result.clone())?;
                Ok(result)
            }
            
            Expression::MultiplyAssign(left, right) => {
                let left_val = self.evaluate_lvalue(left)?;
                let right_val = self.evaluate_expression(right)?;
                let result = left_val.multiply(&right_val)?;
                self.assign_to_lvalue(left, result.clone())?;
                Ok(result)
            }
            
            Expression::DivideAssign(left, right) => {
                let left_val = self.evaluate_lvalue(left)?;
                let right_val = self.evaluate_expression(right)?;
                let result = left_val.divide(&right_val)?;
                self.assign_to_lvalue(left, result.clone())?;
                Ok(result)
            }
            
            Expression::ModuloAssign(left, right) => {
                let left_val = self.evaluate_lvalue(left)?;
                let right_val = self.evaluate_expression(right)?;
                let result = left_val.modulo(&right_val)?;
                self.assign_to_lvalue(left, result.clone())?;
                Ok(result)
            }
            
            Expression::PowerAssign(left, right) => {
                let left_val = self.evaluate_lvalue(left)?;
                let right_val = self.evaluate_expression(right)?;
                let result = left_val.power(&right_val)?;
                self.assign_to_lvalue(left, result.clone())?;
                Ok(result)
            }
            
            // Increment/Decrement
            Expression::PreIncrement(expr) => {
                let current = self.evaluate_lvalue(expr)?;
                let result = current.add(&Value::Number(1.0))?;
                self.assign_to_lvalue(expr, result.clone())?;
                Ok(result)
            }
            
            Expression::PostIncrement(expr) => {
                let current = self.evaluate_lvalue(expr)?;
                let result = current.add(&Value::Number(1.0))?;
                self.assign_to_lvalue(expr, result)?;
                Ok(current)
            }
            
            Expression::PreDecrement(expr) => {
                let current = self.evaluate_lvalue(expr)?;
                let result = current.subtract(&Value::Number(1.0))?;
                self.assign_to_lvalue(expr, result.clone())?;
                Ok(result)
            }
            
            Expression::PostDecrement(expr) => {
                let current = self.evaluate_lvalue(expr)?;
                let result = current.subtract(&Value::Number(1.0))?;
                self.assign_to_lvalue(expr, result)?;
                Ok(current)
            }
            
            // Conditional expression
            Expression::Ternary { condition, true_expr, false_expr } => {
                let condition_val = self.evaluate_expression(condition)?;
                if condition_val.to_bool() {
                    self.evaluate_expression(true_expr)
                } else {
                    self.evaluate_expression(false_expr)
                }
            }
            
            // Function call
            Expression::FunctionCall { name, arguments } => {
                let mut arg_values = Vec::new();
                for arg in arguments {
                    arg_values.push(self.evaluate_expression(arg)?);
                }
                
                self.call_function(name, &arg_values)
            }
            
            // Getline expression
            Expression::Getline { target: _, source: _ } => {
                // Simplified getline - in a full implementation this would read from input
                Ok(Value::Number(0.0))
            }
            
            // Regular expression literal
            Expression::Regex(pattern) => {
                let record = self.context.get_field(0);
                let regex = self.context.get_regex(pattern)?;
                Ok(Value::Number(if regex.is_match(&record) { 1.0 } else { 0.0 }))
            }
        }
    }

    fn evaluate_lvalue(&mut self, expr: &Expression) -> Result<Value> {
        match expr {
            Expression::Identifier(name) => Ok(self.context.get_variable(name)),
            Expression::FieldRef(field_expr) => {
                let index_value = self.evaluate_expression(field_expr)?;
                let index = index_value.to_number() as usize;
                Ok(Value::String(self.context.get_field(index)))
            }
            Expression::ArrayRef { array, index } => {
                let mut array_value = self.evaluate_expression(array)?;
                let index_value = self.evaluate_expression(index)?;
                let index_str = index_value.to_string();
                
                let element = array_value.get_array_element(&index_str);
                Ok(element.clone())
            }
            _ => Err(FastAwkError::runtime_error("Invalid lvalue")),
        }
    }

    fn assign_to_lvalue(&mut self, expr: &Expression, value: Value) -> Result<()> {
        match expr {
            Expression::Identifier(name) => {
                self.context.set_variable(name, value);
                Ok(())
            }
            Expression::FieldRef(field_expr) => {
                let index_value = self.evaluate_expression(field_expr)?;
                let index = index_value.to_number() as usize;
                self.context.set_field(index, value.to_string());
                Ok(())
            }
            Expression::ArrayRef { array: _, index: _ } => {
                // In a full implementation, this would handle array assignment properly
                Ok(())
            }
            _ => Err(FastAwkError::runtime_error("Invalid assignment target")),
        }
    }

    fn call_function(&mut self, name: &str, args: &[Value]) -> Result<Value> {
        // Check built-in functions first
        match name {
            "length" => self.context.builtin_length(args),
            "substr" => self.context.builtin_substr(args),
            "index" => self.context.builtin_index(args),
            "split" => self.context.builtin_split(args),
            "gsub" => self.context.builtin_gsub(args),
            "sub" => self.context.builtin_sub(args),
            "match" => self.context.builtin_match(args),
            "sprintf" => self.context.builtin_sprintf(args),
            "toupper" => self.context.builtin_toupper(args),
            "tolower" => self.context.builtin_tolower(args),
            "sin" => self.context.builtin_sin(args),
            "cos" => self.context.builtin_cos(args),
            "atan2" => self.context.builtin_atan2(args),
            "exp" => self.context.builtin_exp(args),
            "log" => self.context.builtin_log(args),
            "sqrt" => self.context.builtin_sqrt(args),
            "int" => self.context.builtin_int(args),
            "rand" => self.context.builtin_rand(args),
            "srand" => self.context.builtin_srand(args),
            _ => {
                // Check user-defined functions
                if let Some(function) = self.functions.get(name).cloned() {
                    self.call_user_function(&function, args)
                } else {
                    Err(FastAwkError::undefined_function(name))
                }
            }
        }
    }

    fn call_user_function(&mut self, function: &Function, args: &[Value]) -> Result<Value> {
        // Create new call frame
        self.context.push_call_frame(function.name.clone());
        
        // Set parameter values
        for (i, param) in function.parameters.iter().enumerate() {
            let value = args.get(i).cloned().unwrap_or(Value::Undefined);
            self.context.set_variable(param, value);
        }
        
        // Execute function body
        self.execute_action(&function.body)?;
        
        // Get return value
        let return_value = match &self.context.control_flow {
            ControlFlow::Return(value) => value.clone(),
            _ => Value::Undefined,
        };
        
        // Clean up
        self.context.pop_call_frame();
        self.context.clear_control_flow();
        
        Ok(return_value)
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;

    #[test]
    fn test_simple_expression() {
        let mut interpreter = Interpreter::new();
        let expr = Expression::Add(
            Box::new(Expression::Literal(Value::Number(1.0))),
            Box::new(Expression::Literal(Value::Number(2.0))),
        );
        
        let result = interpreter.evaluate_expression(&expr).unwrap();
        assert_eq!(result, Value::Number(3.0));
    }

    #[test]
    fn test_field_reference() {
        let mut interpreter = Interpreter::new();
        interpreter.context.set_current_record("hello world");
        
        let expr = Expression::FieldRef(Box::new(Expression::Literal(Value::Number(1.0))));
        let result = interpreter.evaluate_expression(&expr).unwrap();
        assert_eq!(result, Value::String("hello".to_string()));
    }

    #[test]
    fn test_variable_assignment() {
        let mut interpreter = Interpreter::new();
        
        let expr = Expression::Assign(
            Box::new(Expression::Identifier("x".to_string())),
            Box::new(Expression::Literal(Value::Number(42.0))),
        );
        
        interpreter.evaluate_expression(&expr).unwrap();
        assert_eq!(interpreter.context.get_variable("x"), Value::Number(42.0));
    }

    #[test]
    fn test_function_call() {
        let mut interpreter = Interpreter::new();
        
        let expr = Expression::FunctionCall {
            name: "length".to_string(),
            arguments: vec![Expression::Literal(Value::String("hello".to_string()))],
        };
        
        let result = interpreter.evaluate_expression(&expr).unwrap();
        assert_eq!(result, Value::Number(5.0));
    }

    #[test]
    fn test_simple_program() {
        let mut parser = Parser::new("BEGIN { print \"Hello, World!\" }").unwrap();
        let program = parser.parse().unwrap();
        
        let mut interpreter = Interpreter::new();
        // Note: This would print "Hello, World!" in a real run
        let result = interpreter.execute_program(&program);
        assert!(result.is_ok());
    }
}