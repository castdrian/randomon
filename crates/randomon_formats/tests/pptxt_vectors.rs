mod common;

use common::{Json, assert_bytes_eq, vector};
use randomon_formats::FormatError;
use randomon_formats::text::pptxt::{read_texts, save_entry};

fn text(json: &Json) -> String {
    let units: Vec<u16> = json
        .bytes()
        .chunks(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    String::from_utf16(&units).expect("vector strings are valid UTF-16")
}

fn texts(json: &Json) -> Vec<String> {
    json.array().iter().map(text).collect()
}

fn check_save(actual: randomon_formats::Result<Vec<u8>>, expected: &Json, context: &str) {
    match (expected.has("error"), actual) {
        (false, Ok(bytes)) => assert_bytes_eq(&bytes, &expected.get("bytes").bytes(), context),
        (true, Err(error)) => {
            let java = expected.get("error").str();
            let matches = match java {
                "java.lang.NumberFormatException" => matches!(error, FormatError::Invalid { .. }),
                _ => matches!(error, FormatError::OutOfBounds { .. }),
            };
            assert!(
                matches,
                "{context}: java threw {java}, rust returned {error:?}"
            );
        }
        (java_failed, actual) => panic!("{context}: java failed {java_failed}, rust {actual:?}"),
    }
}

#[test]
fn pptxt_matches_upstream() {
    let vectors = vector("pptxt");
    for (index, case) in vectors.get("cases").array().iter().enumerate() {
        let file = case.get("file").bytes();
        let decoded = read_texts(&file).expect("decode succeeds");
        assert_eq!(decoded, texts(case.get("decoded")), "decode case {index}");
        check_save(
            save_entry(&file, &decoded),
            case.get("same"),
            &format!("same case {index}"),
        );
        check_save(
            save_entry(&file, &texts(case.get("changed"))),
            case.get("changed_output"),
            &format!("changed case {index}"),
        );
    }
}
