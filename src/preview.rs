//! The Markdown preview, laid out like GitHub's dark theme (github-markdown-css), scaled with the zoom.
//! Sizes are GitHub's CSS pixels times `z`.

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::ops::Range;
use std::path::Path;
use std::sync::Arc;

use egui::text::{LayoutJob, TextFormat};
use egui::{
    Align, Color32, FontFamily, FontId, Galley, Label, Pos2, Rect, ScrollArea, Sense, Stroke, Ui,
    UiBuilder, Vec2, pos2, vec2,
};
use pulldown_cmark::Alignment;

use crate::highlight::{self, Span};
use crate::markdown::{self, Alert, Block, Doc, Inlines, Item, Piece, Style};
use crate::theme::{GH, GH_BOLD, MONO_BOLD, family};
use crate::{files, format, icons};

const fn rgba(hex: u32, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied_const((hex >> 16) as u8, (hex >> 8) as u8, hex as u8, a)
}

pub const BG: Color32 = rgba(0x0d1117, 0xff);
const FG: Color32 = rgba(0xf0f6fc, 0xff);
const MUTED: Color32 = rgba(0x9198a1, 0xff);
const LINK: Color32 = rgba(0x4493f8, 0xff);
const BORDER: Color32 = rgba(0x3d444d, 0xff);
const BORDER_MUTED: Color32 = rgba(0x3d444d, 0xb3);
const CODE_BG: Color32 = rgba(0x151b23, 0xff);
const INLINE_CODE_BG: Color32 = rgba(0x656c76, 0x33);
const CHECKED: Color32 = rgba(0x1f6feb, 0xff);

#[derive(Default)]
pub struct Preview {
    hash: u64,
    doc: Doc,
    /// Highlighting is slow, so a code block keeps its colors while other parts of the document change.
    code: HashMap<u64, Arc<Vec<Span>>>,
    /// A `#anchor` that was clicked; the preview scrolls there when it draws it.
    scroll_to: Option<String>,
}

/// How a piece of text looks where it is.
#[derive(Clone, Copy)]
struct Look {
    size: f32,
    color: Color32,
    bold: bool,
    /// Line height as a factor of the size.
    line: f32,
}

fn hash(value: impl Hash) -> u64 {
    let mut h = DefaultHasher::new();
    value.hash(&mut h);
    h.finish()
}

/// `#Some-Heading`, `#user-content-some-heading` and `#fn-1` all name the same places as the ids here.
fn anchor(target: &str) -> String {
    let target = target.trim_start_matches('#');
    target
        .strip_prefix("user-content-")
        .unwrap_or(target)
        .to_lowercase()
}

impl Preview {
    /// Draws the document. Returns a clicked link that isn't an anchor in it: a web address or a file.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        text: &str,
        path: Option<&Path>,
        zoom: f32,
        id: impl egui::AsIdSalt,
    ) -> Option<String> {
        let h = hash(text);
        if h != self.hash {
            self.hash = h;
            self.doc = markdown::parse(&format::for_preview(text));
            if self.code.len() > 256 {
                self.code.clear();
            }
        }
        let z = zoom;
        let mut r = Render {
            z,
            links: &self.doc.links,
            base: path,
            code: &mut self.code,
            scroll_to: &mut self.scroll_to,
            clicked: None,
            n: 0,
        };
        egui::Frame::new().fill(BG).show(ui, |ui| {
            ScrollArea::vertical()
                .id_salt(id)
                .auto_shrink(false)
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = Vec2::ZERO;
                    let width = ui.available_width();
                    // GitHub's .markdown-body: at most 980 wide and centered, 45 padding (15 on narrow screens).
                    let pad = if width < 768.0 * z {
                        15.0 * z
                    } else {
                        45.0 * z
                    };
                    let outer = width.min(980.0 * z);
                    let content = (outer - 2.0 * pad).max(40.0);
                    ui.add_space(32.0 * z);
                    ui.horizontal_top(|ui| {
                        ui.add_space((width - outer) / 2.0 + pad);
                        ui.vertical(|ui| {
                            ui.set_width(content);
                            let body = Look {
                                size: 16.0 * z,
                                color: FG,
                                bold: false,
                                line: 1.5,
                            };
                            r.blocks(ui, &self.doc.blocks, body, 0);
                            r.footnotes(ui, &self.doc);
                        });
                    });
                    ui.add_space(64.0 * z);
                });
        });
        r.clicked
    }
}

struct Render<'a> {
    z: f32,
    links: &'a [String],
    base: Option<&'a Path>,
    code: &'a mut HashMap<u64, Arc<Vec<Span>>>,
    scroll_to: &'a mut Option<String>,
    clicked: Option<String>,
    /// Numbers the code blocks and tables, for their scroll positions.
    n: usize,
}

impl Render<'_> {
    fn margins(&self, block: &Block, depth: usize) -> (f32, f32) {
        let (top, bottom) = match block {
            Block::Plain(_) => (0.0, 0.0),
            Block::Heading { .. } => (24.0, 16.0),
            Block::Rule => (24.0, 24.0),
            Block::List { .. } if depth > 0 => (0.0, 0.0),
            _ => (0.0, 16.0),
        };
        (top * self.z, bottom * self.z)
    }

    /// Blocks with GitHub's margins between them, collapsed as in CSS. The first and last have none outside.
    fn blocks(&mut self, ui: &mut Ui, blocks: &[Block], look: Look, depth: usize) {
        let mut prev: Option<f32> = None;
        for block in blocks {
            let (top, bottom) = self.margins(block, depth);
            if let Some(prev) = prev {
                ui.add_space(prev.max(top));
            }
            self.block(ui, block, look, depth);
            prev = Some(bottom);
        }
    }

    fn block(&mut self, ui: &mut Ui, block: &Block, look: Look, depth: usize) {
        let z = self.z;
        match block {
            Block::Para(text) | Block::Plain(text) => self.inlines(ui, text, look),
            Block::Heading { level, text, id } => {
                let factor = [2.0, 1.5, 1.25, 1.0, 0.875, 0.85][(*level as usize).clamp(1, 6) - 1];
                let size = 16.0 * z * factor;
                let top = ui.cursor().min;
                let color = if *level == 6 { MUTED } else { look.color };
                self.inlines(
                    ui,
                    text,
                    Look {
                        size,
                        color,
                        bold: true,
                        line: 1.25,
                    },
                );
                if *level <= 2 {
                    ui.add_space(0.3 * size);
                    let (rect, _) = ui.allocate_exact_size(
                        vec2(ui.available_width(), z.max(1.0)),
                        Sense::hover(),
                    );
                    ui.painter().rect_filled(rect, 0.0, BORDER_MUTED);
                }
                self.anchor_here(ui, top, id);
            }
            Block::Code { lang, code } => self.code_block(ui, lang, code),
            Block::Quote(blocks) => {
                let bar = 0.25 * 16.0 * z;
                let rect = self.indented(ui, bar + 16.0 * z, 16.0 * z, |r, ui| {
                    r.blocks(
                        ui,
                        blocks,
                        Look {
                            color: MUTED,
                            ..look
                        },
                        depth,
                    )
                });
                ui.painter().rect_filled(
                    Rect::from_min_size(rect.min, vec2(bar, rect.height())),
                    0.0,
                    BORDER,
                );
            }
            Block::Alert(kind, blocks) => self.alert(ui, *kind, blocks, look, depth),
            Block::List { start, items } => self.list(ui, *start, items, look, depth),
            Block::Table { aligns, head, rows } => self.table(ui, aligns, head, rows, look),
            Block::Rule => {
                let (rect, _) =
                    ui.allocate_exact_size(vec2(ui.available_width(), 4.0 * z), Sense::hover());
                ui.painter().rect_filled(rect, 0.0, BORDER);
            }
        }
    }

    /// Scrolls to this spot if its anchor was clicked.
    fn anchor_here(&mut self, ui: &Ui, top: Pos2, id: &str) {
        if self.scroll_to.as_deref() == Some(id) {
            ui.scroll_to_rect(Rect::from_min_size(top, vec2(1.0, 1.0)), Some(Align::TOP));
            *self.scroll_to = None;
        }
    }

    /// Content shifted right by `left` and narrowed by `right`. Returns the whole area it took.
    fn indented(
        &mut self,
        ui: &mut Ui,
        left: f32,
        right: f32,
        content: impl FnOnce(&mut Self, &mut Ui),
    ) -> Rect {
        ui.horizontal_top(|ui| {
            ui.add_space(left);
            ui.vertical(|ui| {
                ui.set_width((ui.available_width() - right).max(20.0));
                content(self, ui);
            });
        })
        .response
        .rect
    }

    // ── Text ──────────────────────────────────────────────────────────

    fn job(&self, text: &str, runs: &[(Range<usize>, Style)], look: Look, wrap: f32) -> LayoutJob {
        let mut job = LayoutJob::default();
        job.wrap.max_width = wrap;
        let code_pad = 0.4 * 0.85 * look.size;
        let mut after_code = false;
        let mut end = 0;
        let plain = Style::default();
        // Runs cover the text; anything they miss is plain.
        let gaps = runs.iter().flat_map(|(r, st)| {
            let gap = (end < r.start).then(|| (end..r.start, &plain));
            end = r.end;
            gap.into_iter().chain([(r.clone(), st)])
        });
        let pieces: Vec<_> = gaps.collect();
        let tail = (end < text.len()).then(|| (end..text.len(), &plain));
        for (range, st) in pieces.into_iter().chain(tail) {
            let size = if st.code {
                look.size * 0.85
            } else if st.sup {
                look.size * 0.75
            } else {
                look.size
            };
            let font = if st.code {
                FontFamily::Monospace
            } else if st.strong || look.bold {
                family(GH_BOLD)
            } else {
                family(GH)
            };
            let color = if st.link.is_some() { LINK } else { look.color };
            let format = TextFormat {
                font_id: FontId::new(size, font),
                color,
                line_height: Some(look.size * look.line),
                italics: st.em,
                strikethrough: if st.strike {
                    Stroke::new(look.size / 16.0, color)
                } else {
                    Stroke::NONE
                },
                valign: if st.sup { Align::TOP } else { Align::BOTTOM },
                ..TextFormat::default()
            };
            // Inline code has .4em of padding on each side.
            let lead = if st.code || after_code { code_pad } else { 0.0 };
            job.append(&text[range], lead, format);
            after_code = st.code;
        }
        job
    }

    fn inlines(&mut self, ui: &mut Ui, text: &Inlines, look: Look) {
        let has_image = text.0.iter().any(|p| matches!(p, Piece::Image { .. }));
        let only_images = text.0.iter().all(|p| {
            matches!(p, Piece::Text { text, .. } if text.trim().is_empty())
                || matches!(p, Piece::Image { .. })
        });
        if has_image && only_images {
            // A row of badges or pictures stays a row.
            ui.horizontal_wrapped(|ui| {
                for piece in &text.0 {
                    if let Piece::Image { url, alt, link } = piece {
                        self.image(ui, url, alt, *link, look);
                    }
                }
            });
            return;
        }
        for piece in &text.0 {
            match piece {
                Piece::Text { text, runs } => {
                    let galley =
                        ui.painter()
                            .layout_job(self.job(text, runs, look, ui.available_width()));
                    let height = galley.size().y.max(look.size * look.line);
                    let (rect, _) =
                        ui.allocate_exact_size(vec2(galley.size().x, height), Sense::hover());
                    self.text(ui, rect.min, galley, runs);
                }
                Piece::Image { url, alt, link } => self.image(ui, url, alt, *link, look),
            }
        }
    }

    /// Paints laid-out text at `pos` as a selectable label, with inline code boxes, and handles its links.
    fn text(
        &mut self,
        ui: &mut Ui,
        pos: Pos2,
        galley: Arc<Galley>,
        runs: &[(Range<usize>, Style)],
    ) {
        let rect = Rect::from_min_size(pos, galley.size());
        if !ui.is_rect_visible(rect) {
            return;
        }
        // Boxes around inline code and links, one per row: (link or None for code, rect, baseline).
        let mut boxes: Vec<(Option<u32>, Rect, f32)> = Vec::new();
        if runs.iter().any(|(_, s)| s.code || s.link.is_some()) {
            let style_at = |b: usize| {
                runs.iter()
                    .find(|(r, _)| r.contains(&b))
                    .map(|(_, s)| *s)
                    .unwrap_or_default()
            };
            let mut chars = galley.job.text.char_indices();
            for row in &galley.rows {
                for glyph in &row.glyphs {
                    let Some((b, _)) = chars.next() else { break };
                    let st = style_at(b);
                    if !st.code && st.link.is_none() {
                        continue;
                    }
                    let min = pos + row.pos.to_vec2() + glyph.pos.to_vec2()
                        - vec2(0.0, glyph.font_ascent);
                    let r = Rect::from_min_size(min, vec2(glyph.advance_width, glyph.font_height));
                    let baseline = pos.y + row.pos.y + glyph.pos.y;
                    let key = if st.code { None } else { st.link };
                    match boxes.last_mut() {
                        Some((k, last, _))
                            if *k == key
                                && (last.top() - r.top()).abs() < 0.5
                                && (last.right() - r.left()).abs() < 1.0 =>
                        {
                            *last = last.union(r)
                        }
                        _ => boxes.push((key, r, baseline)),
                    }
                }
                if row.ends_with_newline {
                    chars.next();
                }
            }
        }
        for (key, r, _) in &boxes {
            if key.is_none() {
                let pad = vec2(0.4 * r.height() * 0.85, 0.2 * r.height() * 0.85);
                ui.painter()
                    .rect_filled(r.expand2(pad), 6.0 * self.z, INLINE_CODE_BG);
            }
        }
        let mut child = ui.new_child(UiBuilder::new().max_rect(rect));
        child.add(Label::new(galley).selectable(true));

        let hovered = boxes
            .iter()
            .find(|(k, r, _)| k.is_some() && ui.rect_contains_pointer(*r))
            .and_then(|(k, ..)| *k);
        if let Some(link) = hovered {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            for (_, r, baseline) in boxes.iter().filter(|(k, ..)| *k == Some(link)) {
                let y = baseline + 1.5 * self.z;
                ui.painter()
                    .hline(r.x_range(), y, Stroke::new(self.z.max(1.0), LINK));
            }
            if ui.input(|i| i.pointer.primary_clicked()) {
                self.follow(link);
            }
        }
        // A footnote reference is the target of the note's way back.
        for (key, r, _) in &boxes {
            if let Some(target) = key.and_then(|k| self.links.get(k as usize)) {
                if let Some(label) = target.strip_prefix("#user-content-fn-") {
                    self.anchor_here(ui, r.min, &format!("fnref-{}", label.to_lowercase()));
                }
            }
        }
    }

    fn follow(&mut self, link: u32) {
        let Some(target) = self.links.get(link as usize) else {
            return;
        };
        if target.starts_with('#') {
            *self.scroll_to = Some(anchor(target));
        } else {
            self.clicked = Some(target.clone());
        }
    }

    fn image(&mut self, ui: &mut Ui, url: &str, alt: &str, link: Option<u32>, look: Look) {
        let uri = if url.starts_with("http://") || url.starts_with("https://") {
            Some(url.to_owned())
        } else if url.contains(':') && !url.get(1..2).is_some_and(|c| c == ":") {
            None // data: and other schemes
        } else {
            self.base
                .map(|base| format!("file://{}", files::resolve(base, url).display()))
        };
        let Some(uri) = uri else {
            // Without a saved file there is no folder to find a relative picture in.
            let galley = ui.painter().layout_job(self.job(
                alt,
                &[],
                Look {
                    color: MUTED,
                    ..look
                },
                ui.available_width(),
            ));
            let (rect, _) = ui.allocate_exact_size(galley.size(), Sense::hover());
            ui.painter().galley(rect.min, galley, MUTED);
            return;
        };
        let image = egui::Image::new(uri)
            .fit_to_original_size(self.z)
            .max_width(ui.available_width())
            .alt_text(alt)
            .sense(if link.is_some() {
                Sense::click()
            } else {
                Sense::hover()
            });
        let response = ui.add(image);
        if let Some(link) = link {
            if response.clicked() {
                self.follow(link);
            }
            response.on_hover_cursor(egui::CursorIcon::PointingHand);
        }
    }

    // ── Blocks ────────────────────────────────────────────────────────

    fn code_block(&mut self, ui: &mut Ui, lang: &str, code: &str) {
        let z = self.z;
        let size = 16.0 * z * 0.85;
        let pad = 16.0 * z;
        let code = code.strip_suffix('\n').unwrap_or(code);
        let spans = self
            .code
            .entry(hash((lang, code)))
            .or_insert_with(|| Arc::new(highlight::spans(lang, code, true)))
            .clone();
        let format = |color: Color32, bold: bool| TextFormat {
            font_id: FontId::new(
                size,
                if bold {
                    family(MONO_BOLD)
                } else {
                    FontFamily::Monospace
                },
            ),
            color,
            line_height: Some(size * 1.45),
            ..TextFormat::default()
        };
        let mut job = LayoutJob::default();
        job.wrap.max_width = f32::INFINITY;
        let mut column = 0;
        let mut end = 0;
        for span in spans.iter() {
            if end < span.range.start {
                push_tabbed(
                    &mut job,
                    &code[end..span.range.start],
                    &mut column,
                    format(FG, false),
                );
            }
            push_tabbed(
                &mut job,
                &code[span.range.clone()],
                &mut column,
                format(span.color, span.bold),
            );
            end = span.range.end;
        }
        if end < code.len() {
            push_tabbed(&mut job, &code[end..], &mut column, format(FG, false));
        }
        let galley = ui.painter().layout_job(job);
        let height = galley.size().y.max(size * 1.45) + 2.0 * pad;
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
        ui.painter().rect_filled(rect, 6.0 * z, CODE_BG);
        if !ui.is_rect_visible(rect) {
            return;
        }
        self.n += 1;
        let inner = rect.shrink(pad);
        let mut child = ui.new_child(UiBuilder::new().max_rect(inner));
        ScrollArea::horizontal()
            .id_salt(("code", self.n))
            .auto_shrink([false, true])
            .show(&mut child, |ui| {
                ui.add(Label::new(galley).selectable(true));
            });
    }

    fn alert(&mut self, ui: &mut Ui, kind: Alert, blocks: &[Block], look: Look, depth: usize) {
        let z = self.z;
        let (color, icon, title) = match kind {
            Alert::Note => (rgba(0x4493f8, 0xff), &icons::NOTE, "Note"),
            Alert::Tip => (rgba(0x3fb950, 0xff), &icons::TIP, "Tip"),
            Alert::Important => (rgba(0xab7df8, 0xff), &icons::IMPORTANT, "Important"),
            Alert::Warning => (rgba(0xd29922, 0xff), &icons::WARNING, "Warning"),
            Alert::Caution => (rgba(0xf85149, 0xff), &icons::CAUTION, "Caution"),
        };
        let bar = 0.25 * 16.0 * z;
        let rect = self.indented(ui, bar + 16.0 * z, 16.0 * z, |r, ui| {
            ui.add_space(8.0 * z);
            let galley = ui.painter().layout_job(r.job(
                title,
                &[],
                Look {
                    color,
                    line: 1.0,
                    ..look
                },
                f32::INFINITY,
            ));
            let (row, _) = ui.allocate_exact_size(
                vec2(24.0 * z + galley.size().x, galley.size().y.max(16.0 * z)),
                Sense::hover(),
            );
            icons::paint(
                ui,
                Rect::from_min_size(
                    pos2(row.left(), row.center().y - 8.0 * z),
                    vec2(16.0 * z, 16.0 * z),
                ),
                icon,
                color,
            );
            ui.painter().galley(
                pos2(
                    row.left() + 24.0 * z,
                    row.center().y - galley.size().y / 2.0,
                ),
                galley,
                color,
            );
            if !blocks.is_empty() {
                ui.add_space(16.0 * z);
            }
            r.blocks(ui, blocks, look, depth);
            ui.add_space(8.0 * z);
        });
        ui.painter().rect_filled(
            Rect::from_min_size(rect.min, vec2(bar, rect.height())),
            0.0,
            color,
        );
    }

    fn list(&mut self, ui: &mut Ui, start: Option<u64>, items: &[Item], look: Look, depth: usize) {
        let z = self.z;
        // In a loose list every item is a paragraph, 16 apart; in a tight one .25em.
        let loose = items
            .iter()
            .any(|item| item.blocks.iter().any(|b| matches!(b, Block::Para(_))));
        let indent = 2.0 * look.size;
        for (i, item) in items.iter().enumerate() {
            if i > 0 {
                ui.add_space(if loose { 16.0 * z } else { 0.25 * look.size });
            }
            let rect = self.indented(ui, indent, 0.0, |r, ui| {
                r.blocks(ui, &item.blocks, look, depth + 1)
            });
            let content = pos2(rect.left() + indent, rect.top());
            let line = look.size * look.line;
            let painter = ui.painter();
            if let Some(checked) = item.task {
                let side = 13.0 * z;
                let b = Rect::from_min_size(
                    pos2(
                        content.x - side - 0.2 * look.size,
                        content.y + (line - side) / 2.0,
                    ),
                    vec2(side, side),
                );
                if checked {
                    painter.rect_filled(b, 3.0 * z, CHECKED);
                    let p = |x: f32, y: f32| b.min + vec2(x, y) * side;
                    painter.line(
                        vec![p(0.22, 0.52), p(0.43, 0.72), p(0.78, 0.3)],
                        Stroke::new(1.6 * z, Color32::WHITE),
                    );
                } else {
                    painter.rect_stroke(
                        b,
                        3.0 * z,
                        Stroke::new(z.max(1.0), MUTED),
                        egui::StrokeKind::Inside,
                    );
                }
            } else if let Some(start) = start {
                let n = start + i as u64;
                let marker = match depth {
                    0 => n.to_string(),
                    1 => roman(n),
                    _ => alpha(n),
                } + ".";
                let galley = painter.layout_job(self.job(&marker, &[], look, f32::INFINITY));
                painter.galley(
                    pos2(
                        content.x - 0.3 * look.size - galley.size().x,
                        content.y + (line - galley.size().y) / 2.0,
                    ),
                    galley,
                    look.color,
                );
            } else {
                let center = pos2(
                    content.x - 0.65 * look.size,
                    content.y + line / 2.0 + 0.04 * look.size,
                );
                let radius = 0.17 * look.size;
                match depth {
                    0 => painter.circle_filled(center, radius, look.color),
                    1 => {
                        painter.circle_stroke(center, radius - 0.5 * z, Stroke::new(z, look.color))
                    }
                    _ => painter.rect_filled(
                        Rect::from_center_size(center, Vec2::splat(radius * 1.8)),
                        0.0,
                        look.color,
                    ),
                };
            }
        }
    }

    fn table(
        &mut self,
        ui: &mut Ui,
        aligns: &[Alignment],
        head: &[Inlines],
        rows: &[Vec<Inlines>],
        look: Look,
    ) {
        let z = self.z;
        let pad = vec2(13.0 * z, 6.0 * z);
        let cols = rows
            .iter()
            .map(Vec::len)
            .chain([head.len()])
            .max()
            .unwrap_or(0);
        if cols == 0 {
            return;
        }
        // Tables keep their natural width, like GitHub's (`width: max-content`), and scroll when wider than the page.
        let lay = |r: &Self,
                   cells: &[Inlines],
                   bold: bool|
         -> Vec<(Arc<Galley>, Vec<(Range<usize>, Style)>)> {
            (0..cols)
                .map(|c| {
                    let (text, runs) = cells.get(c).map(flatten).unwrap_or_default();
                    (
                        ui.painter().layout_job(r.job(
                            &text,
                            &runs,
                            Look { bold, ..look },
                            f32::INFINITY,
                        )),
                        runs,
                    )
                })
                .collect()
        };
        let mut grid = vec![lay(self, head, true)];
        grid.extend(rows.iter().map(|row| lay(self, row, false)));
        let line = look.size * look.line;
        let widths: Vec<f32> = (0..cols)
            .map(|c| grid.iter().map(|row| row[c].0.size().x).fold(0.0, f32::max) + 2.0 * pad.x)
            .collect();
        let heights: Vec<f32> = grid
            .iter()
            .map(|row| row.iter().map(|(g, _)| g.size().y).fold(line, f32::max) + 2.0 * pad.y)
            .collect();
        let size = vec2(widths.iter().sum(), heights.iter().sum());
        self.n += 1;
        ScrollArea::horizontal()
            .id_salt(("table", self.n))
            .auto_shrink([false, true])
            .show(ui, |ui| {
                // Half a line of room around it, for the outer borders.
                let (outer, _) =
                    ui.allocate_exact_size(size + Vec2::splat(z.max(1.0)), Sense::hover());
                if !ui.is_rect_visible(outer) {
                    return;
                }
                let rect = Rect::from_min_size(outer.min + Vec2::splat(z.max(1.0) / 2.0), size);
                let mut y = rect.top();
                for (r, (row, height)) in grid.into_iter().zip(&heights).enumerate() {
                    let row_rect = Rect::from_min_size(pos2(rect.left(), y), vec2(size.x, *height));
                    ui.painter().rect_filled(
                        row_rect,
                        0.0,
                        if r > 0 && r % 2 == 0 { CODE_BG } else { BG },
                    );
                    let mut x = rect.left();
                    for (c, (galley, runs)) in row.into_iter().enumerate() {
                        let cell = Rect::from_min_size(pos2(x, y), vec2(widths[c], *height));
                        ui.painter().rect_stroke(
                            cell,
                            0.0,
                            Stroke::new(z.max(1.0), BORDER),
                            // Neighbours share the line, as with `border-collapse`.
                            egui::StrokeKind::Middle,
                        );
                        let free = widths[c] - 2.0 * pad.x - galley.size().x;
                        let shift = match aligns.get(c) {
                            Some(Alignment::Center) => free / 2.0,
                            Some(Alignment::Right) => free,
                            _ => 0.0,
                        };
                        self.text(ui, pos2(x + pad.x + shift, y + pad.y), galley, &runs);
                        x += widths[c];
                    }
                    y += height;
                }
            });
    }

    fn footnotes(&mut self, ui: &mut Ui, doc: &Doc) {
        if doc.footnotes.is_empty() {
            return;
        }
        let z = self.z;
        ui.add_space(16.0 * z);
        let (rule, _) =
            ui.allocate_exact_size(vec2(ui.available_width(), z.max(1.0)), Sense::hover());
        ui.painter().rect_filled(rule, 0.0, BORDER);
        let look = Look {
            size: 12.0 * z,
            color: MUTED,
            bold: false,
            line: 1.5,
        };
        for (i, note) in doc.footnotes.iter().enumerate() {
            ui.add_space(16.0 * z);
            let top = ui.cursor().min;
            let rect = self.indented(ui, 16.0 * z, 0.0, |r, ui| {
                r.blocks(ui, &note.blocks, look, 1)
            });
            let marker =
                ui.painter()
                    .layout_job(self.job(&format!("{}.", i + 1), &[], look, f32::INFINITY));
            ui.painter().galley(
                pos2(
                    rect.left() + 16.0 * z - 0.3 * look.size - marker.size().x,
                    rect.top(),
                ),
                marker,
                MUTED,
            );
            self.anchor_here(ui, top, &format!("fn-{}", note.label.to_lowercase()));
        }
    }
}

/// Code with its tabs as spaces to the next multiple of 8 columns, as browsers show them.
fn push_tabbed(job: &mut LayoutJob, text: &str, column: &mut usize, format: TextFormat) {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\t' => {
                let n = 8 - *column % 8;
                out.extend(std::iter::repeat_n(' ', n));
                *column += n;
            }
            '\n' => {
                out.push(c);
                *column = 0;
            }
            _ => {
                out.push(c);
                *column += 1;
            }
        }
    }
    job.append(&out, 0.0, format);
}

/// A cell's text in one piece; pictures in table cells show their alt text.
fn flatten(text: &Inlines) -> (String, Vec<(Range<usize>, Style)>) {
    let mut out = String::new();
    let mut runs = Vec::new();
    for piece in &text.0 {
        let base = out.len();
        match piece {
            Piece::Text { text, runs: r } => {
                out.push_str(text);
                runs.extend(
                    r.iter()
                        .map(|(range, st)| (range.start + base..range.end + base, *st)),
                );
            }
            Piece::Image { alt, .. } => {
                out.push_str(alt);
                runs.push((base..out.len(), Style::default()));
            }
        }
    }
    (out, runs)
}

fn roman(mut n: u64) -> String {
    const NUMERALS: [(u64, &str); 13] = [
        (1000, "m"),
        (900, "cm"),
        (500, "d"),
        (400, "cd"),
        (100, "c"),
        (90, "xc"),
        (50, "l"),
        (40, "xl"),
        (10, "x"),
        (9, "ix"),
        (5, "v"),
        (4, "iv"),
        (1, "i"),
    ];
    let mut out = String::new();
    for (value, numeral) in NUMERALS {
        while n >= value {
            out.push_str(numeral);
            n -= value;
        }
    }
    out
}

fn alpha(mut n: u64) -> String {
    let mut out = Vec::new();
    while n > 0 {
        n -= 1;
        out.push(b'a' + (n % 26) as u8);
        n /= 26;
    }
    out.reverse();
    String::from_utf8(out).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_numbers() {
        assert_eq!(
            [1, 4, 9, 14, 2024].map(roman),
            ["i", "iv", "ix", "xiv", "mmxxiv"]
        );
        assert_eq!([1, 26, 27, 52].map(alpha), ["a", "z", "aa", "az"]);
    }

    #[test]
    fn anchors_match_ids() {
        assert_eq!(anchor("#user-content-fn-1"), "fn-1");
        assert_eq!(anchor("#Hello-World"), "hello-world");
    }

    /// Link and inline-code boxes assume one glyph per char, wrapped rows included.
    #[test]
    fn galleys_have_a_glyph_per_char() {
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(Default::default(), |ui| {
            let text = "Some words that wrap across rows,\nwith a break and   spaces.";
            let mut job = LayoutJob::simple(
                text.into(),
                FontId::proportional(14.0),
                Color32::WHITE,
                60.0,
            );
            job.wrap.max_width = 60.0;
            let galley = ui.painter().layout_job(job);
            assert!(galley.rows.len() > 3);
            let glyphs: usize = galley
                .rows
                .iter()
                .map(|r| r.glyphs.len() + r.ends_with_newline as usize)
                .sum();
            assert_eq!(glyphs, text.chars().count());
        });
        output.textures_delta.clear();
    }
}
