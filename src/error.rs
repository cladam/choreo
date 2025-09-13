use crate::parser;
use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("I/O Error: {0}")]
    Io(#[from] io::Error),

    #[error("Parsing error: {0}")]
    ParseError(String),

    #[error("Failed to read environment variable: {0}")]
    EnvVarNotFound(String),

    #[error("Failed to read file: {0}")]
    FileReadError(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Unsupported feature: {0}")]
    Unsupported(String),

    #[error("Regexp error: {0}")]
    Regexp(#[from] regex::Error),

    #[error("UTF-8 conversion error: {0}")]
    Utf8(#[from] std::str::Utf8Error),

    #[error("Failed to convert bytes to UTF-8 string: {0}")]
    UTF8Conversion(#[from] std::string::FromUtf8Error),

    #[error("JSON serialization/deserialization error: {0}")]
    SerdeJson(#[from] serde_json::Error),

    #[error("Pest parsing error: {0}")]
    PestParse(Box<pest::error::Error<parser::parser::Rule>>),

    #[error("{count} test(s) failed.")]
    TestsFailed { count: usize, expected: usize },
}
