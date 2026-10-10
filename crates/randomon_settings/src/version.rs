use crate::java_text::parse_java_int;

pub const VERSION: i32 = 322;

pub const VERSION_STRING: &str = "4.6.1";

pub const OLD_VERSIONS: [(i32, &str); 27] = [
    (100, "1.0.1a"),
    (102, "1.0.2a"),
    (110, "1.1.0"),
    (111, "1.1.1"),
    (112, "1.1.2"),
    (120, "1.2.0a"),
    (150, "1.5.0"),
    (160, "1.6.0a"),
    (161, "1.6.1"),
    (162, "1.6.2"),
    (163, "1.6.3b"),
    (170, "1.7.0b"),
    (171, "1.7.1"),
    (172, "1.7.2"),
    (310, "3.1.0"),
    (311, "4.0.0"),
    (312, "4.0.1"),
    (313, "4.0.2"),
    (314, "4.1.0"),
    (315, "4.2.0"),
    (316, "4.2.1"),
    (317, "4.3.0"),
    (318, "4.4.0"),
    (319, "4.5.0"),
    (320, "4.5.1"),
    (321, "4.6.0"),
    (VERSION, VERSION_STRING),
];

pub fn release_name(version: i32) -> Option<&'static str> {
    OLD_VERSIONS
        .iter()
        .find(|(known, _)| *known == version)
        .map(|(_, name)| *name)
}

pub fn is_release_version_newer(release_version: &str) -> bool {
    if VERSION_STRING.contains("dev") {
        return false;
    }
    let mut chars = release_version.chars();
    if chars.next().is_none() {
        return false;
    }
    let trimmed = chars.as_str();
    let ours: Vec<&str> = VERSION_STRING.split('.').collect();
    let theirs: Vec<&str> = split_java_style(trimmed);
    for (our_piece, their_piece) in ours.iter().zip(theirs.iter()) {
        let (Some(our_number), Some(their_number)) =
            (parse_piece(our_piece), parse_piece(their_piece))
        else {
            return false;
        };
        if our_number != their_number {
            return our_number < their_number;
        }
    }
    false
}

fn parse_piece(piece: &str) -> Option<i32> {
    let units: Vec<u16> = piece.encode_utf16().collect();
    parse_java_int(&units)
}

fn split_java_style(text: &str) -> Vec<&str> {
    if !text.contains('.') {
        return vec![text];
    }
    let mut pieces: Vec<&str> = text.split('.').collect();
    while pieces.last() == Some(&"") {
        pieces.pop();
    }
    pieces
}
