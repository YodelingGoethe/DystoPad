//! Find and replace in the text.

use std::ops::Range;

#[derive(Clone, Default, PartialEq)]
pub struct Query {
    pub needle: String,
    pub match_case: bool,
    pub whole_word: bool,
}

fn fold(c: char, match_case: bool) -> char {
    if match_case {
        return c;
    }
    // Only one-to-one lowercase mappings, so every char keeps its place.
    let mut lower = c.to_lowercase();
    match (lower.next(), lower.next()) {
        (Some(l), None) => l,
        _ => c,
    }
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Byte ranges of every match, without overlaps.
pub fn find_all(text: &str, q: &Query) -> Vec<Range<usize>> {
    let needle: Vec<char> = q.needle.chars().map(|c| fold(c, q.match_case)).collect();
    if needle.is_empty() {
        return Vec::new();
    }
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let byte = |i: usize| chars.get(i).map_or(text.len(), |&(b, _)| b);
    let mut out = Vec::new();
    let mut i = 0;
    while i + needle.len() <= chars.len() {
        let hit = needle
            .iter()
            .enumerate()
            .all(|(k, &n)| fold(chars[i + k].1, q.match_case) == n);
        let whole = !q.whole_word
            || (i.checked_sub(1).is_none_or(|p| !is_word(chars[p].1))
                && chars
                    .get(i + needle.len())
                    .is_none_or(|&(_, c)| !is_word(c)));
        if hit && whole {
            out.push(byte(i)..byte(i + needle.len()));
            i += needle.len();
        } else {
            i += 1;
        }
    }
    out
}

/// The text with every match replaced, and how many there were.
pub fn replace_all(text: &str, q: &Query, with: &str) -> (String, usize) {
    let matches = find_all(text, q);
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    for m in &matches {
        out.push_str(&text[last..m.start]);
        out.push_str(with);
        last = m.end;
    }
    out.push_str(&text[last..]);
    (out, matches.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(needle: &str, match_case: bool, whole_word: bool) -> Query {
        Query {
            needle: needle.into(),
            match_case,
            whole_word,
        }
    }

    #[test]
    fn finds_with_and_without_case() {
        assert_eq!(
            find_all("Straße STRASSE straße", &q("straße", false, false)),
            [0..7, 16..23]
        );
        assert_eq!(find_all("Ab ab AB", &q("ab", true, false)), [3..5]);
        assert_eq!(find_all("aaaa", &q("aa", false, false)), [0..2, 2..4]);
        assert!(find_all("x", &q("", false, false)).is_empty());
    }

    #[test]
    fn whole_words() {
        assert_eq!(
            find_all("cat concat cat_x cat.", &q("cat", false, true)),
            [0..3, 17..20]
        );
    }

    #[test]
    fn replaces() {
        assert_eq!(
            replace_all("a-b-c", &q("-", false, false), "+"),
            ("a+b+c".into(), 2)
        );
    }
}
