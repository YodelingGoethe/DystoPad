//! Markdown as GitHub reads it (tables, task lists, footnotes, alerts, strikethrough, bare links),
//! parsed by pulldown-cmark into a small tree the preview lays out.

use std::collections::HashMap;
use std::ops::Range;

use pulldown_cmark::{
    Alignment, BlockQuoteKind, CodeBlockKind, Event, Options, Parser, Tag, TagEnd,
};

pub fn options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_GFM
}

#[derive(Default, Debug)]
pub struct Doc {
    pub blocks: Vec<Block>,
    /// Link targets, referenced by `Style::link`.
    pub links: Vec<String>,
    /// In the order of their first reference; unreferenced ones are left out, as on GitHub.
    pub footnotes: Vec<Footnote>,
}

#[derive(Debug)]
pub struct Footnote {
    /// The anchor of the definition and of its first reference: `user-content-fn-<label>` and `…fnref-<label>`.
    pub label: String,
    pub blocks: Vec<Block>,
}

#[derive(Debug)]
pub enum Block {
    Para(Inlines),
    /// The text of a tight list item, which has no paragraph around it.
    Plain(Inlines),
    Heading {
        level: u8,
        text: Inlines,
        id: String,
    },
    Code {
        lang: String,
        code: String,
    },
    Quote(Vec<Block>),
    Alert(Alert, Vec<Block>),
    List {
        start: Option<u64>,
        items: Vec<Item>,
    },
    Table {
        aligns: Vec<Alignment>,
        head: Vec<Inlines>,
        rows: Vec<Vec<Inlines>>,
    },
    Rule,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Alert {
    Note,
    Tip,
    Important,
    Warning,
    Caution,
}

#[derive(Debug, Default)]
pub struct Item {
    pub task: Option<bool>,
    pub blocks: Vec<Block>,
}

#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub struct Style {
    pub strong: bool,
    pub em: bool,
    pub strike: bool,
    pub code: bool,
    /// A footnote reference.
    pub sup: bool,
    pub link: Option<u32>,
}

/// Text with styled runs; images break the text into pieces.
#[derive(Default, Debug)]
pub struct Inlines(pub Vec<Piece>);

#[derive(Debug)]
pub enum Piece {
    Text {
        text: String,
        runs: Vec<(Range<usize>, Style)>,
    },
    Image {
        url: String,
        alt: String,
        link: Option<u32>,
    },
}

impl Inlines {
    pub fn plain_text(&self) -> String {
        self.0
            .iter()
            .map(|p| match p {
                Piece::Text { text, .. } => text.as_str(),
                Piece::Image { alt, .. } => alt.as_str(),
            })
            .collect()
    }
}

/// GitHub's heading anchor: lowercase, punctuation dropped, spaces turned into dashes.
pub fn slug(text: &str) -> String {
    text.trim()
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '-' || *c == '_')
        .map(|c| if c.is_whitespace() { '-' } else { c })
        .collect()
}

/// Bare web addresses, as GitHub links them: the byte range and the address to open.
pub fn find_urls(text: &str) -> Vec<(Range<usize>, String)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < text.len() {
        let rest = &text[i..];
        let prefix = ["https://", "http://", "www."].into_iter().find(|p| {
            rest.len() > p.len()
                && rest
                    .get(..p.len())
                    .is_some_and(|s| s.eq_ignore_ascii_case(p))
        });
        let boundary = text[..i]
            .chars()
            .next_back()
            .is_none_or(|c| !c.is_alphanumeric() && !"/.@_-".contains(c));
        let Some(prefix) = prefix.filter(|_| boundary) else {
            i += rest.chars().next().map_or(1, char::len_utf8);
            continue;
        };
        let mut end = rest
            .find(|c: char| c.is_whitespace() || c == '<')
            .unwrap_or(rest.len());
        // Trailing punctuation ends the sentence, not the address; a closing parenthesis only if it has no partner.
        loop {
            let url = &rest[..end];
            match url.chars().next_back() {
                Some('?' | '!' | '.' | ',' | ':' | '*' | '_' | '~' | '\'' | '"') => end -= 1,
                Some(')') if url.matches(')').count() > url.matches('(').count() => end -= 1,
                _ => break,
            }
        }
        if end > prefix.len() && rest[prefix.len()..end].contains(|c: char| c.is_alphanumeric()) {
            let url = &rest[..end];
            let target = if prefix == "www." {
                format!("http://{url}")
            } else {
                url.to_owned()
            };
            out.push((i..i + end, target));
        }
        i += end.max(prefix.len());
    }
    out
}

// ── Building the tree ─────────────────────────────────────────────────

enum Container {
    Root,
    Quote(Option<BlockQuoteKind>),
    List(Option<u64>, Vec<Item>),
    Item(Option<bool>),
    Footnote(String),
    Table {
        aligns: Vec<Alignment>,
        head: Vec<Inlines>,
        rows: Vec<Vec<Inlines>>,
        row: Vec<Inlines>,
    },
}

enum InlineKind {
    Para,
    Plain,
    Heading(u8),
    Cell,
}

struct InlineBuilder {
    kind: InlineKind,
    pieces: Vec<Piece>,
    text: String,
    runs: Vec<(Range<usize>, Style)>,
}

impl InlineBuilder {
    fn new(kind: InlineKind) -> Self {
        Self {
            kind,
            pieces: Vec::new(),
            text: String::new(),
            runs: Vec::new(),
        }
    }

    fn push(&mut self, s: &str, style: Style) {
        if s.is_empty() {
            return;
        }
        let start = self.text.len();
        self.text.push_str(s);
        match self.runs.last_mut() {
            Some((r, st)) if *st == style && r.end == start => r.end = self.text.len(),
            _ => self.runs.push((start..self.text.len(), style)),
        }
    }

    fn break_text(&mut self) {
        if !self.text.is_empty() {
            let text = std::mem::take(&mut self.text);
            self.pieces.push(Piece::Text {
                text,
                runs: std::mem::take(&mut self.runs),
            });
        }
    }

    fn finish(mut self) -> (InlineKind, Inlines) {
        // Line breaks at the very end show nothing.
        while self.text.ends_with('\n') {
            self.text.pop();
            if let Some((r, _)) = self.runs.last_mut() {
                r.end = r.end.min(self.text.len());
                if r.start == r.end {
                    self.runs.pop();
                }
            }
        }
        self.break_text();
        (self.kind, Inlines(self.pieces))
    }
}

struct Builder {
    stack: Vec<(Container, Vec<Block>)>,
    inline: Option<InlineBuilder>,
    style: Style,
    image: Option<(String, String, Option<u32>)>,
    code: Option<(String, String)>,
    html: Option<String>,
    links: Vec<String>,
    footnote_numbers: HashMap<String, usize>,
    footnote_defs: HashMap<String, Vec<Block>>,
    heading_ids: HashMap<String, usize>,
}

impl Builder {
    fn blocks(&mut self) -> &mut Vec<Block> {
        &mut self.stack.last_mut().unwrap().1
    }

    fn link(&mut self, url: String) -> u32 {
        self.links.push(url);
        self.links.len() as u32 - 1
    }

    /// Inline content of a tight list item arrives without a paragraph; it starts one here.
    fn inline(&mut self) -> &mut InlineBuilder {
        self.inline
            .get_or_insert_with(|| InlineBuilder::new(InlineKind::Plain))
    }

    fn flush_inline(&mut self) {
        let Some(builder) = self.inline.take() else {
            return;
        };
        let (kind, text) = builder.finish();
        let block = match kind {
            InlineKind::Para => Block::Para(text),
            InlineKind::Plain => Block::Plain(text),
            InlineKind::Heading(level) => {
                let base = slug(&text.plain_text());
                let n = self.heading_ids.entry(base.clone()).or_insert(0);
                let id = if *n == 0 {
                    base.clone()
                } else {
                    format!("{base}-{n}")
                };
                *n += 1;
                Block::Heading { level, text, id }
            }
            InlineKind::Cell => {
                if let Some((Container::Table { row, .. }, _)) = self.stack.last_mut() {
                    row.push(text);
                }
                return;
            }
        };
        self.blocks().push(block);
    }

    fn text(&mut self, s: &str) {
        if let Some((_, code)) = &mut self.code {
            code.push_str(s);
        } else if let Some(html) = &mut self.html {
            html.push_str(s);
        } else if let Some((_, alt, _)) = &mut self.image {
            alt.push_str(s);
        } else if self.style.link.is_some() || self.style.code {
            let style = self.style;
            self.inline().push(s, style);
        } else {
            let mut last = 0;
            for (range, url) in find_urls(s) {
                let style = self.style;
                self.inline().push(&s[last..range.start], style);
                let link = Some(self.link(url));
                self.inline()
                    .push(&s[range.clone()], Style { link, ..style });
                last = range.end;
            }
            let style = self.style;
            self.inline().push(&s[last..], style);
        }
    }

    fn start(&mut self, tag: Tag) {
        match tag {
            Tag::Paragraph => {
                self.flush_inline();
                self.inline = Some(InlineBuilder::new(InlineKind::Para));
            }
            Tag::Heading { level, .. } => {
                self.flush_inline();
                self.inline = Some(InlineBuilder::new(InlineKind::Heading(level as u8)));
            }
            Tag::TableCell => self.inline = Some(InlineBuilder::new(InlineKind::Cell)),
            Tag::BlockQuote(kind) => self.open(Container::Quote(kind)),
            Tag::List(start) => self.open(Container::List(start, Vec::new())),
            Tag::Item => self.open(Container::Item(None)),
            Tag::FootnoteDefinition(label) => self.open(Container::Footnote(label.to_string())),
            Tag::Table(aligns) => self.open(Container::Table {
                aligns,
                head: Vec::new(),
                rows: Vec::new(),
                row: Vec::new(),
            }),
            Tag::TableHead | Tag::TableRow => {}
            Tag::CodeBlock(kind) => {
                self.flush_inline();
                let lang = match kind {
                    CodeBlockKind::Fenced(info) => {
                        info.split([' ', ',', '{']).next().unwrap_or("").to_owned()
                    }
                    CodeBlockKind::Indented => String::new(),
                };
                self.code = Some((lang, String::new()));
            }
            Tag::HtmlBlock => {
                self.flush_inline();
                self.html = Some(String::new());
            }
            Tag::Emphasis => self.style.em = true,
            Tag::Strong => self.style.strong = true,
            Tag::Strikethrough => self.style.strike = true,
            Tag::Link { dest_url, .. } => self.style.link = Some(self.link(dest_url.to_string())),
            Tag::Image { dest_url, .. } => {
                self.image = Some((dest_url.to_string(), String::new(), self.style.link))
            }
            _ => {}
        }
    }

    fn open(&mut self, container: Container) {
        self.flush_inline();
        self.stack.push((container, Vec::new()));
    }

    fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph | TagEnd::Heading(_) | TagEnd::TableCell => self.flush_inline(),
            TagEnd::Emphasis => self.style.em = false,
            TagEnd::Strong => self.style.strong = false,
            TagEnd::Strikethrough => self.style.strike = false,
            TagEnd::Link => self.style.link = None,
            TagEnd::Image => {
                if let Some((url, alt, link)) = self.image.take() {
                    let inline = self.inline();
                    inline.break_text();
                    inline.pieces.push(Piece::Image { url, alt, link });
                }
            }
            TagEnd::CodeBlock => {
                if let Some((lang, code)) = self.code.take() {
                    self.blocks().push(Block::Code { lang, code });
                }
            }
            TagEnd::HtmlBlock => {
                if let Some(html) = self.html.take() {
                    self.html_block(&html);
                }
            }
            TagEnd::TableHead | TagEnd::TableRow => {
                if let Some((
                    Container::Table {
                        head, rows, row, ..
                    },
                    _,
                )) = self.stack.last_mut()
                {
                    let row = std::mem::take(row);
                    if matches!(tag, TagEnd::TableHead) {
                        *head = row
                    } else {
                        rows.push(row)
                    }
                }
            }
            TagEnd::BlockQuote(_)
            | TagEnd::List(_)
            | TagEnd::Item
            | TagEnd::FootnoteDefinition
            | TagEnd::Table => self.close(),
            _ => {}
        }
    }

    fn close(&mut self) {
        self.flush_inline();
        let (container, blocks) = self.stack.pop().unwrap();
        let block = match container {
            Container::Quote(None) => Block::Quote(blocks),
            Container::Quote(Some(kind)) => Block::Alert(
                match kind {
                    BlockQuoteKind::Note => Alert::Note,
                    BlockQuoteKind::Tip => Alert::Tip,
                    BlockQuoteKind::Important => Alert::Important,
                    BlockQuoteKind::Warning => Alert::Warning,
                    BlockQuoteKind::Caution => Alert::Caution,
                },
                blocks,
            ),
            Container::List(start, items) => Block::List { start, items },
            Container::Item(task) => {
                if let Some((Container::List(_, items), _)) = self.stack.last_mut() {
                    items.push(Item { task, blocks });
                }
                return;
            }
            Container::Footnote(label) => {
                self.footnote_defs.insert(label, blocks);
                return;
            }
            Container::Table {
                aligns, head, rows, ..
            } => Block::Table { aligns, head, rows },
            Container::Root => unreachable!(),
        };
        self.blocks().push(block);
    }

    // shortcut: HTML shows only its text, line breaks and images; render more tags if documents need them.
    fn html_block(&mut self, html: &str) {
        let mut inline = InlineBuilder::new(InlineKind::Para);
        let mut rest = html;
        while let Some(open) = rest.find('<') {
            inline.push(&decode_entities(&rest[..open]), Style::default());
            rest = &rest[open..];
            if let Some(comment) = rest.strip_prefix("<!--") {
                rest = comment.find("-->").map_or("", |e| &comment[e + 3..]);
                continue;
            }
            let end = rest.find('>').map_or(rest.len(), |e| e + 1);
            let tag = &rest[..end];
            let name = tag
                .trim_start_matches(['<', '/'])
                .split(|c: char| !c.is_ascii_alphanumeric())
                .next()
                .unwrap_or("")
                .to_ascii_lowercase();
            match name.as_str() {
                "br" => inline.push("\n", Style::default()),
                "img" => {
                    if let Some(src) = attribute(tag, "src") {
                        inline.break_text();
                        inline.pieces.push(Piece::Image {
                            url: src,
                            alt: attribute(tag, "alt").unwrap_or_default(),
                            link: None,
                        });
                    }
                }
                "p" | "div" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "details" | "summary"
                    if !inline.text.trim().is_empty() =>
                {
                    inline.push("\n", Style::default())
                }
                _ => {}
            }
            rest = &rest[end..];
        }
        inline.push(&decode_entities(rest), Style::default());
        inline.break_text();
        // The indentation and line breaks around tags are layout in HTML, not text.
        let pieces: Vec<_> = inline
            .pieces
            .into_iter()
            .filter_map(|piece| match piece {
                Piece::Text { text, .. } => {
                    let text = text.trim();
                    (!text.is_empty()).then(|| Piece::Text {
                        runs: vec![(0..text.len(), Style::default())],
                        text: text.to_owned(),
                    })
                }
                image => Some(image),
            })
            .collect();
        if !pieces.is_empty() {
            self.flush_inline();
            self.blocks().push(Block::Para(Inlines(pieces)));
        }
    }
}

fn attribute(tag: &str, name: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let at = lower.find(&format!(" {name}="))? + name.len() + 2;
    let value = &tag[at..];
    let value = match value.chars().next()? {
        q @ ('"' | '\'') => value[1..].split(q).next()?,
        _ => value.split([' ', '>', '/']).next()?,
    };
    Some(decode_entities(value))
}

fn decode_entities(s: &str) -> String {
    if !s.contains('&') {
        return s.to_owned();
    }
    s.replace("&nbsp;", "\u{a0}")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

pub fn parse(text: &str) -> Doc {
    let mut b = Builder {
        stack: vec![(Container::Root, Vec::new())],
        inline: None,
        style: Style::default(),
        image: None,
        code: None,
        html: None,
        links: Vec::new(),
        footnote_numbers: HashMap::new(),
        footnote_defs: HashMap::new(),
        heading_ids: HashMap::new(),
    };
    for event in Parser::new_ext(text, options()) {
        match event {
            Event::Start(tag) => b.start(tag),
            Event::End(tag) => b.end(tag),
            Event::Text(s) => b.text(&s),
            Event::Code(s) => {
                let style = Style {
                    code: true,
                    ..b.style
                };
                b.inline().push(&s, style);
            }
            Event::Html(s) => b.text(&s),
            Event::InlineHtml(s) => {
                if s.trim_start_matches('<')
                    .get(..2)
                    .is_some_and(|t| t.eq_ignore_ascii_case("br"))
                {
                    let style = b.style;
                    b.inline().push("\n", style);
                }
            }
            Event::SoftBreak => b.text(" "),
            Event::HardBreak => {
                let style = b.style;
                b.inline().push("\n", style);
            }
            Event::Rule => {
                b.flush_inline();
                b.blocks().push(Block::Rule);
            }
            Event::TaskListMarker(checked) => {
                if let Some((Container::Item(task), _)) = b.stack.last_mut() {
                    *task = Some(checked);
                }
            }
            Event::FootnoteReference(label) => {
                let next = b.footnote_numbers.len() + 1;
                let n = *b.footnote_numbers.entry(label.to_string()).or_insert(next);
                let link = Some(b.link(format!("#user-content-fn-{label}")));
                let style = Style {
                    sup: true,
                    link,
                    ..b.style
                };
                b.inline().push(&n.to_string(), style);
            }
            _ => {}
        }
    }
    b.flush_inline();
    let mut numbered: Vec<_> = b.footnote_numbers.into_iter().collect();
    numbered.sort_by_key(|(_, n)| *n);
    let mut footnotes = Vec::new();
    for (label, _) in numbered {
        if let Some(mut blocks) = b.footnote_defs.remove(&label) {
            // The way back to the reference, at the end of the note's last paragraph.
            let back = b.links.len() as u32;
            b.links.push(format!("#user-content-fnref-{label}"));
            let arrow = Style {
                link: Some(back),
                ..Style::default()
            };
            match blocks.last_mut() {
                Some(Block::Para(Inlines(pieces)) | Block::Plain(Inlines(pieces))) => {
                    match pieces.last_mut() {
                        Some(Piece::Text { text, runs }) => {
                            text.push(' ');
                            let at = text.len();
                            text.push('↩');
                            runs.push((at..text.len(), arrow));
                        }
                        _ => pieces.push(Piece::Text {
                            text: "↩".into(),
                            runs: vec![(0..3, arrow)],
                        }),
                    }
                }
                _ => blocks.push(Block::Para(Inlines(vec![Piece::Text {
                    text: "↩".into(),
                    runs: vec![(0..3, arrow)],
                }]))),
            }
            footnotes.push(Footnote { label, blocks });
        }
    }
    let (_, blocks) = b.stack.pop().unwrap();
    Doc {
        blocks,
        links: b.links,
        footnotes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text_of(block: &Block) -> String {
        match block {
            Block::Para(t) | Block::Plain(t) | Block::Heading { text: t, .. } => t.plain_text(),
            _ => String::new(),
        }
    }

    #[test]
    fn headings_get_github_ids() {
        let doc = parse("# Hello, World!\n## Hello World\n## Hello World");
        let ids: Vec<_> = doc
            .blocks
            .iter()
            .map(|b| match b {
                Block::Heading { id, .. } => id.as_str(),
                _ => "",
            })
            .collect();
        assert_eq!(ids, ["hello-world", "hello-world-1", "hello-world-2"]);
    }

    #[test]
    fn lists_tasks_and_nesting() {
        let doc = parse("- [x] done\n- [ ] open\n  - nested");
        let Block::List { items, start: None } = &doc.blocks[0] else {
            panic!()
        };
        assert_eq!(items[0].task, Some(true));
        assert_eq!(items[1].task, Some(false));
        assert_eq!(text_of(&items[1].blocks[0]), "open");
        assert!(matches!(items[1].blocks[1], Block::List { .. }));
    }

    #[test]
    fn styles_links_and_bare_urls() {
        let doc = parse("**bold** `code` [site](https://a.example) see https://b.example/x).");
        let Block::Para(Inlines(pieces)) = &doc.blocks[0] else {
            panic!()
        };
        let Piece::Text { text, runs } = &pieces[0] else {
            panic!()
        };
        assert_eq!(text, "bold code site see https://b.example/x).");
        assert!(runs[0].1.strong);
        assert!(
            runs.iter()
                .any(|(r, s)| s.code && &text[r.clone()] == "code")
        );
        assert_eq!(doc.links, ["https://a.example", "https://b.example/x"]);
    }

    #[test]
    fn finds_urls_like_github() {
        let found = |s: &str| {
            find_urls(s)
                .into_iter()
                .map(|(r, url)| (s[r].to_owned(), url))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            found("go to www.a.de, now"),
            [("www.a.de".into(), "http://www.a.de".into())]
        );
        assert_eq!(
            found("(see https://en.wikipedia.org/wiki/A_(b))"),
            [(
                "https://en.wikipedia.org/wiki/A_(b)".into(),
                "https://en.wikipedia.org/wiki/A_(b)".into()
            )]
        );
        assert!(found("nohttps://x.de and https:// alone").is_empty());
        // A multi-byte char where a prefix would end.
        assert!(found("Ctrl +/− zoom, ü€ and www").is_empty());
    }

    #[test]
    fn parses_the_docs() {
        for doc in [
            include_str!("../docs/CHANGELOG.md"),
            include_str!("../README.md"),
            include_str!("../docs/Stylingguidelines.md"),
        ] {
            assert!(!parse(doc).blocks.is_empty());
        }
    }

    #[test]
    fn alerts_tables_and_footnotes() {
        let doc = parse(
            "> [!WARNING]\n> Careful\n\n| a | b |\n|---|--:|\n| 1 | 2 |\n\nNote[^n].\n\n[^n]: The note.",
        );
        assert!(
            matches!(&doc.blocks[0], Block::Alert(Alert::Warning, b) if text_of(&b[0]) == "Careful")
        );
        let Block::Table { aligns, head, rows } = &doc.blocks[1] else {
            panic!()
        };
        assert_eq!(aligns[1], Alignment::Right);
        assert_eq!(
            (head[0].plain_text(), rows[0][1].plain_text()),
            ("a".into(), "2".into())
        );
        assert_eq!(text_of(&doc.blocks[2]), "Note1.");
        assert_eq!(doc.footnotes.len(), 1);
        assert_eq!(text_of(&doc.footnotes[0].blocks[0]), "The note. ↩");
    }

    #[test]
    fn html_shows_its_text_and_images() {
        let doc = parse(
            "<p align=\"center\">\n  <img src=\"logo.png\" alt=\"Logo\">\n  Hi &amp; bye<br>there\n</p>\n<!-- hidden -->",
        );
        let Block::Para(Inlines(pieces)) = &doc.blocks[0] else {
            panic!()
        };
        assert!(matches!(&pieces[0], Piece::Image { url, .. } if url == "logo.png"));
        assert!(matches!(&pieces[1], Piece::Text { text, .. } if text == "Hi & bye\nthere"));
        assert_eq!(doc.blocks.len(), 1);
    }
}
