use crate::value::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub rules: Vec<Rule>,
    pub functions: HashMap<String, Function>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    pub pattern: Option<Pattern>,
    pub action: Action,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Begin,
    End,
    Expression(Expression),
    Range(Box<Pattern>, Box<Pattern>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Action {
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Expression(Expression),
    Block(Vec<Statement>),
    If {
        condition: Expression,
        then_stmt: Box<Statement>,
        else_stmt: Option<Box<Statement>>,
    },
    While {
        condition: Expression,
        body: Box<Statement>,
    },
    For {
        init: Option<Expression>,
        condition: Option<Expression>,
        update: Option<Expression>,
        body: Box<Statement>,
    },
    ForIn {
        variable: String,
        array: Expression,
        body: Box<Statement>,
    },
    Break,
    Continue,
    Next,
    Exit(Option<Expression>),
    Return(Option<Expression>),
    Delete(Expression),
    Print(PrintStatement),
    Printf(PrintfStatement),
}

#[derive(Debug, Clone, PartialEq)]
pub struct PrintStatement {
    pub expressions: Vec<Expression>,
    pub output_target: Option<OutputTarget>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PrintfStatement {
    pub format: Expression,
    pub arguments: Vec<Expression>,
    pub output_target: Option<OutputTarget>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OutputTarget {
    File(Expression),
    Pipe(Expression),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Literal(Value),
    Identifier(String),
    FieldRef(Box<Expression>),
    ArrayRef {
        array: Box<Expression>,
        index: Box<Expression>,
    },

    // Arithmetic operations
    Add(Box<Expression>, Box<Expression>),
    Subtract(Box<Expression>, Box<Expression>),
    Multiply(Box<Expression>, Box<Expression>),
    Divide(Box<Expression>, Box<Expression>),
    Modulo(Box<Expression>, Box<Expression>),
    Power(Box<Expression>, Box<Expression>),
    UnaryMinus(Box<Expression>),
    UnaryPlus(Box<Expression>),

    // Comparison operations
    Equal(Box<Expression>, Box<Expression>),
    NotEqual(Box<Expression>, Box<Expression>),
    Less(Box<Expression>, Box<Expression>),
    LessEqual(Box<Expression>, Box<Expression>),
    Greater(Box<Expression>, Box<Expression>),
    GreaterEqual(Box<Expression>, Box<Expression>),
    Match(Box<Expression>, Box<Expression>),
    NotMatch(Box<Expression>, Box<Expression>),

    // Logical operations
    And(Box<Expression>, Box<Expression>),
    Or(Box<Expression>, Box<Expression>),
    Not(Box<Expression>),

    // String operations
    Concatenate(Box<Expression>, Box<Expression>),
    In(Box<Expression>, Box<Expression>),

    // Assignment operations
    Assign(Box<Expression>, Box<Expression>),
    AddAssign(Box<Expression>, Box<Expression>),
    SubtractAssign(Box<Expression>, Box<Expression>),
    MultiplyAssign(Box<Expression>, Box<Expression>),
    DivideAssign(Box<Expression>, Box<Expression>),
    ModuloAssign(Box<Expression>, Box<Expression>),
    PowerAssign(Box<Expression>, Box<Expression>),

    // Increment/Decrement
    PreIncrement(Box<Expression>),
    PostIncrement(Box<Expression>),
    PreDecrement(Box<Expression>),
    PostDecrement(Box<Expression>),

    // Conditional expression
    Ternary {
        condition: Box<Expression>,
        true_expr: Box<Expression>,
        false_expr: Box<Expression>,
    },

    // Function call
    FunctionCall {
        name: String,
        arguments: Vec<Expression>,
    },

    // Getline expression
    Getline {
        target: Option<Box<Expression>>,
        source: Option<Box<Expression>>,
    },

    // Regular expression literal
    Regex(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub parameters: Vec<String>,
    pub body: Action,
}

impl Program {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            functions: HashMap::new(),
        }
    }

    pub fn add_rule(&mut self, rule: Rule) {
        self.rules.push(rule);
    }

    pub fn add_function(&mut self, function: Function) {
        self.functions.insert(function.name.clone(), function);
    }

    pub fn has_begin_rules(&self) -> bool {
        self.rules.iter().any(|rule| matches!(rule.pattern, Some(Pattern::Begin)))
    }

    pub fn has_end_rules(&self) -> bool {
        self.rules.iter().any(|rule| matches!(rule.pattern, Some(Pattern::End)))
    }

    pub fn get_begin_rules(&self) -> Vec<&Rule> {
        self.rules.iter()
            .filter(|rule| matches!(rule.pattern, Some(Pattern::Begin)))
            .collect()
    }

    pub fn get_end_rules(&self) -> Vec<&Rule> {
        self.rules.iter()
            .filter(|rule| matches!(rule.pattern, Some(Pattern::End)))
            .collect()
    }

    pub fn get_main_rules(&self) -> Vec<&Rule> {
        self.rules.iter()
            .filter(|rule| !matches!(rule.pattern, Some(Pattern::Begin) | Some(Pattern::End)))
            .collect()
    }
}

impl Action {
    pub fn new() -> Self {
        Self {
            statements: Vec::new(),
        }
    }

    pub fn add_statement(&mut self, statement: Statement) {
        self.statements.push(statement);
    }

    pub fn is_empty(&self) -> bool {
        self.statements.is_empty()
    }
}

impl PrintStatement {
    pub fn new() -> Self {
        Self {
            expressions: Vec::new(),
            output_target: None,
        }
    }

    pub fn with_expressions(expressions: Vec<Expression>) -> Self {
        Self {
            expressions,
            output_target: None,
        }
    }

    pub fn with_target(mut self, target: OutputTarget) -> Self {
        self.output_target = Some(target);
        self
    }
}

impl PrintfStatement {
    pub fn new(format: Expression) -> Self {
        Self {
            format,
            arguments: Vec::new(),
            output_target: None,
        }
    }

    pub fn with_arguments(mut self, arguments: Vec<Expression>) -> Self {
        self.arguments = arguments;
        self
    }

    pub fn with_target(mut self, target: OutputTarget) -> Self {
        self.output_target = Some(target);
        self
    }
}

impl Expression {
    /// Check if this expression is an lvalue (can be assigned to)
    pub fn is_lvalue(&self) -> bool {
        match self {
            Expression::Identifier(_) => true,
            Expression::FieldRef(_) => true,
            Expression::ArrayRef { .. } => true,
            _ => false,
        }
    }

    /// Get the precedence of this expression for parsing
    pub fn precedence(&self) -> u8 {
        match self {
            Expression::Or(_, _) => 1,
            Expression::And(_, _) => 2,
            Expression::In(_, _) => 3,
            Expression::Match(_, _) | Expression::NotMatch(_, _) => 4,
            Expression::Equal(_, _) | Expression::NotEqual(_, _) => 5,
            Expression::Less(_, _) | Expression::LessEqual(_, _) |
            Expression::Greater(_, _) | Expression::GreaterEqual(_, _) => 6,
            Expression::Concatenate(_, _) => 7,
            Expression::Add(_, _) | Expression::Subtract(_, _) => 8,
            Expression::Multiply(_, _) | Expression::Divide(_, _) | Expression::Modulo(_, _) => 9,
            Expression::Power(_, _) => 10,
            Expression::UnaryMinus(_) | Expression::UnaryPlus(_) | Expression::Not(_) => 11,
            Expression::PreIncrement(_) | Expression::PostIncrement(_) |
            Expression::PreDecrement(_) | Expression::PostDecrement(_) => 12,
            Expression::FieldRef(_) => 13,
            Expression::FunctionCall { .. } => 14,
            Expression::ArrayRef { .. } => 15,
            _ => 0,
        }
    }

    /// Check if this expression has side effects
    pub fn has_side_effects(&self) -> bool {
        match self {
            Expression::Assign(_, _) | Expression::AddAssign(_, _) | Expression::SubtractAssign(_, _) |
            Expression::MultiplyAssign(_, _) | Expression::DivideAssign(_, _) |
            Expression::ModuloAssign(_, _) | Expression::PowerAssign(_, _) => true,
            Expression::PreIncrement(_) | Expression::PostIncrement(_) |
            Expression::PreDecrement(_) | Expression::PostDecrement(_) => true,
            Expression::FunctionCall { .. } => true, // Functions may have side effects
            Expression::Getline { .. } => true,
            
            // Recursive check for compound expressions
            Expression::Add(left, right) | Expression::Subtract(left, right) |
            Expression::Multiply(left, right) | Expression::Divide(left, right) |
            Expression::Modulo(left, right) | Expression::Power(left, right) |
            Expression::Equal(left, right) | Expression::NotEqual(left, right) |
            Expression::Less(left, right) | Expression::LessEqual(left, right) |
            Expression::Greater(left, right) | Expression::GreaterEqual(left, right) |
            Expression::Match(left, right) | Expression::NotMatch(left, right) |
            Expression::And(left, right) | Expression::Or(left, right) |
            Expression::Concatenate(left, right) | Expression::In(left, right) => {
                left.has_side_effects() || right.has_side_effects()
            }
            
            Expression::UnaryMinus(expr) | Expression::UnaryPlus(expr) | Expression::Not(expr) => {
                expr.has_side_effects()
            }
            
            Expression::Ternary { condition, true_expr, false_expr } => {
                condition.has_side_effects() || true_expr.has_side_effects() || false_expr.has_side_effects()
            }
            
            Expression::FieldRef(expr) => expr.has_side_effects(),
            Expression::ArrayRef { array, index } => array.has_side_effects() || index.has_side_effects(),
            
            _ => false,
        }
    }
}

impl Default for Program {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for Action {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_program_creation() {
        let mut program = Program::new();
        
        let rule = Rule {
            pattern: Some(Pattern::Begin),
            action: Action::new(),
        };
        
        program.add_rule(rule);
        assert!(program.has_begin_rules());
        assert!(!program.has_end_rules());
        assert_eq!(program.get_begin_rules().len(), 1);
    }

    #[test]
    fn test_expression_precedence() {
        let add_expr = Expression::Add(
            Box::new(Expression::Literal(Value::Number(1.0))),
            Box::new(Expression::Literal(Value::Number(2.0))),
        );
        let multiply_expr = Expression::Multiply(
            Box::new(Expression::Literal(Value::Number(3.0))),
            Box::new(Expression::Literal(Value::Number(4.0))),
        );
        
        assert!(multiply_expr.precedence() > add_expr.precedence());
    }

    #[test]
    fn test_lvalue_detection() {
        let identifier = Expression::Identifier("var".to_string());
        let field_ref = Expression::FieldRef(Box::new(Expression::Literal(Value::Number(1.0))));
        let literal = Expression::Literal(Value::Number(42.0));
        
        assert!(identifier.is_lvalue());
        assert!(field_ref.is_lvalue());
        assert!(!literal.is_lvalue());
    }

    #[test]
    fn test_side_effects_detection() {
        let assign_expr = Expression::Assign(
            Box::new(Expression::Identifier("var".to_string())),
            Box::new(Expression::Literal(Value::Number(42.0))),
        );
        let add_expr = Expression::Add(
            Box::new(Expression::Literal(Value::Number(1.0))),
            Box::new(Expression::Literal(Value::Number(2.0))),
        );
        
        assert!(assign_expr.has_side_effects());
        assert!(!add_expr.has_side_effects());
    }
}