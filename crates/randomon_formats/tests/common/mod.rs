#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Number(f64, String),
    String(String),
    Array(Vec<Json>),
    Object(BTreeMap<String, Json>),
}

impl Json {
    pub fn get(&self, key: &str) -> &Json {
        match self {
            Json::Object(map) => map.get(key).unwrap_or(&Json::Null),
            _ => &Json::Null,
        }
    }

    pub fn has(&self, key: &str) -> bool {
        matches!(self, Json::Object(map) if map.contains_key(key))
    }

    pub fn array(&self) -> &[Json] {
        match self {
            Json::Array(items) => items,
            other => panic!("expected array, found {other:?}"),
        }
    }

    pub fn str(&self) -> &str {
        match self {
            Json::String(text) => text,
            other => panic!("expected string, found {other:?}"),
        }
    }

    pub fn bool(&self) -> bool {
        match self {
            Json::Bool(value) => *value,
            other => panic!("expected bool, found {other:?}"),
        }
    }

    pub fn i64(&self) -> i64 {
        match self {
            Json::Number(_, raw) => raw
                .parse()
                .unwrap_or_else(|_| panic!("not an integer: {raw}")),
            other => panic!("expected number, found {other:?}"),
        }
    }

    pub fn bytes(&self) -> Vec<u8> {
        base64_decode(self.str())
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Json::Null)
    }
}

pub fn vector(name: &str) -> Json {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("vectors")
        .join(format!("{name}.json"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    parse(&text)
}

pub fn upstream_dir() -> PathBuf {
    std::env::var_os("RANDOMON_UPSTREAM")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/upstream"))
        .join("src/com/dabomstew/pkrandom")
}

pub struct Outcome<'a> {
    pub result: &'a str,
    pub value: &'a Json,
    pub error: Option<&'a str>,
}

pub fn outcome(json: &Json) -> Outcome<'_> {
    Outcome {
        result: json.get("result").str(),
        value: json.get("value"),
        error: match json.get("error") {
            Json::String(text) => Some(text.as_str()),
            _ => None,
        },
    }
}

pub fn parse(text: &str) -> Json {
    let mut parser = Parser {
        bytes: text.as_bytes(),
        position: 0,
    };
    let value = parser.value();
    parser.skip_whitespace();
    assert_eq!(parser.position, parser.bytes.len(), "trailing JSON content");
    value
}

struct Parser<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl Parser<'_> {
    fn skip_whitespace(&mut self) {
        while self.position < self.bytes.len() && self.bytes[self.position].is_ascii_whitespace() {
            self.position += 1;
        }
    }

    fn peek(&mut self) -> u8 {
        self.skip_whitespace();
        self.bytes[self.position]
    }

    fn expect(&mut self, byte: u8) {
        assert_eq!(self.peek(), byte, "unexpected JSON at {}", self.position);
        self.position += 1;
    }

    fn literal(&mut self, word: &str, value: Json) -> Json {
        assert!(self.bytes[self.position..].starts_with(word.as_bytes()));
        self.position += word.len();
        value
    }

    fn value(&mut self) -> Json {
        match self.peek() {
            b'{' => self.object(),
            b'[' => self.array(),
            b'"' => Json::String(self.string()),
            b't' => self.literal("true", Json::Bool(true)),
            b'f' => self.literal("false", Json::Bool(false)),
            b'n' => self.literal("null", Json::Null),
            _ => self.number(),
        }
    }

    fn object(&mut self) -> Json {
        self.expect(b'{');
        let mut map = BTreeMap::new();
        if self.peek() == b'}' {
            self.position += 1;
            return Json::Object(map);
        }
        loop {
            let key = self.string();
            self.expect(b':');
            let value = self.value();
            map.insert(key, value);
            match self.peek() {
                b',' => self.position += 1,
                b'}' => {
                    self.position += 1;
                    return Json::Object(map);
                }
                other => panic!("unexpected {other} in object"),
            }
        }
    }

    fn array(&mut self) -> Json {
        self.expect(b'[');
        let mut items = Vec::new();
        if self.peek() == b']' {
            self.position += 1;
            return Json::Array(items);
        }
        loop {
            items.push(self.value());
            match self.peek() {
                b',' => self.position += 1,
                b']' => {
                    self.position += 1;
                    return Json::Array(items);
                }
                other => panic!("unexpected {other} in array"),
            }
        }
    }

    fn string(&mut self) -> String {
        self.expect(b'"');
        let mut units: Vec<u16> = Vec::new();
        loop {
            let byte = self.bytes[self.position];
            self.position += 1;
            match byte {
                b'"' => return String::from_utf16_lossy(&units),
                b'\\' => {
                    let escape = self.bytes[self.position];
                    self.position += 1;
                    match escape {
                        b'n' => units.push(b'\n' as u16),
                        b'r' => units.push(b'\r' as u16),
                        b't' => units.push(b'\t' as u16),
                        b'u' => {
                            let hex =
                                std::str::from_utf8(&self.bytes[self.position..self.position + 4])
                                    .unwrap();
                            units.push(u16::from_str_radix(hex, 16).unwrap());
                            self.position += 4;
                        }
                        other => units.push(other as u16),
                    }
                }
                other => units.push(other as u16),
            }
        }
    }

    fn number(&mut self) -> Json {
        let start = self.position;
        while self.position < self.bytes.len()
            && matches!(
                self.bytes[self.position],
                b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9'
            )
        {
            self.position += 1;
        }
        let raw = std::str::from_utf8(&self.bytes[start..self.position])
            .unwrap()
            .to_owned();
        Json::Number(raw.parse().unwrap(), raw)
    }
}

pub fn base64_decode(text: &str) -> Vec<u8> {
    let value_of = |byte: u8| -> u32 {
        match byte {
            b'A'..=b'Z' => (byte - b'A') as u32,
            b'a'..=b'z' => (byte - b'a' + 26) as u32,
            b'0'..=b'9' => (byte - b'0' + 52) as u32,
            b'+' => 62,
            b'/' => 63,
            other => panic!("invalid base64 byte {other}"),
        }
    };
    let clean: Vec<u8> = text.bytes().filter(|&byte| byte != b'=').collect();
    let mut output = Vec::with_capacity(clean.len() * 3 / 4);
    for chunk in clean.chunks(4) {
        let mut accumulator = 0u32;
        for (index, &byte) in chunk.iter().enumerate() {
            accumulator |= value_of(byte) << (18 - 6 * index);
        }
        let produced = chunk.len() * 6 / 8;
        for index in 0..produced {
            output.push((accumulator >> (16 - 8 * index)) as u8);
        }
    }
    output
}

pub fn hex_sha256(data: &[u8]) -> String {
    randomon_formats::sha256::sha256(data)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub fn first_difference(left: &[u8], right: &[u8]) -> Option<usize> {
    left.iter()
        .zip(right)
        .position(|(a, b)| a != b)
        .or_else(|| (left.len() != right.len()).then(|| left.len().min(right.len())))
}

pub fn assert_bytes_eq(actual: &[u8], expected: &[u8], context: &str) {
    if let Some(offset) = first_difference(actual, expected) {
        panic!(
            "{context}: bytes differ at offset {offset:#x} (actual len {:#x}, expected len {:#x})",
            actual.len(),
            expected.len()
        );
    }
}
