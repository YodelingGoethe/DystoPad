//! Printing: the document as a web page in the default browser, which opens its print dialog.
//! Markdown looks like GitHub's light theme on paper; plain text is printed as it is.

use std::fmt::Write as _;
use std::hash::{BuildHasher, RandomState};
use std::path::Path;

use pulldown_cmark::{BlockQuoteKind, CodeBlockKind, Event, Parser, Tag, TagEnd};

use crate::{format, highlight, icons, markdown};

const CSS: &str = include_str!("../assets/print/github-markdown-light.css");
const PAGE_CSS: &str = "
body { margin: 0; background: #fff; }
.markdown-body { box-sizing: border-box; max-width: 980px; margin: 0 auto; padding: 32px 45px; }
.markdown-alert-title .octicon { margin-right: 8px; }
.plain { margin: 0; white-space: pre-wrap; overflow-wrap: anywhere; font: 10.5pt Consolas, 'DejaVu Sans Mono', monospace; color: #000; }
@media print { .markdown-body { padding: 0; } }";

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// `file:///C:/My%20Notes/`, for the page's base, so relative pictures load.
fn file_url(dir: &Path) -> String {
    let path = dir.to_string_lossy().replace('\\', "/");
    let mut url = String::from("file://");
    if !path.starts_with('/') {
        url.push('/');
    }
    for b in path.bytes() {
        if b.is_ascii_alphanumeric() || b"/:-._~".contains(&b) {
            url.push(b as char);
        } else {
            let _ = write!(url, "%{b:02X}");
        }
    }
    url + "/"
}

fn code_html(lang: &str, code: &str) -> String {
    let mut html = String::from("<pre><code>");
    let mut end = 0;
    for span in highlight::spans(lang, code, false) {
        html.push_str(&escape(&code[end..span.range.start]));
        let [r, g, b, _] = span.color.to_array();
        let weight = if span.bold { ";font-weight:600" } else { "" };
        let _ = write!(
            html,
            "<span style=\"color:#{r:02x}{g:02x}{b:02x}{weight}\">{}</span>",
            escape(&code[span.range.clone()])
        );
        end = span.range.end;
    }
    html.push_str(&escape(&code[end..]));
    html + "</code></pre>\n"
}

fn markdown_html(text: &str) -> String {
    let source = format::for_preview(text);
    let mut events = Vec::new();
    let mut code: Option<(String, String)> = None;
    for event in Parser::new_ext(&source, markdown::options()) {
        match (event, &mut code) {
            (Event::Start(Tag::CodeBlock(kind)), _) => {
                let lang = match kind {
                    CodeBlockKind::Fenced(info) => {
                        info.split([' ', ',', '{']).next().unwrap_or("").to_owned()
                    }
                    CodeBlockKind::Indented => String::new(),
                };
                code = Some((lang, String::new()));
            }
            (Event::Text(t), Some((_, buf))) => buf.push_str(&t),
            (Event::End(TagEnd::CodeBlock), _) => {
                if let Some((lang, buf)) = code.take() {
                    events.push(Event::Html(code_html(&lang, &buf).into()));
                }
            }
            (event, _) => events.push(event),
        }
    }
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, github_markup(events).into_iter());
    html
}

/// Task lists, alerts and footnotes in GitHub's markup instead of pulldown-cmark's, so github-markdown-css styles them.
fn github_markup(mut events: Vec<Event<'_>>) -> Vec<Event<'_>> {
    let (mut lists, mut items, mut refs) = (Vec::new(), Vec::new(), Vec::<String>::new());
    for i in 0..events.len() {
        let html = match &events[i] {
            Event::Start(Tag::List(_)) => {
                lists.push(i);
                continue;
            }
            Event::End(TagEnd::List(_)) => {
                lists.pop();
                continue;
            }
            Event::Start(Tag::Item) => {
                items.push(i);
                continue;
            }
            Event::End(TagEnd::Item) => {
                items.pop();
                continue;
            }
            Event::TaskListMarker(checked) => {
                let checked = if *checked { " checked" } else { "" };
                if let (Some(&list), Some(&item)) = (lists.last(), items.last()) {
                    events[item] = Event::Html("<li class=\"task-list-item\">".into());
                    if let Event::Start(Tag::List(start)) = events[list] {
                        events[list] = Event::Html(
                            match start {
                                None => "<ul class=\"contains-task-list\">".into(),
                                Some(n) => {
                                    format!("<ol class=\"contains-task-list\" start=\"{n}\">")
                                }
                            }
                            .into(),
                        );
                    }
                }
                format!(
                    "<input type=\"checkbox\" class=\"task-list-item-checkbox\" disabled{checked}> "
                )
            }
            Event::Start(Tag::BlockQuote(Some(kind))) => {
                let (class, title, icon) = match kind {
                    BlockQuoteKind::Note => ("note", "Note", icons::NOTE),
                    BlockQuoteKind::Tip => ("tip", "Tip", icons::TIP),
                    BlockQuoteKind::Important => ("important", "Important", icons::IMPORTANT),
                    BlockQuoteKind::Warning => ("warning", "Warning", icons::WARNING),
                    BlockQuoteKind::Caution => ("caution", "Caution", icons::CAUTION),
                };
                let svg = icon.svg.replacen("<svg ", "<svg class=\"octicon\" ", 1);
                format!(
                    "<div class=\"markdown-alert markdown-alert-{class}\"><p class=\"markdown-alert-title\">{svg}{title}</p>\n"
                )
            }
            Event::End(TagEnd::BlockQuote(Some(_))) => "</div>\n".into(),
            Event::FootnoteReference(label) => {
                let key = label.to_lowercase();
                let n = match refs.iter().position(|r| *r == key) {
                    Some(n) => n + 1,
                    None => {
                        refs.push(key.clone());
                        refs.len()
                    }
                };
                let id = escape(&key);
                format!(
                    "<sup><a href=\"#user-content-fn-{id}\" id=\"user-content-fnref-{id}\" data-footnote-ref>{n}</a></sup>"
                )
            }
            _ => continue,
        };
        events[i] = Event::Html(html.into());
    }

    // The notes go last, numbered by their first reference, as on GitHub.
    let (mut out, mut notes, mut note) = (Vec::new(), Vec::new(), None);
    for event in events {
        match event {
            Event::Start(Tag::FootnoteDefinition(label)) => {
                note = Some((label.to_lowercase(), Vec::new()))
            }
            Event::End(TagEnd::FootnoteDefinition) => notes.extend(note.take()),
            event => match &mut note {
                Some((_, body)) => body.push(event),
                None => out.push(event),
            },
        }
    }
    if refs.is_empty() {
        return out;
    }
    out.push(Event::Html(
        "<section class=\"footnotes\" data-footnotes>\n<ol>\n".into(),
    ));
    for key in refs {
        let Some((_, mut body)) = notes
            .iter()
            .position(|(k, _)| *k == key)
            .map(|i| notes.swap_remove(i))
        else {
            continue;
        };
        let id = escape(&key);
        let back = Event::Html(
            format!(" <a href=\"#user-content-fnref-{id}\" class=\"data-footnote-backref\">↩</a>")
                .into(),
        );
        match body
            .iter()
            .rposition(|e| matches!(e, Event::End(TagEnd::Paragraph)))
        {
            Some(end) => body.insert(end, back),
            None => body.push(back),
        }
        out.push(Event::Html(
            format!("<li id=\"user-content-fn-{id}\">").into(),
        ));
        out.extend(body);
        out.push(Event::Html("</li>\n".into()));
    }
    out.push(Event::Html("</ol>\n</section>\n".into()));
    out
}

/// The page to print. Documents may contain HTML, so the page allows no scripts but its own.
pub fn page(text: &str, md: bool, title: &str, path: Option<&Path>) -> String {
    let nonce = format!("{:x}", RandomState::new().hash_one(title));
    let body = if md {
        markdown_html(text)
    } else {
        format!("<pre class=\"plain\">{}</pre>", escape(text))
    };
    let base = path
        .and_then(Path::parent)
        .map(|d| format!("<base href=\"{}\">", file_url(d)))
        .unwrap_or_default();
    format!(
        "<!doctype html>\n<html><head><meta charset=\"utf-8\">\n\
         <meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; img-src * data: file:; style-src 'unsafe-inline'; script-src 'nonce-{nonce}'\">\n\
         <title>{}</title>{base}<style>{CSS}{PAGE_CSS}</style></head>\n\
         <body><article class=\"markdown-body\">\n{body}</article>\n\
         <script nonce=\"{nonce}\">addEventListener(\"load\", () => setTimeout(print, 300));</script></body></html>\n",
        escape(title)
    )
}

pub fn print(text: &str, md: bool, title: &str, path: Option<&Path>) -> std::io::Result<()> {
    let file = std::env::temp_dir().join("DystoPad print.html");
    std::fs::write(&file, page(text, md, title, path))?;
    open::that_detached(&file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prints_markdown_with_colored_code() {
        let (path, base) = if cfg!(windows) {
            ("C:\\My Notes\\a.md", "file:///C:/My%20Notes/")
        } else {
            ("/My Notes/a.md", "file:///My%20Notes/")
        };
        let html = page(
            "# Hi\n\n```rust\nfn x() {}\n```\n\n- \n",
            true,
            "a <b>",
            Some(Path::new(path)),
        );
        assert!(html.contains("<h1>Hi</h1>"));
        assert!(html.contains("<span style=\"color:#cf222e\">fn</span>"));
        assert!(html.contains("<title>a &lt;b&gt;</title>"));
        assert!(html.contains(&format!("<base href=\"{base}\">")));
    }

    #[test]
    fn prints_task_lists_alerts_and_footnotes_like_github() {
        let html = page(
            "- [x] done\n- [ ] open\n\n> [!TIP]\n> Hint\n\nSee[^n] and[^n].\n\n[^n]: The note.\n\n[^unused]: Gone.\n",
            true,
            "t",
            None,
        );
        assert!(html.contains("<ul class=\"contains-task-list\">"));
        assert!(html.contains(
            "<li class=\"task-list-item\"><input type=\"checkbox\" class=\"task-list-item-checkbox\" disabled checked> done</li>"
        ));
        assert!(html.contains("<div class=\"markdown-alert markdown-alert-tip\"><p class=\"markdown-alert-title\"><svg class=\"octicon\""));
        assert!(html.contains("</svg>Tip</p>"));
        assert!(html.contains("See<sup><a href=\"#user-content-fn-n\" id=\"user-content-fnref-n\" data-footnote-ref>1</a></sup> and<sup>"));
        assert!(html.contains(
            "<li id=\"user-content-fn-n\">\n<p>The note. <a href=\"#user-content-fnref-n\" class=\"data-footnote-backref\">↩</a></p>"
        ));
        assert!(!html.contains("Gone."));
        // The notes come after the text.
        assert!(
            html.find("<section class=\"footnotes\"").unwrap() > html.find("See<sup>").unwrap()
        );
    }

    #[test]
    fn prints_plain_text_escaped() {
        assert!(
            page("<script>", false, "t", None)
                .contains("<pre class=\"plain\">&lt;script&gt;</pre>")
        );
    }
}
