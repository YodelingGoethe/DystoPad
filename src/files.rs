//! Reading and writing documents: encodings, line endings and paths.

use std::path::{Path, PathBuf};

pub struct Decoded {
    /// With `\n` line breaks only; the file's own line ending is put back on save.
    pub text: String,
    pub crlf: bool,
    pub bom: bool,
}

// shortcut: UTF-16 and ANSI files are read correctly but saved as UTF-8 (UTF-16 keeps a BOM); add encoders if a file must keep its encoding.
pub fn decode(bytes: &[u8], crlf_default: bool) -> Decoded {
    use encoding_rs::{UTF_16BE, UTF_16LE, WINDOWS_1252};
    let (text, bom) = if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        (String::from_utf8_lossy(rest).into_owned(), true)
    } else if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        (
            UTF_16LE.decode_without_bom_handling(rest).0.into_owned(),
            true,
        )
    } else if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        (
            UTF_16BE.decode_without_bom_handling(rest).0.into_owned(),
            true,
        )
    } else {
        match std::str::from_utf8(bytes) {
            Ok(s) => (s.to_owned(), false),
            // An old ANSI file: Windows-1252 keeps its umlauts instead of turning them into replacement characters.
            Err(_) => (
                WINDOWS_1252
                    .decode_without_bom_handling(bytes)
                    .0
                    .into_owned(),
                false,
            ),
        }
    };
    let crlf = match text.find('\n') {
        Some(i) => i > 0 && text.as_bytes()[i - 1] == b'\r',
        None => crlf_default,
    };
    let text = if text.contains('\r') {
        text.replace("\r\n", "\n").replace('\r', "\n")
    } else {
        text
    };
    Decoded { text, crlf, bom }
}

/// The file contents for a document, with its original line endings and BOM.
pub fn encode(text: &str, crlf: bool, bom: bool) -> Vec<u8> {
    let mut out = String::with_capacity(text.len() + 3);
    if bom {
        out.push('\u{FEFF}');
    }
    if crlf {
        out.push_str(&text.replace('\n', "\r\n"));
    } else {
        out.push_str(text);
    }
    out.into_bytes()
}

pub fn read(path: &Path) -> std::io::Result<Decoded> {
    Ok(decode(&std::fs::read(path)?, cfg!(windows)))
}

pub fn is_markdown(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| {
        ["md", "markdown", "mdown", "mkd"]
            .iter()
            .any(|m| e.eq_ignore_ascii_case(m))
    })
}

pub fn same_path(a: &Path, b: &Path) -> bool {
    if cfg!(windows) {
        a.to_string_lossy()
            .eq_ignore_ascii_case(&b.to_string_lossy())
    } else {
        a == b
    }
}

/// A link or image path in a document, relative to the document's folder. URL escapes like `%20` are undone.
pub fn resolve(doc: &Path, rel: &str) -> PathBuf {
    let rel = percent_decode(rel.split(['#', '?']).next().unwrap_or(rel));
    doc.parent().unwrap_or(Path::new("")).join(rel)
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = bytes
            .get(i + 1..i + 3)
            .and_then(|h| u8::from_str_radix(std::str::from_utf8(h).ok()?, 16).ok());
        match (bytes[i], hex) {
            (b'%', Some(b)) => {
                out.push(b);
                i += 3;
            }
            (b, _) => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Turns a tab's name into something every file system accepts.
pub fn safe_file_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if "\\/:*?\"<>|".contains(c) || c.is_control() {
                ' '
            } else {
                c
            }
        })
        .collect();
    let cleaned = cleaned.trim().trim_end_matches('.').trim();
    if cleaned.is_empty() {
        "Untitled".into()
    } else {
        cleaned.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_utf8_and_line_endings() {
        let d = decode(b"a\r\nb", false);
        assert_eq!((d.text.as_str(), d.crlf, d.bom), ("a\nb", true, false));
        let d = decode(b"a\nb", true);
        assert_eq!((d.text.as_str(), d.crlf), ("a\nb", false));
        assert!(decode(b"x", true).crlf);
        assert!(!decode(b"x", false).crlf);
    }

    #[test]
    fn reads_boms_and_utf16() {
        let d = decode(&[0xEF, 0xBB, 0xBF, b'h', b'i'], true);
        assert_eq!((d.text.as_str(), d.bom), ("hi", true));
        assert_eq!(decode(&[0xFF, 0xFE, b'h', 0, b'i', 0], true).text, "hi");
        assert_eq!(decode(&[0xFE, 0xFF, 0, b'h', 0, b'i'], true).text, "hi");
    }

    #[test]
    fn falls_back_to_windows_1252() {
        // "Grüße" in Windows-1252 is not valid UTF-8.
        assert_eq!(decode(&[0x47, 0x72, 0xFC, 0xDF, 0x65], true).text, "Grüße");
    }

    #[test]
    fn round_trips_line_endings_and_bom() {
        let d = decode(b"a\r\nb\r\n", false);
        assert_eq!(encode(&d.text, d.crlf, d.bom), b"a\r\nb\r\n");
        assert_eq!(
            encode("one\ntwo", false, true),
            "\u{FEFF}one\ntwo".as_bytes()
        );
    }

    #[test]
    fn paths() {
        assert!(is_markdown(Path::new("C:/notes/README.MD")));
        assert!(!is_markdown(Path::new("notes.txt")));
        let doc = Path::new("/home/a/docs/readme.md");
        assert_eq!(
            resolve(doc, "img/My%20Map.png"),
            Path::new("/home/a/docs/img/My Map.png")
        );
        assert_eq!(
            resolve(doc, "other.md#intro"),
            Path::new("/home/a/docs/other.md")
        );
        assert_eq!(safe_file_name("What: a/b?"), "What  a b");
        assert_eq!(safe_file_name("  ..."), "Untitled");
    }
}
