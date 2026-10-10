use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub enum Json {
    Null,
    Bool(bool),
    Number(String),
    Text(Vec<u16>),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

pub fn load(file_name: &str) -> Json {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("vectors")
        .join(file_name);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
    let mut parser = Parser {
        bytes: text.as_bytes(),
        position: 0,
    };
    let value = parser.value();
    parser.skip_whitespace();
    assert_eq!(
        parser.position,
        parser.bytes.len(),
        "trailing data in {file_name}"
    );
    value
}

impl Json {
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Object(entries) => entries
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value),
            _ => None,
        }
    }

    pub fn field(&self, key: &str) -> &Json {
        self.get(key)
            .unwrap_or_else(|| panic!("missing key {key} in {self:?}"))
    }

    pub fn has(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Json::Null)
    }

    pub fn as_bool(&self) -> bool {
        match self {
            Json::Bool(value) => *value,
            other => panic!("expected bool, found {other:?}"),
        }
    }

    pub fn as_i64(&self) -> i64 {
        match self {
            Json::Number(text) => text
                .parse()
                .unwrap_or_else(|_| panic!("not an integer: {text}")),
            other => panic!("expected number, found {other:?}"),
        }
    }

    pub fn as_i32(&self) -> i32 {
        i32::try_from(self.as_i64()).expect("integer fits in i32")
    }

    pub fn as_units(&self) -> &[u16] {
        match self {
            Json::Text(units) => units,
            other => panic!("expected string, found {other:?}"),
        }
    }

    pub fn as_string(&self) -> String {
        String::from_utf16(self.as_units()).expect("vector string is valid UTF-16")
    }

    pub fn as_array(&self) -> &[Json] {
        match self {
            Json::Array(items) => items,
            other => panic!("expected array, found {other:?}"),
        }
    }

    pub fn as_object(&self) -> &[(String, Json)] {
        match self {
            Json::Object(entries) => entries,
            other => panic!("expected object, found {other:?}"),
        }
    }

    pub fn as_unit_array(&self) -> Vec<u16> {
        self.as_array()
            .iter()
            .map(|unit| u16::try_from(unit.as_i64()).expect("UTF-16 unit"))
            .collect()
    }

    pub fn as_strings(&self) -> Vec<String> {
        self.as_array().iter().map(Json::as_string).collect()
    }

    pub fn as_base64(&self) -> Vec<u8> {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD
            .decode(self.as_string())
            .expect("vector base64 is valid")
    }

    pub fn normalized_object(&self) -> HashMap<String, Json> {
        self.as_object()
            .iter()
            .map(|(key, value)| (normalize_key(key), value.clone()))
            .collect()
    }
}

pub fn normalize_key(key: &str) -> String {
    key.chars()
        .filter(|&c| c != '_')
        .flat_map(char::to_lowercase)
        .collect()
}

struct Parser<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl Parser<'_> {
    fn skip_whitespace(&mut self) {
        while self
            .bytes
            .get(self.position)
            .is_some_and(|byte| byte.is_ascii_whitespace())
        {
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

    fn value(&mut self) -> Json {
        match self.peek() {
            b'{' => self.object(),
            b'[' => self.array(),
            b'"' => Json::Text(self.string()),
            b't' => self.literal("true", Json::Bool(true)),
            b'f' => self.literal("false", Json::Bool(false)),
            b'n' => self.literal("null", Json::Null),
            _ => self.number(),
        }
    }

    fn literal(&mut self, word: &str, value: Json) -> Json {
        assert!(self.bytes[self.position..].starts_with(word.as_bytes()));
        self.position += word.len();
        value
    }

    fn number(&mut self) -> Json {
        let start = self.position;
        while self
            .bytes
            .get(self.position)
            .is_some_and(|&byte| byte == b'-' || byte.is_ascii_digit())
        {
            self.position += 1;
        }
        assert!(self.position > start, "invalid JSON value at {start}");
        Json::Number(String::from_utf8(self.bytes[start..self.position].to_vec()).expect("ASCII"))
    }

    fn string(&mut self) -> Vec<u16> {
        self.expect(b'"');
        let mut units = Vec::new();
        loop {
            let byte = self.bytes[self.position];
            self.position += 1;
            match byte {
                b'"' => return units,
                b'\\' => units.push(self.escape()),
                _ => units.push(u16::from(byte)),
            }
        }
    }

    fn escape(&mut self) -> u16 {
        let kind = self.bytes[self.position];
        self.position += 1;
        match kind {
            b'"' => u16::from(b'"'),
            b'\\' => u16::from(b'\\'),
            b'/' => u16::from(b'/'),
            b'n' => u16::from(b'\n'),
            b'r' => u16::from(b'\r'),
            b't' => u16::from(b'\t'),
            b'u' => {
                let hex = std::str::from_utf8(&self.bytes[self.position..self.position + 4])
                    .expect("ASCII escape");
                self.position += 4;
                u16::from_str_radix(hex, 16).expect("hex escape")
            }
            other => panic!("unsupported escape {other}"),
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

    fn object(&mut self) -> Json {
        self.expect(b'{');
        let mut entries = Vec::new();
        if self.peek() == b'}' {
            self.position += 1;
            return Json::Object(entries);
        }
        loop {
            self.skip_whitespace();
            let key = String::from_utf16(&self.string()).expect("valid key");
            self.expect(b':');
            entries.push((key, self.value()));
            match self.peek() {
                b',' => self.position += 1,
                b'}' => {
                    self.position += 1;
                    return Json::Object(entries);
                }
                other => panic!("unexpected {other} in object"),
            }
        }
    }
}
