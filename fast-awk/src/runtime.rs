use crate::errors::{FastAwkError, Result};
use crate::value::Value;
use regex::Regex;
use std::collections::HashMap;
use std::io::{self, Write};

#[derive(Debug, Clone)]
pub struct RuntimeContext {
    /// User-defined variables
    pub variables: HashMap<String, Value>,
    /// Built-in variables
    pub built_in_vars: HashMap<String, Value>,
    /// Current record fields
    pub fields: Vec<String>,
    /// Current record number
    pub nr: usize,
    /// Current filename
    pub filename: String,
    /// Field separator
    pub fs: String,
    /// Output field separator
    pub ofs: String,
    /// Record separator
    pub rs: String,
    /// Output record separator
    pub ors: String,
    /// SUBSEP (subscript separator)
    pub subsep: String,
    /// RSTART (start of match for match() function)
    pub rstart: usize,
    /// RLENGTH (length of match for match() function)
    pub rlength: usize,
    /// Exit code
    pub exit_code: Option<i32>,
    /// Control flow state
    pub control_flow: ControlFlow,
    /// Function call stack
    pub call_stack: Vec<CallFrame>,
    /// Compiled regex cache
    pub regex_cache: HashMap<String, Regex>,
}

#[derive(Debug, Clone)]
pub enum ControlFlow {
    None,
    Break,
    Continue,
    Next,
    Exit(i32),
    Return(Value),
}

#[derive(Debug, Clone)]
pub struct CallFrame {
    pub function_name: String,
    pub variables: HashMap<String, Value>,
}

impl RuntimeContext {
    pub fn new() -> Self {
        let mut context = Self {
            variables: HashMap::new(),
            built_in_vars: HashMap::new(),
            fields: Vec::new(),
            nr: 0,
            filename: String::new(),
            fs: " ".to_string(),
            ofs: " ".to_string(),
            rs: "\n".to_string(),
            ors: "\n".to_string(),
            subsep: "\034".to_string(), // ASCII 034 (FS)
            rstart: 0,
            rlength: 0,
            exit_code: None,
            control_flow: ControlFlow::None,
            call_stack: Vec::new(),
            regex_cache: HashMap::new(),
        };
        
        // Initialize built-in variables
        context.update_built_in_vars();
        context
    }

    pub fn initialize_with_args(&mut self, variables: &[(String, String)]) -> Result<()> {
        for (name, value) in variables {
            self.set_variable(name, Value::String(value.clone()));
        }
        Ok(())
    }

    pub fn set_current_record(&mut self, record: &str) {
        self.nr += 1;
        self.parse_fields(record);
        self.update_built_in_vars();
    }

    pub fn set_filename(&mut self, filename: String) {
        self.filename = filename;
        self.update_built_in_vars();
    }

    fn parse_fields(&mut self, record: &str) {
        self.fields.clear();
        self.fields.push(record.to_string()); // $0 is the entire record
        
        if self.fs == " " {
            // Default FS: split on whitespace
            self.fields.extend(
                record.split_whitespace()
                    .map(|s| s.to_string())
            );
        } else if self.fs.len() == 1 {
            // Single character FS
            let fs_char = self.fs.chars().next().unwrap();
            self.fields.extend(
                record.split(fs_char)
                    .map(|s| s.to_string())
            );
        } else {
            // Multi-character FS (treated as regex)
            let fs_clone = self.fs.clone();
            if let Ok(regex) = self.get_regex(&fs_clone) {
                self.fields.extend(
                    regex.split(record)
                        .map(|s| s.to_string())
                );
            } else {
                // Fallback: literal string split
                self.fields.extend(
                    record.split(&self.fs)
                        .map(|s| s.to_string())
                );
            }
        }
    }

    fn update_built_in_vars(&mut self) {
        self.built_in_vars.insert("NR".to_string(), Value::Number(self.nr as f64));
        self.built_in_vars.insert("NF".to_string(), Value::Number((self.fields.len().saturating_sub(1)) as f64));
        self.built_in_vars.insert("FILENAME".to_string(), Value::String(self.filename.clone()));
        self.built_in_vars.insert("FS".to_string(), Value::String(self.fs.clone()));
        self.built_in_vars.insert("OFS".to_string(), Value::String(self.ofs.clone()));
        self.built_in_vars.insert("RS".to_string(), Value::String(self.rs.clone()));
        self.built_in_vars.insert("ORS".to_string(), Value::String(self.ors.clone()));
        self.built_in_vars.insert("SUBSEP".to_string(), Value::String(self.subsep.clone()));
        self.built_in_vars.insert("RSTART".to_string(), Value::Number(self.rstart as f64));
        self.built_in_vars.insert("RLENGTH".to_string(), Value::Number(self.rlength as f64));
    }

    pub fn get_variable(&self, name: &str) -> Value {
        // Check built-in variables first
        if let Some(value) = self.built_in_vars.get(name) {
            return value.clone();
        }
        
        // Check current call frame if in function
        if let Some(frame) = self.call_stack.last() {
            if let Some(value) = frame.variables.get(name) {
                return value.clone();
            }
        }
        
        // Check global variables
        self.variables.get(name).cloned().unwrap_or(Value::Undefined)
    }

    pub fn set_variable(&mut self, name: &str, value: Value) {
        // Handle built-in variables
        match name {
            "FS" => {
                self.fs = value.to_string();
                self.update_built_in_vars();
            }
            "OFS" => {
                self.ofs = value.to_string();
                self.update_built_in_vars();
            }
            "RS" => {
                self.rs = value.to_string();
                self.update_built_in_vars();
            }
            "ORS" => {
                self.ors = value.to_string();
                self.update_built_in_vars();
            }
            "SUBSEP" => {
                self.subsep = value.to_string();
                self.update_built_in_vars();
            }
            "NR" | "NF" | "FILENAME" | "RSTART" | "RLENGTH" => {
                return; // Read-only variables
            }
            _ => {
                // Set in current call frame if in function, otherwise global
                if let Some(frame) = self.call_stack.last_mut() {
                    frame.variables.insert(name.to_string(), value);
                } else {
                    self.variables.insert(name.to_string(), value);
                }
            }
        }
    }

    pub fn get_field(&self, index: usize) -> String {
        if index < self.fields.len() {
            self.fields[index].clone()
        } else {
            String::new()
        }
    }

    pub fn set_field(&mut self, index: usize, value: String) {
        // Extend fields vector if necessary
        while self.fields.len() <= index {
            self.fields.push(String::new());
        }
        
        self.fields[index] = value;
        
        // Rebuild $0 if we're setting a field other than $0
        if index > 0 {
            self.rebuild_record();
        }
        
        self.update_built_in_vars();
    }

    fn rebuild_record(&mut self) {
        if self.fields.len() > 1 {
            self.fields[0] = self.fields[1..].join(&self.ofs);
        }
    }

    pub fn get_regex(&mut self, pattern: &str) -> Result<Regex> {
        if let Some(regex) = self.regex_cache.get(pattern) {
            Ok(regex.clone())
        } else {
            let regex = Regex::new(pattern)?;
            self.regex_cache.insert(pattern.to_string(), regex.clone());
            Ok(regex)
        }
    }

    pub fn push_call_frame(&mut self, function_name: String) {
        self.call_stack.push(CallFrame {
            function_name,
            variables: HashMap::new(),
        });
    }

    pub fn pop_call_frame(&mut self) {
        self.call_stack.pop();
    }

    pub fn set_control_flow(&mut self, flow: ControlFlow) {
        self.control_flow = flow;
    }

    pub fn clear_control_flow(&mut self) {
        self.control_flow = ControlFlow::None;
    }

    pub fn has_control_flow(&self) -> bool {
        !matches!(self.control_flow, ControlFlow::None)
    }

    pub fn set_exit_code(&mut self, code: i32) {
        self.exit_code = Some(code);
        self.control_flow = ControlFlow::Exit(code);
    }

    /// Built-in function: length
    pub fn builtin_length(&self, args: &[Value]) -> Result<Value> {
        let string = if args.is_empty() {
            self.get_field(0)
        } else {
            args[0].to_string()
        };
        Ok(Value::Number(string.len() as f64))
    }

    /// Built-in function: substr
    pub fn builtin_substr(&self, args: &[Value]) -> Result<Value> {
        if args.len() < 2 {
            return Err(FastAwkError::invalid_function_call(
                "substr",
                format!("{} arguments", args.len()),
                "requires at least 2 arguments"
            ));
        }
        
        let string = args[0].to_string();
        let start = args[1].to_number() as usize;
        let length = if args.len() > 2 {
            Some(args[2].to_number() as usize)
        } else {
            None
        };
        
        let start_index = if start > 0 { start - 1 } else { 0 };
        let result = if let Some(len) = length {
            string.chars().skip(start_index).take(len).collect()
        } else {
            string.chars().skip(start_index).collect()
        };
        
        Ok(Value::String(result))
    }

    /// Built-in function: index
    pub fn builtin_index(&self, args: &[Value]) -> Result<Value> {
        if args.len() != 2 {
            return Err(FastAwkError::invalid_function_call(
                "index",
                format!("{} arguments", args.len()),
                "requires exactly 2 arguments"
            ));
        }
        
        let string = args[0].to_string();
        let substring = args[1].to_string();
        
        let position = string.find(&substring)
            .map(|pos| pos + 1) // AWK uses 1-based indexing
            .unwrap_or(0);
        
        Ok(Value::Number(position as f64))
    }

    /// Built-in function: split
    pub fn builtin_split(&mut self, args: &[Value]) -> Result<Value> {
        if args.len() < 2 {
            return Err(FastAwkError::invalid_function_call(
                "split",
                format!("{} arguments", args.len()),
                "requires at least 2 arguments"
            ));
        }
        
        let string = args[0].to_string();
        let array_name = args[1].to_string();
        let separator = if args.len() > 2 {
            args[2].to_string()
        } else {
            self.fs.clone()
        };
        
        let parts: Vec<String> = if separator == " " {
            string.split_whitespace().map(|s| s.to_string()).collect()
        } else if separator.len() == 1 {
            string.split(separator.chars().next().unwrap()).map(|s| s.to_string()).collect()
        } else {
            // Use regex for multi-character separator
            if let Ok(regex) = self.get_regex(&separator) {
                regex.split(&string).map(|s| s.to_string()).collect()
            } else {
                vec![string]
            }
        };
        
        // Create array
        let mut array = Value::new_array();
        for (i, part) in parts.iter().enumerate() {
            array.set_array_element(&(i + 1).to_string(), Value::String(part.clone()))?;
        }
        
        // Set the array variable
        self.set_variable(&array_name, array);
        
        Ok(Value::Number(parts.len() as f64))
    }

    /// Built-in function: gsub
    pub fn builtin_gsub(&mut self, args: &[Value]) -> Result<Value> {
        if args.len() < 2 {
            return Err(FastAwkError::invalid_function_call(
                "gsub",
                format!("{} arguments", args.len()),
                "requires at least 2 arguments"
            ));
        }
        
        let pattern = args[0].to_string();
        let replacement = args[1].to_string();
        let target = if args.len() > 2 {
            args[2].to_string()
        } else {
            self.get_field(0)
        };
        
        let regex = self.get_regex(&pattern)?;
        let result = regex.replace_all(&target, replacement.as_str());
        let count = regex.find_iter(&target).count();
        
        // Update the target (either field or variable)
        if args.len() <= 2 {
            self.set_field(0, result.to_string());
        }
        
        Ok(Value::Number(count as f64))
    }

    /// Built-in function: sub
    pub fn builtin_sub(&mut self, args: &[Value]) -> Result<Value> {
        if args.len() < 2 {
            return Err(FastAwkError::invalid_function_call(
                "sub",
                format!("{} arguments", args.len()),
                "requires at least 2 arguments"
            ));
        }
        
        let pattern = args[0].to_string();
        let replacement = args[1].to_string();
        let target = if args.len() > 2 {
            args[2].to_string()
        } else {
            self.get_field(0)
        };
        
        let regex = self.get_regex(&pattern)?;
        let result = regex.replace(&target, replacement.as_str());
        let count = if result != target { 1 } else { 0 };
        
        // Update the target (either field or variable)
        if args.len() <= 2 {
            self.set_field(0, result.to_string());
        }
        
        Ok(Value::Number(count as f64))
    }

    /// Built-in function: match
    pub fn builtin_match(&mut self, args: &[Value]) -> Result<Value> {
        if args.len() != 2 {
            return Err(FastAwkError::invalid_function_call(
                "match",
                format!("{} arguments", args.len()),
                "requires exactly 2 arguments"
            ));
        }
        
        let string = args[0].to_string();
        let pattern = args[1].to_string();
        
        let regex = self.get_regex(&pattern)?;
        
        if let Some(mat) = regex.find(&string) {
            self.rstart = mat.start() + 1; // AWK uses 1-based indexing
            self.rlength = mat.len();
            self.update_built_in_vars();
            Ok(Value::Number(self.rstart as f64))
        } else {
            self.rstart = 0;
            self.rlength = 0;
            self.update_built_in_vars();
            Ok(Value::Number(0.0))
        }
    }

    /// Built-in function: sprintf
    pub fn builtin_sprintf(&self, args: &[Value]) -> Result<Value> {
        if args.is_empty() {
            return Err(FastAwkError::invalid_function_call(
                "sprintf",
                "0 arguments",
                "requires at least 1 argument"
            ));
        }
        
        let format = args[0].to_string();
        let formatted = self.format_string(&format, &args[1..])?;
        Ok(Value::String(formatted))
    }

    /// Built-in function: toupper
    pub fn builtin_toupper(&self, args: &[Value]) -> Result<Value> {
        let string = if args.is_empty() {
            self.get_field(0)
        } else {
            args[0].to_string()
        };
        Ok(Value::String(string.to_uppercase()))
    }

    /// Built-in function: tolower
    pub fn builtin_tolower(&self, args: &[Value]) -> Result<Value> {
        let string = if args.is_empty() {
            self.get_field(0)
        } else {
            args[0].to_string()
        };
        Ok(Value::String(string.to_lowercase()))
    }

    /// Built-in function: sin
    pub fn builtin_sin(&self, args: &[Value]) -> Result<Value> {
        if args.len() != 1 {
            return Err(FastAwkError::invalid_function_call(
                "sin",
                format!("{} arguments", args.len()),
                "requires exactly 1 argument"
            ));
        }
        Ok(Value::Number(args[0].to_number().sin()))
    }

    /// Built-in function: cos
    pub fn builtin_cos(&self, args: &[Value]) -> Result<Value> {
        if args.len() != 1 {
            return Err(FastAwkError::invalid_function_call(
                "cos",
                format!("{} arguments", args.len()),
                "requires exactly 1 argument"
            ));
        }
        Ok(Value::Number(args[0].to_number().cos()))
    }

    /// Built-in function: atan2
    pub fn builtin_atan2(&self, args: &[Value]) -> Result<Value> {
        if args.len() != 2 {
            return Err(FastAwkError::invalid_function_call(
                "atan2",
                format!("{} arguments", args.len()),
                "requires exactly 2 arguments"
            ));
        }
        Ok(Value::Number(args[0].to_number().atan2(args[1].to_number())))
    }

    /// Built-in function: exp
    pub fn builtin_exp(&self, args: &[Value]) -> Result<Value> {
        if args.len() != 1 {
            return Err(FastAwkError::invalid_function_call(
                "exp",
                format!("{} arguments", args.len()),
                "requires exactly 1 argument"
            ));
        }
        Ok(Value::Number(args[0].to_number().exp()))
    }

    /// Built-in function: log
    pub fn builtin_log(&self, args: &[Value]) -> Result<Value> {
        if args.len() != 1 {
            return Err(FastAwkError::invalid_function_call(
                "log",
                format!("{} arguments", args.len()),
                "requires exactly 1 argument"
            ));
        }
        let n = args[0].to_number();
        if n <= 0.0 {
            return Err(FastAwkError::runtime_error("log of non-positive number"));
        }
        Ok(Value::Number(n.ln()))
    }

    /// Built-in function: sqrt
    pub fn builtin_sqrt(&self, args: &[Value]) -> Result<Value> {
        if args.len() != 1 {
            return Err(FastAwkError::invalid_function_call(
                "sqrt",
                format!("{} arguments", args.len()),
                "requires exactly 1 argument"
            ));
        }
        let n = args[0].to_number();
        if n < 0.0 {
            return Err(FastAwkError::runtime_error("sqrt of negative number"));
        }
        Ok(Value::Number(n.sqrt()))
    }

    /// Built-in function: int
    pub fn builtin_int(&self, args: &[Value]) -> Result<Value> {
        if args.len() != 1 {
            return Err(FastAwkError::invalid_function_call(
                "int",
                format!("{} arguments", args.len()),
                "requires exactly 1 argument"
            ));
        }
        Ok(Value::Number(args[0].to_number().trunc()))
    }

    /// Built-in function: rand
    pub fn builtin_rand(&self, _args: &[Value]) -> Result<Value> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        // Simple pseudo-random number generator
        let mut hasher = DefaultHasher::new();
        std::ptr::addr_of!(self).hash(&mut hasher);
        let hash = hasher.finish();
        
        Ok(Value::Number((hash % 1000000) as f64 / 1000000.0))
    }

    /// Built-in function: srand
    pub fn builtin_srand(&self, _args: &[Value]) -> Result<Value> {
        // In a real implementation, this would seed the random number generator
        Ok(Value::Number(0.0))
    }

    /// Format string for printf-style functions
    fn format_string(&self, format: &str, args: &[Value]) -> Result<String> {
        // Simplified printf formatting
        let mut result = String::new();
        let mut arg_index = 0;
        let mut chars = format.chars().peekable();
        
        while let Some(ch) = chars.next() {
            if ch == '%' {
                if let Some(&next_ch) = chars.peek() {
                    if next_ch == '%' {
                        result.push('%');
                        chars.next();
                        continue;
                    }
                }
                
                // Parse format specifier
                let mut spec = String::new();
                spec.push(ch);
                
                while let Some(&next_ch) = chars.peek() {
                    spec.push(next_ch);
                    chars.next();
                    
                    if "diouxXeEfFgGaAcsp".contains(next_ch) {
                        break;
                    }
                }
                
                if arg_index < args.len() {
                    let formatted = self.format_value(&spec, &args[arg_index])?;
                    result.push_str(&formatted);
                    arg_index += 1;
                } else {
                    result.push_str(&spec);
                }
            } else {
                result.push(ch);
            }
        }
        
        Ok(result)
    }

    fn format_value(&self, spec: &str, value: &Value) -> Result<String> {
        let last_char = spec.chars().last().unwrap_or('s');
        
        match last_char {
            'd' | 'i' => Ok(format!("{:.0}", value.to_number())),
            'o' => Ok(format!("{:o}", value.to_number() as u64)),
            'x' => Ok(format!("{:x}", value.to_number() as u64)),
            'X' => Ok(format!("{:X}", value.to_number() as u64)),
            'f' | 'F' => Ok(format!("{:.6}", value.to_number())),
            'e' => Ok(format!("{:.6e}", value.to_number())),
            'E' => Ok(format!("{:.6E}", value.to_number())),
            'g' => Ok(format!("{:.6}", value.to_number())),
            'G' => Ok(format!("{:.6}", value.to_number())),
            'c' => {
                let n = value.to_number() as u8;
                Ok((n as char).to_string())
            }
            's' => Ok(value.to_string()),
            _ => Err(FastAwkError::invalid_format_specifier(spec.to_string())),
        }
    }

    pub fn print_values(&self, values: &[Value]) -> Result<()> {
        if values.is_empty() {
            println!("{}", self.get_field(0));
        } else {
            let output = values
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(&self.ofs);
            print!("{}{}", output, self.ors);
        }
        io::stdout().flush()?;
        Ok(())
    }

    pub fn printf_format(&self, format: &Value, args: &[Value]) -> Result<()> {
        let formatted = self.format_string(&format.to_string(), args)?;
        print!("{}", formatted);
        io::stdout().flush()?;
        Ok(())
    }
}

impl Default for RuntimeContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_access() {
        let mut ctx = RuntimeContext::new();
        ctx.set_current_record("hello world test");
        
        assert_eq!(ctx.get_field(0), "hello world test");
        assert_eq!(ctx.get_field(1), "hello");
        assert_eq!(ctx.get_field(2), "world");
        assert_eq!(ctx.get_field(3), "test");
    }

    #[test]
    fn test_variable_assignment() {
        let mut ctx = RuntimeContext::new();
        
        ctx.set_variable("test", Value::String("hello".to_string()));
        assert_eq!(ctx.get_variable("test"), Value::String("hello".to_string()));
        
        ctx.set_variable("FS", Value::String(",".to_string()));
        assert_eq!(ctx.fs, ",");
    }

    #[test]
    fn test_builtin_functions() {
        let mut ctx = RuntimeContext::new();
        
        let result = ctx.builtin_length(&[Value::String("hello".to_string())]).unwrap();
        assert_eq!(result, Value::Number(5.0));
        
        let result = ctx.builtin_substr(&[
            Value::String("hello".to_string()),
            Value::Number(2.0),
            Value::Number(3.0)
        ]).unwrap();
        assert_eq!(result, Value::String("ell".to_string()));
        
        let result = ctx.builtin_index(&[
            Value::String("hello world".to_string()),
            Value::String("world".to_string())
        ]).unwrap();
        assert_eq!(result, Value::Number(7.0));
    }

    #[test]
    fn test_field_separator() {
        let mut ctx = RuntimeContext::new();
        ctx.fs = ",".to_string();
        ctx.set_current_record("a,b,c");
        
        assert_eq!(ctx.get_field(1), "a");
        assert_eq!(ctx.get_field(2), "b");
        assert_eq!(ctx.get_field(3), "c");
    }
}