//! The text you edit: colors for Markdown and search matches, and the keys that format.

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::ops::Range;
use std::sync::Arc;

use egui::text::{CCursor, CCursorRange, LayoutJob, TextFormat};
use egui::text_edit::TextEditState;
use egui::{
    Color32, Context, Event, FontFamily, FontId, Galley, Id, InputState, Key, Modifiers, Stroke, Ui,
};
use pulldown_cmark::{CodeBlockKind, Event as Md, Parser, Tag, TagEnd};

use crate::highlight::{self, Span};
use crate::theme::{self, MONO_BOLD, category as cat, family, tint};
use crate::{format, markdown};

/// Line height of the editor, as a factor of the font size.
pub const LINE: f32 = 1.6;

// Colors by index, so a style is two bytes per byte of text.
const TEXT: u8 = 0;
const MARK: u8 = 1;
const HEADING: u8 = 2;
const STRONG: u8 = 3;
const LINK: u8 = 4;
const CODE: u8 = 5;
const QUOTE: u8 = 6;
const KEYWORD: u8 = 7;
const STRING: u8 = 8;
const NUMBER: u8 = 9;
const FUNCTION: u8 = 10;
const TYPE: u8 = 11;
const INVALID: u8 = 12;

fn color(index: u8) -> Color32 {
    match index {
        MARK => tint(cat::VIOLET, 0.7),
        HEADING => tint(theme::LINE, 0.45),
        STRONG => Color32::WHITE,
        LINK => tint(theme::LINE, 0.7),
        CODE => tint(theme::MONEY, 0.55),
        QUOTE => theme::MUTED,
        KEYWORD => tint(cat::PINK_RED, 0.7),
        STRING => tint(cat::LIME, 0.7),
        NUMBER => tint(cat::AMBER, 0.7),
        FUNCTION => tint(cat::AQUA, 0.7),
        TYPE => tint(cat::BLUE, 0.7),
        INVALID => theme::DANGER,
        _ => theme::TEXT,
    }
}

/// Code in the editor takes the neon category colors in place of GitHub's.
fn neon(span: &Span) -> u8 {
    match span.color.to_array() {
        [0xff, 0x7b, 0x72, _] | [0x7e, 0xe7, 0x87, _] => KEYWORD,
        [0xa5, 0xd6, 0xff, _] => STRING,
        [0x79, 0xc0, 0xff, _] => NUMBER,
        [0xd2, 0xa8, 0xff, _] => FUNCTION,
        [0x91, 0x98, 0xa1, _] => QUOTE,
        [0xff, 0xa6, 0x57, _] | [0x1f, 0x6f, 0xeb, _] => TYPE,
        [0xf8, 0x51, 0x49, _] => INVALID,
        _ => TEXT,
    }
}

const BOLD: u8 = 1;
const ITALIC: u8 = 2;
const STRIKE: u8 = 4;
const MATCH: u8 = 8;
const CURRENT: u8 = 16;

#[derive(Clone, Copy, PartialEq, Default)]
struct Fmt {
    color: u8,
    flags: u8,
}

struct Styles(Vec<Fmt>);

impl Styles {
    fn color(&mut self, r: Range<usize>, c: u8) {
        self.0[r].iter_mut().for_each(|f| f.color = c);
    }

    fn flag(&mut self, r: Range<usize>, flag: u8) {
        self.0[r].iter_mut().for_each(|f| f.flags |= flag);
    }
}

/// Markdown colors, applied outside in: a strong text in a quote is white, not grey.
fn markdown(text: &str, styles: &mut Styles, code_cache: &mut HashMap<u64, Arc<Vec<Span>>>) {
    // A code block's text comes in pieces (inside a quote or list each line is its own); this joins them.
    let mut code: Option<(String, String, Vec<(usize, usize)>)> = None;
    // A bare "-" under a list item would make the item a heading while you type; as "+" it stays an item,
    // and the text keeps its length, so positions still match.
    let parsed: String = text
        .split_inclusive('\n')
        .map(|l| {
            if l.trim() == "-" {
                l.replacen('-', "+", 1)
            } else {
                l.to_owned()
            }
        })
        .collect();
    for (event, r) in Parser::new_ext(&parsed, markdown::options()).into_offset_iter() {
        let src = &text[r.clone()];
        match event {
            Md::Start(Tag::Heading { .. }) => {
                styles.color(r.clone(), HEADING);
                styles.flag(r.clone(), BOLD);
                let hashes = src.len() - src.trim_start_matches('#').len();
                styles.color(r.start..r.start + hashes, MARK);
            }
            Md::Start(Tag::Strong) => {
                styles.color(r.clone(), STRONG);
                styles.flag(r.clone(), BOLD);
                mark_ends(styles, &r, src, 2);
            }
            Md::Start(Tag::Emphasis) => {
                styles.flag(r.clone(), ITALIC);
                mark_ends(styles, &r, src, 1);
            }
            Md::Start(Tag::Strikethrough) => {
                styles.flag(r.clone(), STRIKE);
                let n = src.len() - src.trim_start_matches('~').len();
                mark_ends(styles, &r, src, n);
            }
            Md::Start(Tag::Link { .. } | Tag::Image { .. }) | Md::FootnoteReference(_) => {
                styles.color(r, LINK)
            }
            Md::Code(_) => styles.color(r, CODE),
            Md::Html(_) | Md::InlineHtml(_) => styles.color(r, KEYWORD),
            Md::Rule => styles.color(r, MARK),
            Md::Start(Tag::BlockQuote(_)) => {
                styles.color(r.clone(), QUOTE);
                styles.flag(r.clone(), ITALIC);
                let mut at = r.start;
                for line in src.split_inclusive('\n') {
                    let lead = line.len() - line.trim_start().len();
                    let quotes =
                        line[lead..].len() - line[lead..].trim_start_matches(['>', ' ']).len();
                    styles.color(at + lead..at + lead + quotes, MARK);
                    at += line.len();
                }
            }
            Md::Start(Tag::Item) => {
                if let Some(item) = format::parse_item(src.lines().next().unwrap_or("")) {
                    styles.color(
                        r.start + item.indent.len()
                            ..r.start + item.indent.len() + item.marker.len(),
                        MARK,
                    );
                }
            }
            Md::Start(Tag::CodeBlock(kind)) => {
                styles.color(r, MARK);
                let lang = match kind {
                    CodeBlockKind::Fenced(info) => {
                        info.split([' ', ',', '{']).next().unwrap_or("").to_owned()
                    }
                    CodeBlockKind::Indented => String::new(),
                };
                code = Some((lang, String::new(), Vec::new()));
            }
            Md::Text(s) if code.is_some() => {
                let (_, joined, pieces) = code.as_mut().unwrap();
                styles.color(r.clone(), TEXT);
                pieces.push((joined.len(), r.start));
                joined.push_str(&s);
            }
            Md::End(TagEnd::CodeBlock) => {
                let Some((lang, joined, pieces)) = code.take() else {
                    continue;
                };
                let key = {
                    let mut h = DefaultHasher::new();
                    (&lang, &joined).hash(&mut h);
                    h.finish()
                };
                let spans = code_cache
                    .entry(key)
                    .or_insert_with(|| Arc::new(highlight::spans(&lang, &joined, true)))
                    .clone();
                for span in spans.iter() {
                    let c = neon(span);
                    if c == TEXT {
                        continue;
                    }
                    // Map the joined text back to where each piece sits in the document.
                    let mut start = span.range.start;
                    while start < span.range.end {
                        let p = pieces.partition_point(|&(j, _)| j <= start) - 1;
                        let (j, at) = pieces[p];
                        let piece_end = pieces.get(p + 1).map_or(joined.len(), |&(next, _)| next);
                        let end = span.range.end.min(piece_end);
                        styles.color(at + start - j..at + end - j, c);
                        start = end;
                    }
                }
            }
            _ => {}
        }
    }
}

/// Colors `n` marker chars at both ends of an element, if they are marker chars.
fn mark_ends(styles: &mut Styles, r: &Range<usize>, src: &str, n: usize) {
    if n == 0 || src.len() < 2 * n || !src.starts_with(['*', '_', '~']) {
        return;
    }
    styles.color(r.start..r.start + n, MARK);
    if src.ends_with(['*', '_', '~']) {
        styles.color(r.end - n..r.end, MARK);
    }
}

/// The colored text, kept until the text, the size or the matches change.
#[derive(Default)]
pub struct Highlighter {
    key: u64,
    job: LayoutJob,
    code: HashMap<u64, Arc<Vec<Span>>>,
    /// Struck-through text, with its color: egui puts the line too low at this line height, so it is painted here.
    strikes: Vec<(Range<usize>, Color32)>,
}

impl Highlighter {
    pub fn layout(
        &mut self,
        ui: &Ui,
        text: &str,
        wrap: f32,
        size: f32,
        md: bool,
        matches: &[Range<usize>],
        current: Option<usize>,
    ) -> Arc<Galley> {
        let key = {
            let mut h = DefaultHasher::new();
            (text, size.to_bits(), md, matches, current).hash(&mut h);
            h.finish()
        };
        if key != self.key {
            self.key = key;
            self.job = self.build(text, size, md, matches, current);
        }
        let mut job = self.job.clone();
        job.wrap.max_width = wrap;
        ui.painter().layout_job(job)
    }

    fn build(
        &mut self,
        text: &str,
        size: f32,
        md: bool,
        matches: &[Range<usize>],
        current: Option<usize>,
    ) -> LayoutJob {
        let mut styles = Styles(vec![Fmt::default(); text.len()]);
        if md {
            if self.code.len() > 256 {
                self.code.clear();
            }
            markdown(text, &mut styles, &mut self.code);
        }
        for (i, m) in matches.iter().enumerate() {
            styles.flag(m.clone(), if Some(i) == current { CURRENT } else { MATCH });
        }
        let mut job = LayoutJob::default();
        self.strikes.clear();
        let mut start = 0;
        for b in text.char_indices().map(|(b, _)| b).chain([text.len()]) {
            let ends = b == text.len() || styles.0[b] != styles.0[start];
            if ends && b > start {
                let fmt = styles.0[start];
                job.append(&text[start..b], 0.0, text_format(fmt, size));
                if fmt.flags & STRIKE != 0 {
                    self.strikes.push((start..b, color(fmt.color)));
                }
                start = b;
            }
        }
        if text.is_empty() {
            job.append("", 0.0, text_format(Fmt::default(), size));
        }
        job
    }
}

impl Highlighter {
    /// Lines through the middle of the struck-through text, over the galley the editor drew at `origin`.
    pub fn paint_strikes(&self, ui: &Ui, galley: &Galley, origin: egui::Pos2) {
        if self.strikes.is_empty() {
            return;
        }
        let mut chars = galley.job.text.char_indices();
        for row in &galley.rows {
            let mut line: Option<(f32, f32, f32, Color32)> = None;
            let flush = |line: &mut Option<(f32, f32, f32, Color32)>| {
                if let Some((x0, x1, y, c)) = line.take() {
                    ui.painter().hline(x0..=x1, y, Stroke::new(1.0, c));
                }
            };
            for glyph in &row.glyphs {
                let Some((b, _)) = chars.next() else { return };
                let Some((_, c)) = self.strikes.iter().find(|(r, _)| r.contains(&b)) else {
                    flush(&mut line);
                    continue;
                };
                let x0 = origin.x + row.pos.x + glyph.pos.x;
                let x1 = x0 + glyph.advance_width;
                // Halfway up a lowercase letter.
                let y =
                    (origin.y + row.pos.y + glyph.pos.y - glyph.font_height * 0.27).round() + 0.5;
                match &mut line {
                    Some((_, end, _, color)) if *color == *c => *end = x1,
                    _ => {
                        flush(&mut line);
                        line = Some((x0, x1, y, *c));
                    }
                }
            }
            flush(&mut line);
            if row.ends_with_newline {
                chars.next();
            }
        }
    }
}

fn text_format(f: Fmt, size: f32) -> TextFormat {
    let c = color(f.color);
    TextFormat {
        font_id: FontId::new(
            size,
            if f.flags & BOLD != 0 {
                family(MONO_BOLD)
            } else {
                FontFamily::Monospace
            },
        ),
        color: c,
        line_height: Some(size * LINE),
        italics: f.flags & ITALIC != 0,
        background: if f.flags & CURRENT != 0 {
            theme::fade(theme::MONEY, 0.45)
        } else if f.flags & MATCH != 0 {
            theme::fade(theme::MONEY, 0.2)
        } else {
            Color32::TRANSPARENT
        },
        ..TextFormat::default()
    }
}

// â”€â”€ Keys and edits â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Takes a key press with exactly these modifiers out of the input, so the text field doesn't see it.
fn take(i: &mut InputState, key: Key, mods: Modifiers) -> bool {
    let found = i.events.iter().position(|e| {
        matches!(e, Event::Key { key: k, pressed: true, modifiers: m, .. }
            if *k == key && m.shift == mods.shift && m.alt == mods.alt && m.command == mods.command && m.ctrl == mods.ctrl)
    });
    found.map(|p| i.events.remove(p)).is_some()
}

pub fn selection(ctx: &Context, id: Id) -> Option<Range<usize>> {
    let range = TextEditState::load(ctx, id)?.cursor.char_range()?;
    let [a, b] = range.sorted_cursors();
    Some(a.index.into()..b.index.into())
}

pub fn set_selection(ctx: &Context, id: Id, sel: Range<usize>) {
    let mut state = TextEditState::load(ctx, id).unwrap_or_default();
    state.cursor.set_char_range(Some(CCursorRange::two(
        CCursor::new(sel.start),
        CCursor::new(sel.end),
    )));
    state.store(ctx, id);
}

/// Tab, Shift+Tab, Enter in lists and Ctrl+B/I/K, before the text field handles keys.
/// Returns the edit to apply, with the selection after it.
pub fn keys(ui: &Ui, id: Id, text: &str, md: bool, link_text: &str) -> Option<format::Edit> {
    if !ui.memory(|m| m.has_focus(id)) {
        return None;
    }
    let sel = selection(ui.ctx(), id)?;
    let ctrl = Modifiers::COMMAND;
    ui.input_mut(|i| {
        if take(i, Key::Tab, Modifiers::SHIFT) {
            Some(format::outdent(text, sel))
        } else if take(i, Key::Tab, Modifiers::NONE) {
            Some(format::indent(text, sel, md))
        } else if !md {
            None
        } else if take(i, Key::B, ctrl) {
            Some(format::toggle_wrap(text, sel, "**"))
        } else if take(i, Key::I, ctrl) {
            Some(format::toggle_wrap(text, sel, "*"))
        } else if take(i, Key::K, ctrl) {
            Some(format::insert_link(text, sel, link_text))
        } else if sel.is_empty() && i.key_pressed(Key::Enter) {
            let edit = format::continue_markup(text, sel.start)?;
            take(i, Key::Enter, Modifiers::NONE).then_some(edit)
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn colors(text: &str) -> Vec<(String, u8)> {
        let mut styles = Styles(vec![Fmt::default(); text.len()]);
        markdown(text, &mut styles, &mut HashMap::new());
        let mut out: Vec<(String, u8)> = Vec::new();
        for (b, c) in text.char_indices() {
            let col = styles.0[b].color;
            match out.last_mut() {
                Some((s, last)) if *last == col => s.push(c),
                _ => out.push((c.to_string(), col)),
            }
        }
        out.into_iter()
            .filter(|(s, _)| !s.trim().is_empty())
            .collect()
    }

    #[test]
    fn colors_markdown() {
        assert_eq!(
            colors("# Title"),
            [("#".into(), MARK), (" Title".into(), HEADING)]
        );
        assert_eq!(
            colors("a **b** c")[1..4],
            [
                ("**".into(), MARK),
                ("b".into(), STRONG),
                ("**".into(), MARK)
            ]
        );
        assert_eq!(
            colors("- item"),
            [("- ".into(), MARK), ("item".into(), TEXT)]
        );
        assert!(colors("`x`").iter().all(|(_, c)| *c == CODE));
    }

    #[test]
    fn a_new_nested_item_is_no_heading() {
        assert!(colors("- milk\n    - ").iter().all(|(_, c)| *c != HEADING));
    }

    #[test]
    fn colors_code_blocks_inside_quotes() {
        let found = colors("> ```rust\n> fn x() {}\n> ```");
        assert!(
            found.contains(&("fn".into(), KEYWORD)),
            "{:?}",
            found
                .iter()
                .map(|(s, c)| format!("{s}:{c}"))
                .collect::<Vec<_>>()
        );
        assert!(found.contains(&("x".into(), FUNCTION)));
    }
}
