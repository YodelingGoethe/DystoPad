//! The neon look from `docs/Stylingguidelines.md`, drawn with egui: palette, fonts, glows and the small widgets.
//! CSS's `color-mix()` becomes `mix`, and a glow is a blurred shadow behind a shape or faint copies around text.

use std::sync::Arc;

use egui::epaint::{CornerRadius, Shadow};
use egui::{
    Color32, Context, FontData, FontDefinitions, FontFamily, FontId, Frame, Galley, Margin,
    Painter, Pos2, Rect, Response, Sense, Stroke, TextStyle, Ui, pos2, vec2,
};

use crate::icons::{self, Icon};

const fn rgb(hex: u32) -> Color32 {
    Color32::from_rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

pub const BG: Color32 = rgb(0x030308);
pub const LINE: Color32 = rgb(0x4cc9ff);
pub const TEXT: Color32 = rgb(0xe8f3ff);
pub const MUTED: Color32 = rgb(0x7f8bab);
pub const OK: Color32 = rgb(0x36ff8a);
pub const DANGER: Color32 = rgb(0xff3b5c);
pub const MONEY: Color32 = rgb(0xffd84d);
pub const LOGO: Color32 = rgb(0xff3fb4);
pub const POPUP: Color32 = rgb(0x070912);
/// The editor's background: the guide's panel fill over the page.
pub const EDITOR_BG: Color32 = rgb(0x04050c);

/// The guide's category colors, used for highlighting in the editor.
pub mod category {
    use super::*;
    pub const PINK_RED: Color32 = rgb(0xff2d6f);
    pub const AMBER: Color32 = rgb(0xffb627);
    pub const AQUA: Color32 = rgb(0x00ffd0);
    pub const BLUE: Color32 = rgb(0x3f8cff);
    pub const LIME: Color32 = rgb(0xa6ff00);
    pub const VIOLET: Color32 = rgb(0xc77dff);
}

/// `color-mix(in srgb, a t, b)`.
pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let m = |x: u8, y: u8| (x as f32 * t + y as f32 * (1.0 - t)).round() as u8;
    Color32::from_rgba_unmultiplied(
        m(a.r(), b.r()),
        m(a.g(), b.g()),
        m(a.b(), b.b()),
        m(a.a(), b.a()),
    )
}

/// A color mixed with white, for text that has to stay readable.
pub fn tint(c: Color32, t: f32) -> Color32 {
    mix(c, Color32::WHITE, t)
}

/// `color-mix(in srgb, c t, transparent)`.
pub fn fade(c: Color32, t: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (t * 255.0).round() as u8)
}

/// The hairline between the bars.
pub fn hair() -> Color32 {
    fade(LINE, 0.22)
}

pub fn glow(c: Color32, blur: u8, strength: f32) -> Shadow {
    Shadow {
        offset: [0, 0],
        blur,
        spread: 0,
        color: fade(c, strength),
    }
}

/// `.neon-box` on the popup fill: dialogs and menus.
pub fn neon_box(c: Color32) -> Frame {
    Frame::new()
        .fill(POPUP)
        .stroke(Stroke::new(1.5, tint(c, 0.55)))
        .corner_radius(12)
        .shadow(glow(c, 16, 0.5))
        .inner_margin(Margin::symmetric(26, 22))
}

// ── Fonts ─────────────────────────────────────────────────────────────

pub const NEON: &str = "neon";
pub const UI_BOLD: &str = "ui-bold";
pub const MONO_BOLD: &str = "mono-bold";
/// GitHub's text font in the preview.
pub const GH: &str = "gh";
pub const GH_BOLD: &str = "gh-bold";

pub fn family(name: &str) -> FontFamily {
    FontFamily::Name(name.into())
}

/// The first of these system font files that exists.
fn system_font(candidates: &[&str]) -> Option<FontData> {
    let windir = std::env::var("WINDIR").unwrap_or_else(|_| r"C:\Windows".into());
    candidates.iter().find_map(|c| {
        let path = c.replace("%FONTS%", &format!(r"{windir}\Fonts"));
        std::fs::read(path).ok().map(FontData::from_owned)
    })
}

pub fn install_fonts(ctx: &Context) {
    let mut fonts = FontDefinitions::default();
    let mut add = |name: &str, data: FontData| {
        fonts.font_data.insert(name.into(), Arc::new(data));
    };
    add(
        "Quicksand",
        FontData::from_static(include_bytes!("../assets/fonts/Quicksand-Regular.ttf")),
    );
    add(
        "Quicksand SemiBold",
        FontData::from_static(include_bytes!("../assets/fonts/Quicksand-SemiBold.ttf")),
    );
    add(
        "Tilt Neon",
        FontData::from_static(include_bytes!("../assets/fonts/TiltNeon-Regular.ttf")),
    );

    // The text you edit uses the system's monospace font, as Notepad does; the preview uses GitHub's fonts.
    // Missing ones fall back to egui's built-in Hack and Ubuntu.
    let loaded = [
        (
            "mono",
            &[
                "%FONTS%/consola.ttf",
                "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
                "/usr/share/fonts/TTF/DejaVuSansMono.ttf",
            ][..],
        ),
        (
            "mono bold",
            &[
                "%FONTS%/consolab.ttf",
                "/usr/share/fonts/truetype/dejavu/DejaVuSansMono-Bold.ttf",
                "/usr/share/fonts/TTF/DejaVuSansMono-Bold.ttf",
            ],
        ),
        (
            "gh",
            &[
                "%FONTS%/segoeui.ttf",
                "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
                "/usr/share/fonts/noto/NotoSans-Regular.ttf",
                "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            ],
        ),
        (
            "gh bold",
            &[
                "%FONTS%/seguisb.ttf",
                "/usr/share/fonts/truetype/noto/NotoSans-SemiBold.ttf",
                "/usr/share/fonts/noto/NotoSans-SemiBold.ttf",
                "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
            ],
        ),
    ]
    .map(|(name, candidates)| match system_font(candidates) {
        Some(data) => {
            add(name, data);
            Some(name)
        }
        None => None,
    });
    let [mono, mono_bold, gh, gh_bold] = loaded;
    let mono = mono.unwrap_or("Hack");
    let gh = gh.unwrap_or("Ubuntu-Light");

    // egui's own fonts stay behind ours, for symbols ours lack.
    let fallbacks = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let mut set = |family: FontFamily, first: &[&str]| {
        let list: Vec<String> = first
            .iter()
            .map(|s| s.to_string())
            .chain(fallbacks.iter().cloned())
            .collect();
        fonts.families.insert(family, list);
    };
    set(FontFamily::Proportional, &["Quicksand"]);
    set(FontFamily::Monospace, &[mono]);
    set(family(NEON), &["Tilt Neon", "Quicksand"]);
    set(family(UI_BOLD), &["Quicksand SemiBold"]);
    set(family(MONO_BOLD), &[mono_bold.unwrap_or(mono), mono]);
    set(family(GH), &[gh]);
    set(family(GH_BOLD), &[gh_bold.unwrap_or(gh), gh]);
    ctx.set_fonts(fonts);
}

// ── Style ─────────────────────────────────────────────────────────────

pub fn install_style(ctx: &Context) {
    ctx.set_theme(egui::Theme::Dark);
    ctx.all_styles_mut(|style| {
        style.text_styles = [
            (TextStyle::Small, FontId::proportional(12.0)),
            (TextStyle::Body, FontId::proportional(14.0)),
            (TextStyle::Button, FontId::proportional(14.0)),
            (TextStyle::Heading, FontId::new(22.0, family(NEON))),
            (TextStyle::Monospace, FontId::monospace(15.0)),
        ]
        .into();
        style.spacing.item_spacing = vec2(8.0, 6.0);
        style.spacing.button_padding = vec2(10.0, 5.0);
        style.spacing.menu_margin = Margin::same(6);
        style.spacing.scroll.bar_width = 8.0;
        style.interaction.selectable_labels = false;

        let v = &mut style.visuals;
        v.dark_mode = true;
        v.panel_fill = BG;
        v.window_fill = POPUP;
        v.extreme_bg_color = rgb(0x04060f);
        v.faint_bg_color = fade(LINE, 0.06);
        v.code_bg_color = rgb(0x0b0e1a);
        v.hyperlink_color = tint(LINE, 0.5);
        v.window_stroke = Stroke::new(1.5, tint(LINE, 0.55));
        v.window_corner_radius = CornerRadius::same(12);
        v.menu_corner_radius = CornerRadius::same(12);
        v.window_shadow = glow(LINE, 16, 0.45);
        v.popup_shadow = glow(LINE, 14, 0.45);
        v.selection.bg_fill = fade(LINE, 0.28);
        v.selection.stroke = Stroke::new(1.0, TEXT);
        v.text_cursor.stroke = Stroke::new(2.0, LINE);
        v.override_text_color = None;

        let w = &mut v.widgets;
        w.noninteractive.bg_stroke = Stroke::new(1.0, hair());
        w.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);
        w.noninteractive.bg_fill = Color32::TRANSPARENT;
        for (state, fill, stroke) in [
            (&mut w.inactive, Color32::TRANSPARENT, fade(LINE, 0.35)),
            (&mut w.hovered, fade(LINE, 0.14), fade(LINE, 0.6)),
            (&mut w.active, fade(LINE, 0.22), LINE),
            (&mut w.open, fade(LINE, 0.14), fade(LINE, 0.6)),
        ] {
            state.bg_fill = fill;
            state.weak_bg_fill = fill;
            state.bg_stroke = Stroke::new(1.0, stroke);
            state.fg_stroke = Stroke::new(1.0, TEXT);
            state.corner_radius = CornerRadius::same(8);
            state.expansion = 0.0;
        }
        w.inactive.bg_stroke = Stroke::NONE;
    });
}

// ── Glowing text and icons ────────────────────────────────────────────

/// Text that glows: faint copies around it in the glow color, then the text itself.
pub fn paint_glow_text(
    painter: &Painter,
    pos: Pos2,
    galley: &Arc<Galley>,
    glow_color: Color32,
    text_color: Color32,
) {
    for (d, a) in [(1.0, 0.35), (2.0, 0.12)] {
        for off in [vec2(d, 0.0), vec2(-d, 0.0), vec2(0.0, d), vec2(0.0, -d)] {
            painter.galley_with_override_text_color(pos + off, galley.clone(), fade(glow_color, a));
        }
    }
    painter.galley_with_override_text_color(pos, galley.clone(), text_color);
}

/// `.neon-text`: Tilt Neon, the color mixed with white, glowing in the color.
pub fn neon_text(ui: &mut Ui, text: &str, size: f32, c: Color32) -> Response {
    let galley =
        ui.painter()
            .layout_no_wrap(text.into(), FontId::new(size, family(NEON)), Color32::WHITE);
    let (rect, response) = ui.allocate_exact_size(galley.size() + vec2(4.0, 4.0), Sense::hover());
    paint_glow_text(
        ui.painter(),
        rect.min + vec2(2.0, 2.0),
        &galley,
        c,
        tint(c, 0.3),
    );
    response
}

// ── Widgets ───────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
pub enum ButtonKind {
    Normal,
    Primary,
    Danger,
}

/// `.btn`: an outlined neon tube, filled faintly on hover.
pub fn button(ui: &mut Ui, label: &str, kind: ButtonKind) -> Response {
    let c = match kind {
        ButtonKind::Normal => LINE,
        ButtonKind::Primary => OK,
        ButtonKind::Danger => DANGER,
    };
    let galley = ui.painter().layout_no_wrap(
        label.into(),
        FontId::new(15.0, family(NEON)),
        Color32::WHITE,
    );
    let size = galley.size() + vec2(36.0, 16.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let hovered = response.hovered() || response.has_focus();
    let painter = ui.painter();
    let radius = CornerRadius::same(10);
    painter.add(
        glow(
            c,
            if hovered { 18 } else { 9 },
            if hovered { 0.7 } else { 0.4 },
        )
        .as_shape(rect, radius),
    );
    painter.rect(
        rect,
        radius,
        if hovered { mix(c, POPUP, 0.14) } else { POPUP },
        Stroke::new(1.5, tint(c, 0.6)),
        egui::StrokeKind::Inside,
    );
    let pos = rect.center() - galley.size() / 2.0;
    paint_glow_text(painter, pos, &galley, c, tint(c, 0.25));
    if hovered {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

/// `.toggle`: a small switch, green when on.
pub fn switch(ui: &mut Ui, on: &mut bool, label: &str) -> Response {
    let galley = ui
        .painter()
        .layout_no_wrap(label.into(), FontId::proportional(13.0), TEXT);
    let track = vec2(30.0, 16.0);
    let size = vec2(
        track.x + 8.0 + galley.size().x,
        track.y.max(galley.size().y),
    );
    let (rect, mut response) = ui.allocate_exact_size(size, Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    let t = ui.ctx().animate_bool(response.id, *on);
    let painter = ui.painter();
    let track_rect = Rect::from_min_size(pos2(rect.left(), rect.center().y - track.y / 2.0), track);
    let c = mix(OK, MUTED, t);
    if *on {
        painter.add(glow(OK, 8, 0.6).as_shape(track_rect, CornerRadius::same(8)));
    }
    painter.rect_stroke(
        track_rect,
        8.0,
        Stroke::new(1.5, fade(c, 0.7 + 0.3 * t)),
        egui::StrokeKind::Inside,
    );
    let knob_x = egui::lerp(track_rect.left() + 7.5..=track_rect.right() - 7.5, t);
    painter.circle_filled(
        pos2(knob_x, track_rect.center().y),
        4.5,
        mix(rgb(0xeafff2), MUTED, t),
    );
    painter.galley(
        pos2(
            track_rect.right() + 8.0,
            rect.center().y - galley.size().y / 2.0,
        ),
        galley,
        TEXT,
    );
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

/// `.pill-toggle`: one choice of several. Returns the clicked option.
pub fn pills(ui: &mut Ui, options: &[&str], selected: usize) -> Option<usize> {
    let font = FontId::proportional(13.0);
    let galleys: Vec<_> = options
        .iter()
        .map(|o| ui.painter().layout_no_wrap((*o).into(), font.clone(), TEXT))
        .collect();
    let pad = vec2(14.0, 5.0);
    let widths: Vec<f32> = galleys.iter().map(|g| g.size().x + 2.0 * pad.x).collect();
    let height = galleys[0].size().y + 2.0 * pad.y;
    let (rect, _) = ui.allocate_exact_size(vec2(widths.iter().sum(), height), Sense::hover());
    let radius = CornerRadius::same((height / 2.0) as u8);
    let mut clicked = None;
    let mut x = rect.left();
    for (i, (galley, w)) in galleys.into_iter().zip(&widths).enumerate() {
        let seg = Rect::from_min_size(pos2(x, rect.top()), vec2(*w, height));
        let response = ui.interact(seg, ui.scope_id().with(("pill", i)), Sense::click());
        let on = i == selected;
        if on || response.hovered() {
            let r = match i {
                0 => CornerRadius {
                    nw: radius.nw,
                    sw: radius.sw,
                    ..Default::default()
                },
                _ if i == options.len() - 1 => CornerRadius {
                    ne: radius.ne,
                    se: radius.se,
                    ..Default::default()
                },
                _ => CornerRadius::ZERO,
            };
            ui.painter()
                .rect_filled(seg, r, fade(LINE, if on { 0.22 } else { 0.1 }));
        }
        let pos = seg.center() - galley.size() / 2.0;
        if on {
            paint_glow_text(ui.painter(), pos, &galley, LINE, Color32::WHITE);
        } else {
            ui.painter()
                .galley_with_override_text_color(pos, galley, MUTED);
        }
        if response.clicked() {
            clicked = Some(i);
        }
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        x += w;
    }
    ui.painter().rect_stroke(
        rect,
        radius,
        Stroke::new(1.0, fade(LINE, 0.4)),
        egui::StrokeKind::Inside,
    );
    clicked
}

/// `.fmt-btn`: a square icon button for the bars. A dropdown gets a small arrow; `open` keeps it lit.
pub fn icon_button(
    ui: &mut Ui,
    icon: &Icon,
    tooltip: &str,
    enabled: bool,
    dropdown: bool,
    open: bool,
) -> Response {
    let size = vec2(if dropdown { 46.0 } else { 32.0 }, 32.0);
    let (rect, response) = ui.allocate_exact_size(
        size,
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    if enabled && (response.hovered() || open) {
        ui.painter().rect_filled(rect, 8.0, fade(LINE, 0.14));
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let color = if enabled {
        tint(LINE, 0.55)
    } else {
        fade(tint(LINE, 0.55), 0.32)
    };
    let icon_rect =
        Rect::from_center_size(pos2(rect.left() + 16.0, rect.center().y), vec2(18.0, 18.0));
    icons::paint(ui, icon_rect, icon, color);
    if dropdown {
        icons::paint(
            ui,
            Rect::from_center_size(pos2(rect.right() - 10.0, rect.center().y), vec2(12.0, 12.0)),
            &icons::CHEVRON,
            color,
        );
    }
    response.on_hover_text(tooltip)
}

/// `.btn`s side by side, centered. Returns the index of the clicked one.
pub fn button_row(ui: &mut Ui, buttons: &[(&str, ButtonKind)]) -> Option<usize> {
    let font = FontId::new(15.0, family(NEON));
    let gap = 10.0;
    let width: f32 = buttons
        .iter()
        .map(|(l, _)| {
            ui.painter()
                .layout_no_wrap((*l).into(), font.clone(), TEXT)
                .size()
                .x
                + 36.0
        })
        .sum::<f32>()
        + gap * (buttons.len() as f32 - 1.0);
    let mut clicked = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        ui.add_space(((ui.available_width() - width) / 2.0).max(0.0));
        for (i, (label, kind)) in buttons.iter().enumerate() {
            if button(ui, label, *kind).clicked() {
                clicked = Some(i);
            }
        }
    });
    clicked
}

/// A text field with a neon outline that glows while it has focus.
pub fn field(ui: &mut Ui, edit: egui::TextEdit) -> Response {
    // The frame goes under the text, so its place is taken before the field is drawn.
    let under = ui.painter().add(egui::Shape::Noop);
    let response = ui.add(edit.frame(Frame::NONE.inner_margin(Margin::symmetric(8, 5))));
    let (rect, focused) = (response.rect, response.has_focus());
    let mut shapes: Vec<egui::Shape> = Vec::new();
    if focused {
        shapes.push(
            glow(LINE, 8, 0.45)
                .as_shape(rect, CornerRadius::same(8))
                .into(),
        );
    }
    shapes.push(egui::Shape::rect_filled(rect, 8.0, rgb(0x04060f)));
    let stroke = Stroke::new(1.0, if focused { LINE } else { fade(LINE, 0.35) });
    shapes.push(egui::Shape::rect_stroke(
        rect,
        8.0,
        stroke,
        egui::StrokeKind::Inside,
    ));
    ui.painter().set(under, egui::Shape::Vec(shapes));
    response
}
