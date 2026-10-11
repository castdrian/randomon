fn kmp_table(needle: &[u8]) -> Vec<i32> {
    let mut table = vec![0i32; needle.len().max(2)];
    table[0] = -1;
    table[1] = 0;
    let mut position = 2;
    let mut candidate = 0usize;
    while position < needle.len() {
        if needle[position - 1] == needle[candidate] {
            table[position] = candidate as i32 + 1;
            position += 1;
            candidate += 1;
        } else if candidate > 0 {
            candidate = table[candidate] as usize;
        } else {
            table[position] = 0;
            position += 1;
        }
    }
    table
}

pub fn search(haystack: &[u8], needle: &[u8]) -> Vec<usize> {
    let table = kmp_table(needle);
    let mut results = Vec::new();
    let mut match_start: i64 = 0;
    let mut character: i64 = 0;
    let end = haystack.len() as i64;
    while match_start + character < end {
        if needle[character as usize] == haystack[(character + match_start) as usize] {
            character += 1;
            if character as usize == needle.len() {
                results.push(match_start as usize);
                character = 0;
                match_start += needle.len() as i64;
            }
        } else {
            let fallback = i64::from(table[character as usize]);
            match_start = match_start + character - fallback;
            character = if fallback > -1 { fallback } else { 0 };
        }
    }
    results
}

pub fn find_unique_hex(haystack: &[u8], hex: &str) -> i32 {
    if !hex.len().is_multiple_of(2) {
        return -3;
    }
    let needle: Vec<u8> = (0..hex.len() / 2)
        .map(|index| u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16).unwrap_or(0))
        .collect();
    let found = search(haystack, &needle);
    match found.len() {
        0 => -1,
        1 => found[0] as i32,
        _ => -2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_reports_non_overlapping_matches() {
        let haystack = [1u8, 2, 1, 2, 1, 2, 3];
        assert_eq!(search(&haystack, &[1, 2, 1]), vec![0]);
        assert_eq!(search(&haystack, &[2, 3]), vec![5]);
        assert!(search(&haystack, &[3, 3]).is_empty());
    }

    #[test]
    fn unique_hex_search_reports_missing_and_duplicates() {
        let haystack = [0xAAu8, 0xBB, 0xCC, 0xAA, 0xBB];
        assert_eq!(find_unique_hex(&haystack, "BBCC"), 1);
        assert_eq!(find_unique_hex(&haystack, "AABB"), -2);
        assert_eq!(find_unique_hex(&haystack, "DD"), -1);
        assert_eq!(find_unique_hex(&haystack, "ABC"), -3);
    }
}
