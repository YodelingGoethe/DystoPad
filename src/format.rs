//! Markdown editing: the format bar's commands and the keys that continue and nest lists.
//! Every command takes the text and the selection (in chars) and returns both changed.

use std::borrow::Cow;
use std::ops::Range;

#[derive(Debug, PartialEq)]
pub struct Edit {
    pub text: String,
    /// The new selection, in chars.
    pub sel: Range<usize>,
}

fn byte_of(text: &str, char_idx: usize) -> usize {
    text.char_indices()
        .nth(char_idx)
        .map_or(text.len(), |(b, _)| b)
}

fn char_of(text: &str, byte_idx: usize) -> usize {
    text[..byte_idx].chars().count()
}

fn line_start(text: &str, b: usize) -> usize {
    text[..b].rfind('\n').map_or(0, |i| i + 1)
}

fn line_end(text: &str, b: usize) -> usize {
    text[b..].find('\n').map_or(text.len(), |i| b + i)
}

/// Byte ranges of every line the selection touches.
fn lines(text: &str, sel: &Range<usize>) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut start = line_start(text, sel.start);
    loop {
        let end = line_end(text, start);
        out.push(start..end);
        if end >= sel.end || end == text.len() {
            return out;
        }
        start = end + 1;
    }
}

/// Applies non-overlapping replacements (byte ranges, in order) and moves the selection with them.
/// Text inserted exactly at a selection edge pushes that edge forward.
fn apply(text: &str, edits: &[(Range<usize>, String)], sel: Range<usize>) -> Edit {
    let map = |p: usize| {
        let mut shift = 0isize;
        for (r, ins) in edits {
            if r.end <= p {
                shift += ins.len() as isize - r.len() as isize;
            } else if r.start < p {
                return (r.start as isize + shift) as usize + ins.len().min(p - r.start);
            }
        }
        (p as isize + shift) as usize
    };
    let mut out = String::with_capacity(text.len() + 16);
    let mut last = 0;
    for (r, ins) in edits {
        out.push_str(&text[last..r.start]);
        out.push_str(ins);
        last = r.end;
    }
    out.push_str(&text[last..]);
    let (a, b) = (map(sel.start), map(sel.end));
    Edit {
        sel: char_of(&out, a)..char_of(&out, b),
        text: out,
    }
}

fn bytes(text: &str, sel: &Range<usize>) -> Range<usize> {
    byte_of(text, sel.start)..byte_of(text, sel.end)
}

// ── Inline markers ────────────────────────────────────────────────────

fn run(s: impl Iterator<Item = char>, ch: char) -> usize {
    s.take(3).take_while(|&c| c == ch).count()
}

/// "**x**" is bold, "*x*" italic and "***x***" both, so a run of three counts as wrapped for either.
fn is_wrapped(run: usize, len: usize) -> bool {
    run == len || run == 3
}

/// Bold (`**`), italic (`*`) or strikethrough (`~~`): wraps the selection, or unwraps it if it already is.
pub fn toggle_wrap(text: &str, sel: Range<usize>, marker: &str) -> Edit {
    let ch = marker.chars().next().unwrap();
    let len = marker.len();
    let r = bytes(text, &sel);
    let before = run(text[..r.start].chars().rev(), ch);
    let after = run(text[r.end..].chars(), ch);
    if is_wrapped(before.min(after), len) {
        let edits = [
            (r.start - len..r.start, String::new()),
            (r.end..r.end + len, String::new()),
        ];
        return apply(text, &edits, r);
    }
    let inner = &text[r.clone()];
    if inner.len() > 2 * len
        && is_wrapped(
            run(inner.chars(), ch).min(run(inner.chars().rev(), ch)),
            len,
        )
    {
        let edits = [
            (r.start..r.start + len, String::new()),
            (r.end - len..r.end, String::new()),
        ];
        let e = apply(text, &edits, r.start..r.start);
        let start = e.sel.start;
        return Edit {
            sel: start..start + inner[len..inner.len() - len].chars().count(),
            text: e.text,
        };
    }
    let edits = [
        (r.start..r.start, marker.to_owned()),
        (r.end..r.end, marker.to_owned()),
    ];
    // The selection stays on the wrapped text, between the markers.
    Edit {
        text: apply(text, &edits, r).text,
        sel: sel.start + len..sel.end + len,
    }
}

// ── Line prefixes ─────────────────────────────────────────────────────

fn heading_prefix(line: &str) -> usize {
    let hashes = line.bytes().take_while(|&b| b == b'#').count();
    if !(1..=6).contains(&hashes) {
        return 0;
    }
    let spaces = line[hashes..]
        .bytes()
        .take_while(|&b| b == b' ' || b == b'\t')
        .count();
    if spaces > 0 || hashes == line.len() {
        hashes + spaces
    } else {
        0
    }
}

/// Level 1 to 6 makes the selected lines headings; 0 makes them plain text again.
pub fn set_heading(text: &str, sel: Range<usize>, level: usize) -> Edit {
    let r = bytes(text, &sel);
    let insert = if level > 0 {
        format!("{} ", "#".repeat(level))
    } else {
        String::new()
    };
    let edits: Vec<_> = lines(text, &r)
        .into_iter()
        .map(|l| (l.start..l.start + heading_prefix(&text[l]), insert.clone()))
        .collect();
    apply(text, &edits, r)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ListKind {
    Bullet,
    Numbered,
    Task,
}

/// A list item or quote at the start of a line.
#[derive(Debug, PartialEq)]
pub struct Item<'a> {
    pub indent: &'a str,
    /// The marker with its trailing space, e.g. "- ", "2. ", "- [ ] " or "> ".
    pub marker: &'a str,
    pub kind: Option<ListKind>,
    pub number: Option<u64>,
}

impl Item<'_> {
    fn len(&self) -> usize {
        self.indent.len() + self.marker.len()
    }
}

pub fn parse_item(line: &str) -> Option<Item<'_>> {
    let indent_len = line.len() - line.trim_start_matches([' ', '\t']).len();
    let (indent, rest) = line.split_at(indent_len);
    // A marker must be followed by a space or end the line.
    let spaced = |n: usize| match rest.as_bytes().get(n) {
        None => Some(n),
        Some(b' ') => Some(n + 1),
        _ => None,
    };
    let (len, kind, number) = if rest.starts_with('>') {
        (spaced(1).unwrap_or(1), None, None)
    } else if rest.starts_with(['-', '*', '+']) {
        let len = spaced(1)?;
        let task = rest[len..]
            .get(..3)
            .is_some_and(|b| matches!(b, "[ ]" | "[x]" | "[X]"));
        match task.then(|| rest.as_bytes().get(len + 3)).flatten() {
            None if task => (len + 3, Some(ListKind::Task), None),
            Some(b' ') => (len + 4, Some(ListKind::Task), None),
            _ => (len, Some(ListKind::Bullet), None),
        }
    } else {
        let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
        if !(1..=9).contains(&digits) || !matches!(rest.as_bytes().get(digits), Some(b'.' | b')')) {
            return None;
        }
        (
            spaced(digits + 1)?,
            Some(ListKind::Numbered),
            rest[..digits].parse().ok(),
        )
    };
    Some(Item {
        indent,
        marker: &rest[..len],
        kind,
        number,
    })
}

fn marker_for(kind: ListKind, i: usize) -> String {
    match kind {
        ListKind::Bullet => "- ".into(),
        ListKind::Task => "- [ ] ".into(),
        ListKind::Numbered => format!("{}. ", i + 1),
    }
}

/// Turns the selected lines into a list of this kind, or back into plain lines if they already are one.
pub fn toggle_list(text: &str, sel: Range<usize>, kind: ListKind) -> Edit {
    let r = bytes(text, &sel);
    let all = lines(text, &r);
    // Blank lines inside a selection stay blank.
    let lines: Vec<_> = if all.len() == 1 {
        all
    } else {
        all.into_iter()
            .filter(|l| !text[l.clone()].trim().is_empty())
            .collect()
    };
    let items: Vec<_> = lines
        .iter()
        .map(|l| {
            let line = &text[l.clone()];
            match parse_item(line).filter(|it| it.kind.is_some()) {
                Some(it) => (l.start + it.indent.len(), it.marker.len(), it.kind),
                None => (
                    l.start + line.len() - line.trim_start_matches([' ', '\t']).len(),
                    0,
                    None,
                ),
            }
        })
        .collect();
    let off = items.iter().all(|&(_, _, k)| k == Some(kind));
    let edits: Vec<_> = items
        .iter()
        .enumerate()
        .map(|(i, &(at, len, _))| {
            (
                at..at + len,
                if off {
                    String::new()
                } else {
                    marker_for(kind, i)
                },
            )
        })
        .collect();
    apply(text, &edits, r)
}

/// `[selection](https://)` with the address selected, or `[text](https://)` with the text selected.
pub fn insert_link(text: &str, sel: Range<usize>, placeholder: &str) -> Edit {
    const URL: &str = "https://";
    let r = bytes(text, &sel);
    let label = if r.is_empty() {
        placeholder
    } else {
        &text[r.clone()]
    };
    let e = apply(
        text,
        &[(r.clone(), format!("[{label}]({URL})"))],
        r.start..r.start,
    );
    let label_chars = label.chars().count();
    let start = sel.start + 1;
    let new_sel = if r.is_empty() {
        start..start + label_chars
    } else {
        start + label_chars + 2..start + label_chars + 2 + URL.len()
    };
    Edit {
        text: e.text,
        sel: new_sel,
    }
}

/// A two-column table below the current line, with the first header selected.
pub fn insert_table(text: &str, sel: Range<usize>, header: impl Fn(usize) -> String) -> Edit {
    let b = byte_of(text, sel.end);
    let (ls, le) = (line_start(text, b), line_end(text, b));
    let (h1, h2) = (header(1), header(2));
    let (w1, w2) = (h1.chars().count(), h2.chars().count());
    let table = format!(
        "| {h1} | {h2} |\n| {} | {} |\n| {} | {} |",
        "-".repeat(w1),
        "-".repeat(w2),
        " ".repeat(w1),
        " ".repeat(w2)
    );
    // A table right after a paragraph needs a blank line in between.
    let blank = text[ls..le].trim().is_empty();
    let (at, lead) = if blank {
        (ls..le, "")
    } else {
        (le..le, "\n\n")
    };
    let e = apply(
        text,
        &[(at.clone(), format!("{lead}{table}"))],
        at.start..at.start,
    );
    let start = char_of(&e.text, at.start) + lead.len() + 2;
    Edit {
        text: e.text,
        sel: start..start + w1,
    }
}

// ── Keys ──────────────────────────────────────────────────────────────

/// Enter in a list or quote: continues it. On an empty item it steps out one level, or ends the list.
/// `None` means Enter does what it always does.
pub fn continue_markup(text: &str, cursor: usize) -> Option<Edit> {
    let b = byte_of(text, cursor);
    let (ls, le) = (line_start(text, b), line_end(text, b));
    let line = &text[ls..le];
    let item = parse_item(line)?;
    if b < ls + item.len() {
        return None;
    }
    if line[item.len()..].trim().is_empty() && b == le {
        if item.indent.is_empty() {
            return Some(apply(text, &[(ls..le, String::new())], ls..ls));
        }
        let cut = if item.indent.starts_with('\t') {
            1
        } else {
            item.indent.len().min(4)
        };
        return Some(apply(text, &[(ls..ls + cut, String::new())], b..b));
    }
    let next = match (item.kind, item.number) {
        (Some(ListKind::Numbered), Some(n)) => {
            format!("{}{}", n + 1, &item.marker[n.to_string().len()..])
        }
        (Some(ListKind::Task), _) => format!("{} [ ] ", &item.marker[..1]),
        _ => item.marker.to_owned(),
    };
    Some(apply(
        text,
        &[(b..b, format!("\n{}{next}", item.indent))],
        b..b,
    ))
}

/// Tab. In a Markdown list it nests the selected items; otherwise it types a tab, or indents every selected line.
pub fn indent(text: &str, sel: Range<usize>, markdown: bool) -> Edit {
    let r = bytes(text, &sel);
    let lines = lines(text, &r);
    let in_list = markdown
        && lines
            .iter()
            .all(|l| parse_item(&text[l.clone()]).is_some_and(|it| it.kind.is_some()));
    if in_list {
        let mut edits: Vec<_> = lines
            .iter()
            .map(|l| (l.start..l.start, "    ".to_owned()))
            .collect();
        // A numbered item that starts a nested list counts from 1, or the nested list would start at its old number.
        if let [l] = lines.as_slice() {
            let item = parse_item(&text[l.clone()]).unwrap();
            let prev = (l.start > 0)
                .then(|| &text[line_start(text, l.start - 1)..l.start - 1])
                .and_then(parse_item);
            let starts_list = prev.is_none_or(|p| {
                p.kind != Some(ListKind::Numbered) || p.indent.len() != item.indent.len() + 4
            });
            if let (Some(n), true) = (item.number, starts_list) {
                if n != 1 {
                    let at = l.start + item.indent.len();
                    edits.push((at..at + n.to_string().len(), "1".into()));
                }
            }
        }
        return apply(text, &edits, r);
    }
    if lines.len() > 1 {
        let edits: Vec<_> = lines
            .iter()
            .map(|l| (l.start..l.start, "\t".to_owned()))
            .collect();
        return apply(text, &edits, r);
    }
    let e = apply(text, &[(r.clone(), "\t".into())], r.start..r.start);
    Edit {
        sel: e.sel.end..e.sel.end,
        text: e.text,
    }
}

/// Shift+Tab: removes one level of indentation (a tab or up to four spaces) from every selected line.
pub fn outdent(text: &str, sel: Range<usize>) -> Edit {
    let r = bytes(text, &sel);
    let edits: Vec<_> = lines(text, &r)
        .into_iter()
        .map(|l| {
            let line = &text[l.clone()];
            let cut = if line.starts_with('\t') {
                1
            } else {
                line.bytes().take(4).take_while(|&b| b == b' ').count()
            };
            (l.start..l.start + cut, String::new())
        })
        .collect();
    apply(text, &edits, r)
}

/// The text the preview renders. A list item that is only its marker ("- " right after pressing Enter and Tab)
/// would read as a heading underline for the line above, so it gets an invisible character.
pub fn for_preview(text: &str) -> Cow<'_, str> {
    let bare = |l: &str| matches!(l.trim(), "-" | "*" | "+");
    if !text.lines().any(bare) {
        return Cow::Borrowed(text);
    }
    let fixed: Vec<String> = text
        .lines()
        .map(|l| {
            if bare(l) {
                format!("{} \u{200B}", l.trim_end())
            } else {
                l.to_owned()
            }
        })
        .collect();
    Cow::Owned(fixed.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Shows the result with [ ] around the new selection.
    fn show(e: Edit) -> String {
        let a = byte_of(&e.text, e.sel.start);
        let b = byte_of(&e.text, e.sel.end);
        format!("{}[{}]{}", &e.text[..a], &e.text[a..b], &e.text[b..])
    }

    #[test]
    fn wraps_and_unwraps() {
        assert_eq!(show(toggle_wrap("a word", 2..6, "**")), "a **[word]**");
        assert_eq!(show(toggle_wrap("a **word**", 4..8, "**")), "a [word]");
        assert_eq!(show(toggle_wrap("a **word**", 2..10, "**")), "a [word]");
        assert_eq!(show(toggle_wrap("ab", 1..1, "~~")), "a~~[]~~b");
    }

    #[test]
    fn keeps_bold_and_italic_apart() {
        assert_eq!(show(toggle_wrap("**word**", 2..6, "*")), "***[word]***");
        assert_eq!(show(toggle_wrap("***word***", 3..7, "*")), "**[word]**");
        assert_eq!(show(toggle_wrap("***word***", 3..7, "**")), "*[word]*");
        assert_eq!(show(toggle_wrap("*word*", 1..5, "**")), "***[word]***");
    }

    #[test]
    fn works_with_umlauts() {
        assert_eq!(
            show(toggle_wrap("schön grün", 6..10, "**")),
            "schön **[grün]**"
        );
    }

    #[test]
    fn headings() {
        assert_eq!(show(set_heading("Title", 0..0, 2)), "## []Title");
        assert_eq!(show(set_heading("### Title", 9..9, 1)), "# Title[]");
        assert_eq!(show(set_heading("# Title", 7..7, 0)), "Title[]");
        assert_eq!(show(set_heading("a\nb", 0..3, 3)), "### [a\n### b]");
    }

    #[test]
    fn lists() {
        assert_eq!(
            show(toggle_list("a\nb", 0..3, ListKind::Bullet)),
            "- [a\n- b]"
        );
        assert_eq!(
            show(toggle_list("- a\n- b", 0..7, ListKind::Bullet)),
            "[a\nb]"
        );
        assert_eq!(
            show(toggle_list("a\n\nb", 0..4, ListKind::Numbered)),
            "1. [a\n\n2. b]"
        );
        assert_eq!(
            show(toggle_list("  - a", 5..5, ListKind::Task)),
            "  - [ ] a[]"
        );
        assert_eq!(show(toggle_list("1. a", 4..4, ListKind::Bullet)), "- a[]");
    }

    #[test]
    fn links_and_tables() {
        assert_eq!(
            show(insert_link("see docs", 4..8, "text")),
            "see [docs]([https://])"
        );
        assert_eq!(show(insert_link("", 0..0, "text")), "[[text]](https://)");
        let h = |n: usize| format!("Col {n}");
        assert_eq!(
            show(insert_table("", 0..0, h)),
            "| [Col 1] | Col 2 |\n| ----- | ----- |\n|       |       |"
        );
        assert_eq!(
            show(insert_table("text", 2..2, h)),
            "text\n\n| [Col 1] | Col 2 |\n| ----- | ----- |\n|       |       |"
        );
    }

    #[test]
    fn parses_items() {
        assert_eq!(
            parse_item("  - [x] done").map(|i| (i.indent, i.marker, i.kind)),
            Some(("  ", "- [x] ", Some(ListKind::Task)))
        );
        assert_eq!(
            parse_item("12) x").map(|i| (i.marker, i.number)),
            Some(("12) ", Some(12)))
        );
        assert_eq!(
            parse_item("> quote").map(|i| (i.marker, i.kind)),
            Some(("> ", None))
        );
        assert_eq!(parse_item("-x"), None);
        assert_eq!(parse_item("2024 was"), None);
    }

    #[test]
    fn enter_continues_and_ends_lists() {
        let enter = |t: &str| show(continue_markup(t, t.chars().count()).expect("handled"));
        assert_eq!(enter("- one"), "- one\n- []");
        assert_eq!(enter("9. nine"), "9. nine\n10. []");
        assert_eq!(enter("- [x] done"), "- [x] done\n- [ ] []");
        assert_eq!(enter("> said"), "> said\n> []");
        // An empty nested item steps out one level, an empty top-level item ends the list.
        assert_eq!(enter("- one\n    - "), "- one\n- []");
        assert_eq!(enter("- one\n- "), "- one\n[]");
        assert_eq!(continue_markup("plain", 5), None);
        // Enter inside the marker itself is plain Enter.
        assert_eq!(continue_markup("- one", 1), None);
    }

    #[test]
    fn enter_splits_an_item() {
        assert_eq!(
            show(continue_markup("- onetwo", 5).unwrap()),
            "- one\n- []two"
        );
    }

    #[test]
    fn tab_nests_list_items_and_types_tabs() {
        assert_eq!(show(indent("- a\n- b", 7..7, true)), "- a\n    - b[]");
        assert_eq!(show(indent("1. a\n2. b", 9..9, true)), "1. a\n    1. b[]");
        assert_eq!(
            show(indent("1. a\n    1. b\n3. c", 18..18, true)),
            "1. a\n    1. b\n    3. c[]"
        );
        assert_eq!(show(indent("abc", 0..0, true)), "\t[]abc");
        assert_eq!(show(indent("- a", 3..3, false)), "- a\t[]");
        assert_eq!(show(indent("a\nb", 0..3, false)), "\t[a\n\tb]");
        assert_eq!(show(outdent("\ta\n    b", 0..8)), "[a\nb]");
    }

    #[test]
    fn preview_keeps_bare_markers_as_list_items() {
        assert_eq!(for_preview("- one\n    - "), "- one\n    - \u{200B}");
        assert!(matches!(for_preview("# fine"), Cow::Borrowed(_)));
    }
}
