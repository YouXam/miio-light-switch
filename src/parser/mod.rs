use core::fmt;

use pest::Parser;

#[derive(pest_derive::Parser)]
#[grammar = "parser/lang.pest"]
struct LangParser;

#[derive(Debug, Clone)]
pub enum Value {
    Integer(u32),
    Boolean(bool),
    String(String),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Integer(i) => write!(f, "{}", i),
            Value::Boolean(b) => write!(f, "{}", b),
            Value::String(s) => write!(f, "{:?}", s),
        }
    }
}

fn parse_string(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s[1..s.len() - 1].chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(&next_char) = chars.peek() {
                match next_char {
                    'n' => {
                        result.push('\n');
                        chars.next();
                    }
                    't' => {
                        result.push('\t');
                        chars.next();
                    }
                    '\\' => {
                        result.push('\\');
                        chars.next();
                    }
                    '"' => {
                        result.push('"');
                        chars.next();
                    }
                    _ => {
                        result.push(c);
                    }
                }
            }
        } else {
            result.push(c);
        }
    }

    result
}

pub fn parse(input: &str) -> Vec<Value> {
    let pairs = LangParser::parse(Rule::values, input).unwrap();
    let mut values = Vec::new();

    for pair in pairs {
        match pair.as_rule() {
            Rule::boolean => {
                values.push(Value::Boolean(pair.as_str().parse().unwrap()));
            }
            Rule::integer => {
                values.push(Value::Integer(pair.as_str().parse().unwrap()));
            }
            Rule::string => {
                values.push(Value::String(parse_string(pair.as_str())));
            }
            _ => {}
        }
    }

    return values;
}