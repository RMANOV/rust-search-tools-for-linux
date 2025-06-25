use std::path::PathBuf;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, FastAwkError>;

#[derive(Error, Debug)]
pub enum FastAwkError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("CSV processing error: {0}")]
    Csv(#[from] csv::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Regex compilation error: {0}")]
    Regex(#[from] regex::Error),

    #[error("File not found: {path}")]
    FileNotFound { path: PathBuf },

    #[error("Script parsing error at line {line}, column {column}: {message}")]
    ParseError {
        line: usize,
        column: usize,
        message: String,
    },

    #[error("Runtime error: {message}")]
    Runtime { message: String },

    #[error("Invalid field reference: {field}")]
    InvalidFieldReference { field: String },

    #[error("Division by zero in expression")]
    DivisionByZero,

    #[error("Type error: cannot {operation} {left_type} and {right_type}")]
    TypeError {
        operation: String,
        left_type: String,
        right_type: String,
    },

    #[error("Variable '{name}' is not defined")]
    UndefinedVariable { name: String },

    #[error("Function '{name}' is not defined")]
    UndefinedFunction { name: String },

    #[error("Invalid function call: {function}({args}) - {reason}")]
    InvalidFunctionCall {
        function: String,
        args: String,
        reason: String,
    },

    #[error("Array index out of bounds: {index} (array size: {size})")]
    ArrayIndexOutOfBounds { index: i64, size: usize },

    #[error("Invalid array index: {index} (must be integer)")]
    InvalidArrayIndex { index: String },

    #[error("Invalid assignment: {message}")]
    InvalidAssignment { message: String },

    #[error("Control flow error: {message}")]
    ControlFlow { message: String },

    #[error("Encoding error: {message}")]
    EncodingError { message: String },

    #[error("Pattern matching error: {message}")]
    PatternError { message: String },

    #[error("Memory limit exceeded: {current} bytes (limit: {limit} bytes)")]
    MemoryLimitExceeded { current: usize, limit: usize },

    #[error("Script execution timeout")]
    ExecutionTimeout,

    #[error("Invalid script syntax: {message}")]
    SyntaxError { message: String },

    #[error("Built-in variable '{name}' cannot be modified")]
    ReadOnlyVariable { name: String },

    #[error("Invalid format specifier: {format}")]
    InvalidFormatSpecifier { format: String },

    #[error("General error: {0}")]
    General(String),
}

impl FastAwkError {
    pub fn file_not_found(path: PathBuf) -> Self {
        Self::FileNotFound { path }
    }

    pub fn parse_error(line: usize, column: usize, message: impl Into<String>) -> Self {
        Self::ParseError {
            line,
            column,
            message: message.into(),
        }
    }

    pub fn runtime_error(message: impl Into<String>) -> Self {
        Self::Runtime {
            message: message.into(),
        }
    }

    pub fn invalid_field_reference(field: impl Into<String>) -> Self {
        Self::InvalidFieldReference {
            field: field.into(),
        }
    }

    pub fn type_error(operation: impl Into<String>, left_type: impl Into<String>, right_type: impl Into<String>) -> Self {
        Self::TypeError {
            operation: operation.into(),
            left_type: left_type.into(),
            right_type: right_type.into(),
        }
    }

    pub fn undefined_variable(name: impl Into<String>) -> Self {
        Self::UndefinedVariable {
            name: name.into(),
        }
    }

    pub fn undefined_function(name: impl Into<String>) -> Self {
        Self::UndefinedFunction {
            name: name.into(),
        }
    }

    pub fn invalid_function_call(
        function: impl Into<String>,
        args: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self::InvalidFunctionCall {
            function: function.into(),
            args: args.into(),
            reason: reason.into(),
        }
    }

    pub fn array_index_out_of_bounds(index: i64, size: usize) -> Self {
        Self::ArrayIndexOutOfBounds { index, size }
    }

    pub fn invalid_array_index(index: impl Into<String>) -> Self {
        Self::InvalidArrayIndex {
            index: index.into(),
        }
    }

    pub fn invalid_assignment(message: impl Into<String>) -> Self {
        Self::InvalidAssignment {
            message: message.into(),
        }
    }

    pub fn control_flow_error(message: impl Into<String>) -> Self {
        Self::ControlFlow {
            message: message.into(),
        }
    }

    pub fn encoding_error(message: impl Into<String>) -> Self {
        Self::EncodingError {
            message: message.into(),
        }
    }

    pub fn pattern_error(message: impl Into<String>) -> Self {
        Self::PatternError {
            message: message.into(),
        }
    }

    pub fn memory_limit_exceeded(current: usize, limit: usize) -> Self {
        Self::MemoryLimitExceeded { current, limit }
    }

    pub fn syntax_error(message: impl Into<String>) -> Self {
        Self::SyntaxError {
            message: message.into(),
        }
    }

    pub fn read_only_variable(name: impl Into<String>) -> Self {
        Self::ReadOnlyVariable {
            name: name.into(),
        }
    }

    pub fn invalid_format_specifier(format: impl Into<String>) -> Self {
        Self::InvalidFormatSpecifier {
            format: format.into(),
        }
    }
}

// Helper function to convert parsing errors with context
pub fn parse_error_with_context(
    input: &str,
    position: usize,
    message: impl Into<String>,
) -> FastAwkError {
    let lines_before = input[..position].matches('\n').count();
    let line_number = lines_before + 1;
    
    let column = if let Some(last_newline) = input[..position].rfind('\n') {
        position - last_newline
    } else {
        position + 1
    };

    FastAwkError::parse_error(line_number, column, message)
}

impl From<Box<dyn std::error::Error>> for FastAwkError {
    fn from(err: Box<dyn std::error::Error>) -> Self {
        FastAwkError::General(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = FastAwkError::runtime_error("test error");
        assert_eq!(err.to_string(), "Runtime error: test error");

        let err = FastAwkError::undefined_variable("var_name");
        assert_eq!(err.to_string(), "Variable 'var_name' is not defined");

        let err = FastAwkError::type_error("add", "string", "number");
        assert_eq!(err.to_string(), "Type error: cannot add string and number");
    }

    #[test]
    fn test_parse_error_with_context() {
        let input = "line1\nline2\nerror here";
        let position = 12; // Points to "error"
        let err = parse_error_with_context(input, position, "test parse error");
        
        match err {
            FastAwkError::ParseError { line, column, message } => {
                assert_eq!(line, 3);
                assert_eq!(message, "test parse error");
            }
            _ => panic!("Expected ParseError"),
        }
    }
}