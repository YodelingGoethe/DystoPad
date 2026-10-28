//! Stroke-only 24×24 line icons (paths from Lucide) drawn as neon tubes: a blurred copy under the line.
//! The SVGs are white, so `tint` colors them. The GitHub alert icons are Octicons, filled and without glow.

use egui::{Color32, Image, Rect, Ui};

pub struct Icon {
    uri: &'static str,
    pub svg: &'static str,
}

macro_rules! line_icon {
    ($name:ident, $body:literal) => {
        pub const $name: Icon = Icon {
            uri: concat!("bytes://icon/", stringify!($name), ".svg"),
            // The view box leaves room around the 24×24 icon for the glow.
            svg: concat!(
                r##"<svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="-4 -4 32 32" fill="none" stroke="#fff" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">"##,
                r##"<filter id="g" x="-50%" y="-50%" width="200%" height="200%"><feGaussianBlur stdDeviation="1.3"/></filter>"##,
                r##"<g filter="url(#g)" opacity="0.55">"##, $body, "</g>", $body, "</svg>"
            ),
        };
    };
}

macro_rules! octicon {
    ($name:ident, $d:literal) => {
        pub const $name: Icon = Icon {
            uri: concat!("bytes://octicon/", stringify!($name), ".svg"),
            svg: concat!(r##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" fill="#fff"><path d=""##, $d, r##""/></svg>"##),
        };
    };
}

line_icon!(PLUS, r#"<path d="M5 12h14M12 5v14"/>"#);
line_icon!(X, r#"<path d="M18 6 6 18M6 6l12 12"/>"#);
line_icon!(CHEVRON, r#"<path d="m6 9 6 6 6-6"/>"#);
line_icon!(CHEVRON_RIGHT, r#"<path d="m9 18 6-6-6-6"/>"#);
line_icon!(ARROW_UP, r#"<path d="m5 12 7-7 7 7M12 19V5"/>"#);
line_icon!(ARROW_DOWN, r#"<path d="M12 5v14M19 12l-7 7-7-7"/>"#);
line_icon!(
    GEAR,
    r#"<path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"/><circle cx="12" cy="12" r="3"/>"#
);
line_icon!(HEADING, r#"<path d="M6 12h12M6 20V4M18 20V4"/>"#);
line_icon!(
    LIST,
    r#"<path d="M3 6h.01M3 12h.01M3 18h.01M8 6h13M8 12h13M8 18h13"/>"#
);
line_icon!(
    BOLD,
    r#"<path d="M6 12h9a4 4 0 0 1 0 8H7a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1h7a4 4 0 0 1 0 8"/>"#
);
line_icon!(ITALIC, r#"<path d="M19 4h-9M14 20H5M15 4 9 20"/>"#);
line_icon!(
    STRIKE,
    r#"<path d="M16 4H9a3 3 0 0 0-2.83 4M14 12a4 4 0 0 1 0 8H6M4 12h16"/>"#
);
line_icon!(
    LINK,
    r#"<path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"/><path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"/>"#
);
line_icon!(
    TABLE,
    r#"<rect x="3" y="3" width="18" height="18" rx="2"/><path d="M3 9h18M3 15h18M12 3v18"/>"#
);

octicon!(
    NOTE,
    "M0 8a8 8 0 1 1 16 0A8 8 0 0 1 0 8Zm8-6.5a6.5 6.5 0 1 0 0 13 6.5 6.5 0 0 0 0-13ZM6.5 7.75A.75.75 0 0 1 7.25 7h1a.75.75 0 0 1 .75.75v2.75h.25a.75.75 0 0 1 0 1.5h-2a.75.75 0 0 1 0-1.5h.25v-2h-.25a.75.75 0 0 1-.75-.75ZM8 6a1 1 0 1 1 0-2 1 1 0 0 1 0 2Z"
);
octicon!(
    TIP,
    "M8 1.5c-2.363 0-4 1.69-4 3.75 0 .984.424 1.625.984 2.304l.214.253c.223.264.47.556.673.848.284.411.537.896.621 1.49a.75.75 0 0 1-1.484.211c-.04-.282-.163-.547-.37-.847a8.456 8.456 0 0 0-.542-.68c-.084-.1-.173-.205-.268-.32C3.201 7.75 2.5 6.766 2.5 5.25 2.5 2.31 4.863 0 8 0s5.5 2.31 5.5 5.25c0 1.516-.701 2.5-1.328 3.259-.095.115-.184.22-.268.319-.207.245-.383.453-.541.681-.208.3-.33.565-.37.847a.751.751 0 0 1-1.485-.212c.084-.593.337-1.078.621-1.489.203-.292.45-.584.673-.848.075-.088.147-.173.213-.253.561-.679.985-1.32.985-2.304 0-2.06-1.637-3.75-4-3.75ZM5.75 12h4.5a.75.75 0 0 1 0 1.5h-4.5a.75.75 0 0 1 0-1.5ZM6 15.25a.75.75 0 0 1 .75-.75h2.5a.75.75 0 0 1 0 1.5h-2.5a.75.75 0 0 1-.75-.75Z"
);
octicon!(
    IMPORTANT,
    "M0 1.75C0 .784.784 0 1.75 0h12.5C15.216 0 16 .784 16 1.75v9.5A1.75 1.75 0 0 1 14.25 13H8.06l-2.573 2.573A1.458 1.458 0 0 1 3 14.543V13H1.75A1.75 1.75 0 0 1 0 11.25Zm1.75-.25a.25.25 0 0 0-.25.25v9.5c0 .138.112.25.25.25h2a.75.75 0 0 1 .75.75v2.19l2.72-2.72a.749.749 0 0 1 .53-.22h6.5a.25.25 0 0 0 .25-.25v-9.5a.25.25 0 0 0-.25-.25Zm7 2.25v2.5a.75.75 0 0 1-1.5 0v-2.5a.75.75 0 0 1 1.5 0ZM9 9a1 1 0 1 1-2 0 1 1 0 0 1 2 0Z"
);
octicon!(
    WARNING,
    "M6.457 1.047c.659-1.234 2.427-1.234 3.086 0l6.082 11.378A1.75 1.75 0 0 1 14.082 15H1.918a1.75 1.75 0 0 1-1.543-2.575Zm1.763.707a.25.25 0 0 0-.44 0L1.698 13.132a.25.25 0 0 0 .22.368h12.164a.25.25 0 0 0 .22-.368Zm.53 3.996v2.5a.75.75 0 0 1-1.5 0v-2.5a.75.75 0 0 1 1.5 0ZM9 11a1 1 0 1 1-2 0 1 1 0 0 1 2 0Z"
);
octicon!(
    CAUTION,
    "M4.47.22A.749.749 0 0 1 5 0h6c.199 0 .389.079.53.22l4.25 4.25c.141.14.22.331.22.53v6a.749.749 0 0 1-.22.53l-4.25 4.25A.749.749 0 0 1 11 16H5a.749.749 0 0 1-.53-.22L.22 11.53A.749.749 0 0 1 0 11V5c0-.199.079-.389.22-.53Zm.84 1.28L1.5 5.31v5.38l3.81 3.81h5.38l3.81-3.81V5.31L10.69 1.5ZM8 4a.75.75 0 0 1 .75.75v3.5a.75.75 0 0 1-1.5 0v-3.5A.75.75 0 0 1 8 4Zm0 8a1 1 0 1 1 0-2 1 1 0 0 1 0 2Z"
);

/// Paints a line icon so its 24×24 drawing fills `rect`; the glow spills past it. Octicons fill `rect` exactly.
pub fn paint(ui: &Ui, rect: Rect, icon: &Icon, color: Color32) {
    let rect = if icon.uri.starts_with("bytes://icon/") {
        rect.expand(rect.width() / 6.0)
    } else {
        rect
    };
    Image::from_bytes(icon.uri, icon.svg.as_bytes())
        .tint(color)
        .paint_at(ui, rect);
}
