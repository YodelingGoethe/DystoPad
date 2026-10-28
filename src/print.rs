//! Printing: the document as a web page in the default browser, which opens its print dialog.
//! Markdown looks like GitHub's light theme on paper; plain text is printed as it is.

use std::fmt::Write as _;
use std::hash::{BuildHasher, RandomState};
use std::path::Path;

use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag, TagEnd};

use crate::{format, highlight, markdown};

const CSS: &str = include_str!("../assets/print/github-markdown-light.css");
const PAGE_CSS: &str = "
body { margin: 0; background: #fff; }
.markdown-body { box-sizing: border-box; max-width: 980px; margin: 0 auto; padding: 32px 45px; }
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
    pulldown_cmark::html::push_html(&mut html, events.into_iter());
    html
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
        let html = page(
            "# Hi\n\n```rust\nfn x() {}\n```\n\n- \n",
            true,
            "a <b>",
            Some(Path::new("C:\\My Notes\\a.md")),
        );
        assert!(html.contains("<h1>Hi</h1>"));
        assert!(html.contains("<span style=\"color:#cf222e\">fn</span>"));
        assert!(html.contains("<title>a &lt;b&gt;</title>"));
        assert!(html.contains("<base href=\"file:///C:/My%20Notes/\">"));
    }

    #[test]
    fn prints_plain_text_escaped() {
        assert!(
            page("<script>", false, "t", None)
                .contains("<pre class=\"plain\">&lt;script&gt;</pre>")
        );
    }
}
