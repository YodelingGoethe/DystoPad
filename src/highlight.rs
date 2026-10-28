//! Code block colors as on GitHub: syntect's grammars, mapped to GitHub's "prettylights" colors.

use std::ops::Range;
use std::str::FromStr;
use std::sync::LazyLock;

use egui::Color32;
use syntect::highlighting::{
    Color, FontStyle, HighlightIterator, HighlightState, Highlighter, ScopeSelectors,
    StyleModifier, Theme, ThemeItem, ThemeSettings,
};
use syntect::parsing::{ParseState, ScopeStack, SyntaxReference, SyntaxSet};

/// (scopes, dark color, light color for printing, bold). Later rules win over earlier ones of the same specificity.
const RULES: &[(&str, u32, u32, bool)] = &[
    (
        "comment, punctuation.definition.comment",
        0x9198a1,
        0x59636e,
        false,
    ),
    (
        "constant, support.constant, support.function.builtin, variable.language, entity.name.constant, meta.property-name, support.type.property-name",
        0x79c0ff,
        0x0550ae,
        false,
    ),
    (
        "entity, entity.name, entity.name.function, entity.name.class, entity.name.type, entity.other.inherited-class, support.function, variable.function, meta.function-call.identifier",
        0xd2a8ff,
        0x6639ba,
        false,
    ),
    ("entity.name.tag", 0x7ee787, 0x0550ae, false),
    (
        "keyword, storage, storage.type, storage.modifier",
        0xff7b72,
        0xcf222e,
        false,
    ),
    (
        "keyword.operator, punctuation.accessor, keyword.operator.assignment",
        0xf0f6fc,
        0x1f2328,
        false,
    ),
    (
        "keyword.operator.word, keyword.operator.logical.python, keyword.operator.new",
        0xff7b72,
        0xcf222e,
        false,
    ),
    (
        "storage.modifier.import, storage.modifier.package",
        0xf0f6fc,
        0x1f2328,
        false,
    ),
    // GitHub's Java grammar has no idea of ALL_CAPS constants.
    ("constant.other.java", 0xf0f6fc, 0x1f2328, false),
    (
        "string, punctuation.definition.string, string.regexp",
        0xa5d6ff,
        0x0a3069,
        false,
    ),
    (
        "constant.character.escape, string.regexp constant.character.escape",
        0x79c0ff,
        0x0550ae,
        false,
    ),
    (
        "variable.other.readwrite.instance, variable.other.readwrite.global, variable.other.readwrite.class, punctuation.definition.variable",
        0xffa657,
        0x953800,
        false,
    ),
    (
        "variable.parameter, variable.other, meta.function-call.arguments",
        0xf0f6fc,
        0x1f2328,
        false,
    ),
    (
        "support.type.property-name.json, entity.name.tag.yaml",
        0x7ee787,
        0x116329,
        false,
    ),
    (
        "markup.heading, markup.heading entity.name",
        0x1f6feb,
        0x0550ae,
        true,
    ),
    (
        "markup.list, punctuation.definition.list_item",
        0xf2cc60,
        0x3b2300,
        false,
    ),
    ("markup.raw, markup.inline.raw", 0x79c0ff, 0x0550ae, false),
    ("markup.quote", 0x7ee787, 0x116329, false),
    (
        "markup.inserted, meta.diff.header.to-file, punctuation.definition.inserted",
        0xaff5b4,
        0x116329,
        false,
    ),
    (
        "markup.deleted, meta.diff.header.from-file, punctuation.definition.deleted",
        0xffdcd7,
        0x82071e,
        false,
    ),
    (
        "markup.changed, punctuation.definition.changed",
        0xffdfb6,
        0x953800,
        false,
    ),
    (
        "meta.diff.range, meta.diff.index, meta.separator",
        0xd2a8ff,
        0x8250df,
        true,
    ),
    (
        "invalid.illegal, invalid.deprecated",
        0xf85149,
        0x82071e,
        false,
    ),
];

pub const DARK_TEXT: u32 = 0xf0f6fc;
pub const LIGHT_TEXT: u32 = 0x1f2328;

struct Hl {
    syntaxes: SyntaxSet,
    dark: Theme,
    light: Theme,
}

fn theme(dark: bool) -> Theme {
    let color = |hex: u32| Color {
        r: (hex >> 16) as u8,
        g: (hex >> 8) as u8,
        b: hex as u8,
        a: 0xff,
    };
    Theme {
        name: None,
        author: None,
        settings: ThemeSettings {
            foreground: Some(color(if dark { DARK_TEXT } else { LIGHT_TEXT })),
            ..ThemeSettings::default()
        },
        scopes: RULES
            .iter()
            .map(|&(scopes, d, l, bold)| ThemeItem {
                scope: ScopeSelectors::from_str(scopes).unwrap(),
                style: StyleModifier {
                    foreground: Some(color(if dark { d } else { l })),
                    background: None,
                    font_style: Some(if bold {
                        FontStyle::BOLD
                    } else {
                        FontStyle::empty()
                    }),
                },
            })
            .collect(),
    }
}

static HL: LazyLock<Hl> = LazyLock::new(|| Hl {
    syntaxes: SyntaxSet::load_defaults_newlines(),
    dark: theme(true),
    light: theme(false),
});

fn syntax(lang: &str) -> Option<&'static SyntaxReference> {
    let lang = lang.trim().to_ascii_lowercase();
    // Names GitHub knows that syntect's grammars spell differently or lack (the closest relative stands in).
    let token = match lang.as_str() {
        "" => return None,
        "ts" | "typescript" | "tsx" | "jsx" | "mjs" | "cjs" | "javascript" | "node" => "js",
        "csharp" | "c#" => "cs",
        "shell" | "sh" | "zsh" | "console" | "shellscript" => "bash",
        "c++" => "cpp",
        "py" | "python3" => "python",
        "golang" => "go",
        "yml" => "yaml",
        "jsonc" | "json5" => "json",
        "md" => "markdown",
        "rs" => "rust",
        "kt" | "kotlin" => "java",
        "toml" | "ini" | "cfg" => "ini",
        other => other,
    };
    let ss = &HL.syntaxes;
    ss.find_syntax_by_token(token)
        .or_else(|| ss.find_syntax_by_extension(token))
}

/// One colored stretch of a code block.
#[derive(Clone, Debug, PartialEq)]
pub struct Span {
    pub range: Range<usize>,
    pub color: Color32,
    pub bold: bool,
}

/// Colors for a code block, or none for a language syntect doesn't know (shown in the plain text color).
pub fn spans(lang: &str, code: &str, dark: bool) -> Vec<Span> {
    let Some(syntax) = syntax(lang) else {
        return Vec::new();
    };
    let hl = &*HL;
    let highlighter = Highlighter::new(if dark { &hl.dark } else { &hl.light });
    let mut parse = ParseState::new(syntax);
    let mut state = HighlightState::new(&highlighter, ScopeStack::new());
    let mut out: Vec<Span> = Vec::new();
    let mut offset = 0;
    for line in code.split_inclusive('\n') {
        // A grammar error leaves the rest of the block uncolored, as GitHub does.
        let Ok(ops) = parse.parse_line(line, &hl.syntaxes) else {
            return out;
        };
        for (style, piece) in HighlightIterator::new(&mut state, &ops, line, &highlighter) {
            let c = style.foreground;
            let span = Span {
                range: offset..offset + piece.len(),
                color: Color32::from_rgb(c.r, c.g, c.b),
                bold: style.font_style.contains(FontStyle::BOLD),
            };
            offset += piece.len();
            match out.last_mut() {
                Some(last)
                    if last.color == span.color
                        && last.bold == span.bold
                        && last.range.end == span.range.start =>
                {
                    last.range.end = span.range.end
                }
                _ => out.push(span),
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The color of the first occurrence of `word`, which must be colored as one.
    fn color_of(lang: &str, code: &str, word: &str) -> u32 {
        let at = code.find(word).unwrap();
        let spans = spans(lang, code, true);
        let span = spans.iter().find(|s| s.range.contains(&at)).unwrap();
        assert!(
            span.range.end >= at + word.len(),
            "{word} is split: {spans:?}"
        );
        let [r, g, b, _] = span.color.to_array();
        u32::from_be_bytes([0, r, g, b])
    }

    #[test]
    fn java_like_github() {
        let code = "public void onInitializeClient() {\n\tCONFIG = OneChestConfig.load();\n\tif(Globals.IsDevBuild) {\n\tCONFIG.enabled = false;\n\tCONFIG.save();}\n}\n";
        assert_eq!(color_of("java", code, "public"), 0xff7b72);
        assert_eq!(color_of("java", code, "void"), 0xff7b72);
        assert_eq!(color_of("java", code, "onInitializeClient"), 0xd2a8ff);
        assert_eq!(color_of("java", code, "load"), 0xd2a8ff);
        assert_eq!(color_of("java", code, "save"), 0xd2a8ff);
        assert_eq!(color_of("java", code, "false"), 0x79c0ff);
        assert_eq!(color_of("java", code, "if"), 0xff7b72);
        for plain in [
            "CONFIG",
            "OneChestConfig",
            "Globals",
            "IsDevBuild",
            "enabled",
        ] {
            assert_eq!(color_of("java", code, plain), DARK_TEXT, "{plain}");
        }
    }

    #[test]
    fn other_languages() {
        let rust = "fn main() {\n    // hi\n    let s = \"x\";\n    s.len();\n}\n";
        assert_eq!(color_of("rust", rust, "fn"), 0xff7b72);
        assert_eq!(color_of("rust", rust, "main"), 0xd2a8ff);
        assert_eq!(color_of("rust", rust, "// hi"), 0x9198a1);
        assert_eq!(color_of("rs", rust, "len"), 0xd2a8ff);
        let js = "const a = foo.bar(1);\n";
        assert_eq!(color_of("ts", js, "const"), 0xff7b72);
        assert_eq!(color_of("ts", js, "bar"), 0xd2a8ff);
        assert_eq!(color_of("ts", js, "1"), 0x79c0ff);
        assert!(spans("nonsense-lang", "x", true).is_empty());
    }
}
