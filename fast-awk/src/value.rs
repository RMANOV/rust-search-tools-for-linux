use crate::errors::{FastAwkError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
    String(String),
    Number(f64),
    Array(HashMap<String, Value>),
    Undefined,
}

impl Value {
    pub fn new_string(s: impl Into<String>) -> Self {
        Value::String(s.into())
    }

    pub fn new_number(n: f64) -> Self {
        Value::Number(n)
    }

    pub fn new_array() -> Self {
        Value::Array(HashMap::new())
    }

    pub fn is_string(&self) -> bool {
        matches!(self, Value::String(_))
    }

    pub fn is_number(&self) -> bool {
        matches!(self, Value::Number(_))
    }

    pub fn is_array(&self) -> bool {
        matches!(self, Value::Array(_))
    }

    pub fn is_undefined(&self) -> bool {
        matches!(self, Value::Undefined)
    }

    /// Convert to string (AWK string conversion rules)
    pub fn to_string(&self) -> String {
        match self {
            Value::String(s) => s.clone(),
            Value::Number(n) => {
                if n.fract() == 0.0 && *n >= i64::MIN as f64 && *n <= i64::MAX as f64 {
                    format!("{}", *n as i64)
                } else {
                    format!("{}", n)
                }
            }
            Value::Array(_) => "[array]".to_string(),
            Value::Undefined => "".to_string(),
        }
    }

    /// Convert to number (AWK numeric conversion rules)
    pub fn to_number(&self) -> f64 {
        match self {
            Value::Number(n) => *n,
            Value::String(s) => {
                // AWK numeric conversion: parse leading numeric part
                let trimmed = s.trim();
                if trimmed.is_empty() {
                    return 0.0;
                }

                // Find the longest prefix that could be a number
                let mut end_pos = 0;
                let mut has_dot = false;
                let mut has_e = false;
                let chars: Vec<char> = trimmed.chars().collect();

                // Handle leading sign
                if !chars.is_empty() && (chars[0] == '+' || chars[0] == '-') {
                    end_pos = 1;
                }

                while end_pos < chars.len() {
                    match chars[end_pos] {
                        '0'..='9' => end_pos += 1,
                        '.' if !has_dot && !has_e => {
                            has_dot = true;
                            end_pos += 1;
                        }
                        'e' | 'E' if !has_e && end_pos > 0 => {
                            has_e = true;
                            end_pos += 1;
                            // Handle sign after e/E
                            if end_pos < chars.len() && (chars[end_pos] == '+' || chars[end_pos] == '-') {
                                end_pos += 1;
                            }
                        }
                        _ => break,
                    }
                }

                if end_pos == 0 || (end_pos == 1 && (chars[0] == '+' || chars[0] == '-')) {
                    0.0
                } else {
                    trimmed[..end_pos].parse().unwrap_or(0.0)
                }
            }
            Value::Array(arr) => arr.len() as f64,
            Value::Undefined => 0.0,
        }
    }

    /// Convert to boolean (AWK truthiness rules)
    pub fn to_bool(&self) -> bool {
        match self {
            Value::String(s) => !s.is_empty(),
            Value::Number(n) => *n != 0.0,
            Value::Array(arr) => !arr.is_empty(),
            Value::Undefined => false,
        }
    }

    /// Get array element (creates array if not exists)
    pub fn get_array_element(&mut self, key: &str) -> &mut Value {
        match self {
            Value::Array(ref mut map) => {
                map.entry(key.to_string()).or_insert(Value::Undefined)
            }
            _ => {
                *self = Value::new_array();
                if let Value::Array(ref mut map) = self {
                    map.entry(key.to_string()).or_insert(Value::Undefined)
                } else {
                    unreachable!()
                }
            }
        }
    }

    /// Set array element
    pub fn set_array_element(&mut self, key: &str, value: Value) -> Result<()> {
        match self {
            Value::Array(ref mut map) => {
                map.insert(key.to_string(), value);
                Ok(())
            }
            _ => {
                *self = Value::new_array();
                if let Value::Array(ref mut map) = self {
                    map.insert(key.to_string(), value);
                    Ok(())
                } else {
                    unreachable!()
                }
            }
        }
    }

    /// Check if array has key
    pub fn has_array_key(&self, key: &str) -> bool {
        match self {
            Value::Array(map) => map.contains_key(key),
            _ => false,
        }
    }

    /// Get array keys
    pub fn array_keys(&self) -> Vec<String> {
        match self {
            Value::Array(map) => map.keys().cloned().collect(),
            _ => Vec::new(),
        }
    }

    /// Get array length
    pub fn array_len(&self) -> usize {
        match self {
            Value::Array(map) => map.len(),
            _ => 0,
        }
    }

    /// AWK string comparison
    pub fn compare_string(&self, other: &Value) -> std::cmp::Ordering {
        self.to_string().cmp(&other.to_string())
    }

    /// AWK numeric comparison
    pub fn compare_numeric(&self, other: &Value) -> std::cmp::Ordering {
        self.to_number().partial_cmp(&other.to_number()).unwrap_or(std::cmp::Ordering::Equal)
    }

    /// AWK comparison (follows AWK rules for string vs numeric comparison)
    pub fn compare(&self, other: &Value) -> std::cmp::Ordering {
        match (self, other) {
            (Value::Number(_), Value::Number(_)) => self.compare_numeric(other),
            (Value::String(s1), Value::String(s2)) => {
                // If both look like numbers, compare numerically
                if self.looks_like_number() && other.looks_like_number() {
                    self.compare_numeric(other)
                } else {
                    s1.cmp(s2)
                }
            }
            _ => {
                // Mixed types: compare as strings unless both look like numbers
                if self.looks_like_number() && other.looks_like_number() {
                    self.compare_numeric(other)
                } else {
                    self.compare_string(other)
                }
            }
        }
    }

    /// Check if a string value looks like a number (for comparison purposes)
    fn looks_like_number(&self) -> bool {
        match self {
            Value::Number(_) => true,
            Value::String(s) => {
                let trimmed = s.trim();
                !trimmed.is_empty() && (
                    trimmed.parse::<f64>().is_ok() ||
                    // Handle hexadecimal
                    (trimmed.starts_with("0x") || trimmed.starts_with("0X")) &&
                    i64::from_str_radix(&trimmed[2..], 16).is_ok()
                )
            }
            _ => false,
        }
    }

    /// Arithmetic addition
    pub fn add(&self, other: &Value) -> Result<Value> {
        Ok(Value::Number(self.to_number() + other.to_number()))
    }

    /// Arithmetic subtraction
    pub fn subtract(&self, other: &Value) -> Result<Value> {
        Ok(Value::Number(self.to_number() - other.to_number()))
    }

    /// Arithmetic multiplication
    pub fn multiply(&self, other: &Value) -> Result<Value> {
        Ok(Value::Number(self.to_number() * other.to_number()))
    }

    /// Arithmetic division
    pub fn divide(&self, other: &Value) -> Result<Value> {
        let divisor = other.to_number();
        if divisor == 0.0 {
            return Err(FastAwkError::DivisionByZero);
        }
        Ok(Value::Number(self.to_number() / divisor))
    }

    /// Arithmetic modulo
    pub fn modulo(&self, other: &Value) -> Result<Value> {
        let divisor = other.to_number();
        if divisor == 0.0 {
            return Err(FastAwkError::DivisionByZero);
        }
        Ok(Value::Number(self.to_number() % divisor))
    }

    /// Arithmetic power
    pub fn power(&self, other: &Value) -> Result<Value> {
        Ok(Value::Number(self.to_number().powf(other.to_number())))
    }

    /// String concatenation
    pub fn concatenate(&self, other: &Value) -> Value {
        Value::String(format!("{}{}", self.to_string(), other.to_string()))
    }

    /// Regular expression match
    pub fn regex_match(&self, pattern: &regex::Regex) -> bool {
        pattern.is_match(&self.to_string())
    }

    /// String contains substring
    pub fn contains(&self, substring: &Value) -> bool {
        self.to_string().contains(&substring.to_string())
    }

    /// Get string length
    pub fn string_len(&self) -> usize {
        self.to_string().len()
    }

    /// Get type name for error messages
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::String(_) => "string",
            Value::Number(_) => "number",
            Value::Array(_) => "array",
            Value::Undefined => "undefined",
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(s)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::String(s.to_string())
    }
}

impl From<f64> for Value {
    fn from(n: f64) -> Self {
        Value::Number(n)
    }
}

impl From<i32> for Value {
    fn from(n: i32) -> Self {
        Value::Number(n as f64)
    }
}

impl From<i64> for Value {
    fn from(n: i64) -> Self {
        Value::Number(n as f64)
    }
}

impl From<usize> for Value {
    fn from(n: usize) -> Self {
        Value::Number(n as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_conversion() {
        let val = Value::String("hello".to_string());
        assert_eq!(val.to_string(), "hello");
        assert_eq!(val.to_number(), 0.0);
        assert!(val.to_bool());

        let val = Value::String("123".to_string());
        assert_eq!(val.to_number(), 123.0);

        let val = Value::String("123.45".to_string());
        assert_eq!(val.to_number(), 123.45);

        let val = Value::String("123abc".to_string());
        assert_eq!(val.to_number(), 123.0);

        let val = Value::String("".to_string());
        assert!(!val.to_bool());
    }

    #[test]
    fn test_number_conversion() {
        let val = Value::Number(42.0);
        assert_eq!(val.to_string(), "42");
        assert_eq!(val.to_number(), 42.0);
        assert!(val.to_bool());

        let val = Value::Number(42.5);
        assert_eq!(val.to_string(), "42.5");

        let val = Value::Number(0.0);
        assert!(!val.to_bool());
    }

    #[test]
    fn test_arithmetic() {
        let a = Value::Number(10.0);
        let b = Value::Number(3.0);

        assert_eq!(a.add(&b).unwrap(), Value::Number(13.0));
        assert_eq!(a.subtract(&b).unwrap(), Value::Number(7.0));
        assert_eq!(a.multiply(&b).unwrap(), Value::Number(30.0));
        assert_eq!(a.divide(&b).unwrap().to_number(), 10.0 / 3.0);
        assert_eq!(a.modulo(&b).unwrap(), Value::Number(1.0));
    }

    #[test]
    fn test_comparison() {
        let a = Value::Number(10.0);
        let b = Value::Number(20.0);
        assert_eq!(a.compare(&b), std::cmp::Ordering::Less);

        let a = Value::String("10".to_string());
        let b = Value::String("20".to_string());
        assert_eq!(a.compare(&b), std::cmp::Ordering::Less);

        let a = Value::String("abc".to_string());
        let b = Value::String("def".to_string());
        assert_eq!(a.compare(&b), std::cmp::Ordering::Less);
    }

    #[test]
    fn test_array_operations() {
        let mut arr = Value::new_array();
        
        arr.set_array_element("key1", Value::String("value1".to_string())).unwrap();
        assert!(arr.has_array_key("key1"));
        
        let element = arr.get_array_element("key1");
        assert_eq!(*element, Value::String("value1".to_string()));
        
        assert_eq!(arr.array_len(), 1);
        assert!(arr.array_keys().contains(&"key1".to_string()));
    }
}