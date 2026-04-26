use std::collections::HashMap;
use std::io::Cursor;

use anyhow::{Context, Result};

use crate::cli::Format;

pub fn parse(input: &str, format: Format) -> Result<HashMap<String, String>> {
    match format {
        Format::Dotenv => parse_dotenv(input),
        Format::Json => parse_json(input),
    }
}

pub fn parse_dotenv(input: &str) -> Result<HashMap<String, String>> {
    let mut map = HashMap::new();
    let iter = dotenvy::from_read_iter(Cursor::new(input.as_bytes()));
    for item in iter {
        let (k, v) = item.context("failed to parse dotenv")?;
        map.insert(k, v);
    }
    Ok(map)
}

pub fn parse_json(input: &str) -> Result<HashMap<String, String>> {
    serde_json::from_str::<HashMap<String, String>>(input)
        .context("failed to parse json (only flat string values are supported)")
}

#[cfg(test)]
mod tests {
    use super::*;

    // dotenv tests

    #[test]
    fn dotenv_simple() {
        let map = parse_dotenv("KEY=value\n").unwrap();
        assert_eq!(map["KEY"], "value");
    }

    #[test]
    fn dotenv_ignores_comments_and_blank_lines() {
        let input = "# comment\n\nFOO=bar\n";
        let map = parse_dotenv(input).unwrap();
        assert_eq!(map.len(), 1);
        assert_eq!(map["FOO"], "bar");
    }

    #[test]
    fn dotenv_double_quoted_value() {
        let map = parse_dotenv("KEY=\"hello world\"\n").unwrap();
        assert_eq!(map["KEY"], "hello world");
    }

    #[test]
    fn dotenv_single_quoted_value() {
        let map = parse_dotenv("KEY='hello world'\n").unwrap();
        assert_eq!(map["KEY"], "hello world");
    }

    #[test]
    fn dotenv_equals_in_value() {
        let map = parse_dotenv("URL=postgres://u:p@h/db?x=1\n").unwrap();
        assert_eq!(map["URL"], "postgres://u:p@h/db?x=1");
    }

    #[test]
    fn dotenv_last_value_wins_on_duplicate_key() {
        let map = parse_dotenv("K=first\nK=second\n").unwrap();
        assert_eq!(map["K"], "second");
    }

    #[test]
    fn dotenv_invalid_line_errors() {
        assert!(parse_dotenv("NOT_VALID_LINE\n").is_err());
    }

    // json tests

    #[test]
    fn json_flat_object() {
        let map = parse_json(r#"{"A":"1","B":"2"}"#).unwrap();
        assert_eq!(map.len(), 2);
        assert_eq!(map["A"], "1");
        assert_eq!(map["B"], "2");
    }

    #[test]
    fn json_empty_object() {
        let map = parse_json("{}").unwrap();
        assert!(map.is_empty());
    }

    #[test]
    fn json_nested_errors() {
        assert!(parse_json(r#"{"A":{"B":"1"}}"#).is_err());
    }

    #[test]
    fn json_array_value_errors() {
        assert!(parse_json(r#"{"A":["1"]}"#).is_err());
    }

    #[test]
    fn json_number_value_errors() {
        assert!(parse_json(r#"{"A":1}"#).is_err());
    }
}
