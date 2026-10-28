//! The window: tabs, menus, the editor and preview, the status bar, dialogs and the session.

use std::ops::Range;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use egui::containers::menu::{MenuBar, MenuButton};
use egui::epaint::CornerRadius;
use egui::text::{CCursor, LayoutJob, TextWrapping};
use egui::{
    Align, Align2, Area, Button, CentralPanel, Color32, ComboBox, Context, CursorIcon, Event,
    FontId, Frame, Id, Key, Layout, Margin, Modal, Modifiers, Order, Panel, Popup, Rect, RichText,
    ScrollArea, Sense, Stroke, TextEdit, Ui, UiBuilder, ViewportCommand, pos2, vec2,
};

use crate::format::{self, ListKind};
use crate::i18n::{Dict, Lang, fill};
use crate::preview::Preview;
use crate::search::{self, Query};
use crate::session::{self, SavedTab, Session, Settings, View};
use crate::theme::{self, ButtonKind, LINE, MUTED, TEXT, fade, family, tint};
use crate::{editor, files, icons, print, single_instance};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const CHANGELOG: &str = include_str!("../docs/CHANGELOG.md");

pub struct Tab {
    id: u64,
    path: Option<PathBuf>,
    text: String,
    /// The text as it is in the file; `None` when that is unknown, which counts as unsaved.
    saved: Option<String>,
    md: bool,
    /// Ignored while Markdown is off.
    view: View,
    crlf: bool,
    bom: bool,
    /// The selection in chars, as the editor last reported it.
    cursor: Range<usize>,
    /// Put the cursor back where the session left it, the first time the tab is shown.
    restore_cursor: bool,
    chars: usize,
    preview: Preview,
    hl: editor::Highlighter,
}

impl Tab {
    fn new(id: u64) -> Self {
        Self {
            id,
            path: None,
            text: String::new(),
            saved: Some(String::new()),
            md: false,
            view: View::Split,
            crlf: cfg!(windows),
            bom: false,
            cursor: 0..0,
            restore_cursor: false,
            chars: 0,
            preview: Preview::default(),
            hl: editor::Highlighter::default(),
        }
    }

    fn dirty(&self) -> bool {
        self.saved.as_deref() != Some(self.text.as_str())
    }

    fn set_text(&mut self, text: String) {
        self.chars = text.chars().count();
        self.text = text;
    }

    fn title(&self, untitled: &str) -> String {
        match &self.path {
            Some(path) => path.file_name().map_or_else(
                || path.display().to_string(),
                |n| n.to_string_lossy().into_owned(),
            ),
            None => first_words(&self.text).unwrap_or_else(|| untitled.to_owned()),
        }
    }

    fn editor_id(&self) -> Id {
        Id::unique(("doc", self.id))
    }

    fn shown_view(&self) -> View {
        if self.md { self.view } else { View::Editor }
    }
}

/// What an unsaved tab is called, as in Notepad: the start of its first line, without Markdown marks.
pub fn first_words(text: &str) -> Option<String> {
    let line = text
        .lines()
        .map(str::trim)
        .find(|l| !l.trim_matches(['#', '>', '-', '*', ' ']).is_empty())?;
    let line = line.trim_start_matches(['#', '>', ' ']);
    let line = format::parse_item(line)
        .map_or(line, |item| &line[item.indent.len() + item.marker.len()..]);
    let words = line.split_whitespace().collect::<Vec<_>>().join(" ");
    Some(words.chars().take(40).collect())
}

fn char_to_byte(text: &str, c: usize) -> usize {
    text.char_indices().nth(c).map_or(text.len(), |(b, _)| b)
}

fn byte_to_char(text: &str, b: usize) -> usize {
    text[..b].chars().count()
}

fn line_col(text: &str, cursor: usize) -> (usize, usize) {
    text.chars().take(cursor).fold((1, 1), |(line, col), c| {
        if c == '\n' {
            (line + 1, 1)
        } else {
            (line, col + 1)
        }
    })
}

#[derive(Clone, Copy)]
enum Fmt {
    Heading(usize),
    List(ListKind),
    Wrap(&'static str),
    Link,
    Table,
}

#[derive(Clone, Copy)]
enum Action {
    NewTab,
    Open,
    Save,
    SaveAs,
    Print,
    Close(u64),
    Activate(usize),
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    SelectAll,
    Find(bool),
    GoTo,
    Zoom(i32),
    ZoomReset,
    Wrap,
    Fullscreen,
    Settings,
    Format(Fmt),
    SetView(View),
    Markdown(bool),
}

enum Dialog {
    /// Closing a tab with unsaved changes.
    SavePrompt(u64),
    Settings,
    Changelog(Box<Preview>),
    GoTo {
        line: String,
        focus: bool,
    },
}

struct Find {
    query: Query,
    replace: String,
    replacing: bool,
    focus: bool,
    /// The matches and the hash of the text and query they belong to.
    cache: (u64, Vec<Range<usize>>),
}

pub struct App {
    tabs: Vec<Tab>,
    active: usize,
    next_id: u64,
    settings: Settings,
    actions: Vec<Action>,
    dialog: Option<Dialog>,
    find: Option<Find>,
    toasts: Vec<(String, Instant)>,
    inbox: Receiver<Vec<PathBuf>>,
    /// Events for the editor's text field (undo, redo), handed to it in the frame they are run.
    inject: Vec<Event>,
    focus_editor: bool,
    scroll_to_cursor: bool,
    session_due: Option<Instant>,
    title: String,
    fullscreen: bool,
    zoom_wheel: f32,
    frames: u32,
}

impl App {
    pub fn new(cc: &eframe::CreationContext, files: Vec<PathBuf>) -> Self {
        let ctx = &cc.egui_ctx;
        theme::install_fonts(ctx);
        theme::install_style(ctx);
        egui_extras::install_image_loaders(ctx);
        ctx.options_mut(|o| o.zoom_with_keyboard = false);

        let (send, inbox) = std::sync::mpsc::channel();
        let wake = ctx.clone();
        single_instance::listen(send, move || wake.request_repaint());

        let mut app = Self {
            tabs: Vec::new(),
            active: 0,
            next_id: 1,
            settings: session::load_settings(),
            actions: Vec::new(),
            dialog: None,
            find: None,
            toasts: Vec::new(),
            inbox,
            inject: Vec::new(),
            focus_editor: true,
            scroll_to_cursor: false,
            session_due: None,
            title: String::new(),
            fullscreen: false,
            zoom_wheel: 1.0,
            frames: 0,
        };
        let saved = session::load_session();
        for tab in saved.tabs {
            let tab = app.restore(tab);
            app.tabs.push(tab);
        }
        if app.tabs.is_empty() {
            let tab = app.blank();
            app.tabs.push(tab);
        }
        app.active = saved.active.min(app.tabs.len() - 1);
        for file in files {
            app.open_path(file);
        }
        app
    }

    fn t(&self) -> &'static Dict {
        self.settings.lang.dict()
    }

    fn tab(&self) -> &Tab {
        &self.tabs[self.active]
    }

    fn zoom(&self) -> f32 {
        self.settings.zoom as f32 / 100.0
    }

    fn blank(&mut self) -> Tab {
        self.next_id += 1;
        Tab::new(self.next_id)
    }

    fn restore(&mut self, saved: SavedTab) -> Tab {
        let mut tab = self.blank();
        tab.md = saved.md;
        tab.view = saved.view;
        tab.crlf = saved.crlf;
        tab.bom = saved.bom;
        match saved.path.as_deref().map(files::read) {
            // An unchanged file is read again, in case it changed on disk.
            Some(Ok(disk)) => {
                tab.saved = Some(disk.text.clone());
                if saved.dirty {
                    tab.set_text(saved.text);
                } else {
                    (tab.crlf, tab.bom) = (disk.crlf, disk.bom);
                    tab.set_text(disk.text);
                }
            }
            // A file that is gone keeps its text and counts as unsaved.
            Some(Err(_)) => {
                tab.saved = None;
                tab.set_text(saved.text);
            }
            None => tab.set_text(saved.text),
        }
        tab.path = saved.path;
        let cursor = saved.cursor.min(tab.chars);
        tab.cursor = cursor..cursor;
        tab.restore_cursor = true;
        tab
    }

    /// Schedules the session to be written, once typing pauses.
    fn touch(&mut self) {
        self.session_due = Some(Instant::now() + Duration::from_millis(800));
    }

    fn save_session(&mut self) {
        self.session_due = None;
        let session = Session {
            active: self.active,
            tabs: self
                .tabs
                .iter()
                .map(|t| SavedTab {
                    path: t.path.clone(),
                    text: t.text.clone(),
                    dirty: t.dirty(),
                    md: t.md,
                    view: t.view,
                    cursor: t.cursor.end,
                    crlf: t.crlf,
                    bom: t.bom,
                })
                .collect(),
        };
        if let Err(e) = session::save_session(&session) {
            self.toast(format!("session.json: {e}"));
        }
    }

    fn save_settings(&mut self) {
        if let Err(e) = session::save_settings(&self.settings) {
            self.toast(format!("settings.json: {e}"));
        }
    }

    fn toast(&mut self, message: String) {
        self.toasts.push((message, Instant::now()));
    }

    // ── Files ─────────────────────────────────────────────────────────

    fn open_path(&mut self, path: PathBuf) {
        if let Some(i) = self.tabs.iter().position(|t| {
            t.path
                .as_deref()
                .is_some_and(|p| files::same_path(p, &path))
        }) {
            self.active = i;
            self.focus_editor = true;
            return;
        }
        let name = path.file_name().map_or_else(
            || path.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        match files::read(&path) {
            Err(e) => self.toast(fill(self.t().open_failed, &[&name, &e])),
            Ok(disk) => {
                let mut tab = self.blank();
                tab.md = files::is_markdown(&path);
                tab.view = if tab.md { View::Preview } else { View::Split };
                (tab.crlf, tab.bom) = (disk.crlf, disk.bom);
                tab.saved = Some(disk.text.clone());
                tab.set_text(disk.text);
                tab.path = Some(path);
                // An empty, untouched tab makes room for the file, as in Notepad.
                let current = self.tab();
                if current.path.is_none() && current.text.is_empty() {
                    self.tabs[self.active] = tab;
                } else {
                    self.tabs.push(tab);
                    self.active = self.tabs.len() - 1;
                }
                self.focus_editor = true;
                self.touch();
            }
        }
    }

    fn open_dialog(&mut self) {
        let t = self.t();
        let mut dialog = rfd::FileDialog::new()
            .add_filter(t.doc_files, &["txt", "md", "markdown"])
            .add_filter(t.text_files, &["txt"])
            .add_filter(t.md_files, &["md", "markdown"])
            .add_filter(t.all_files, &["*"]);
        if let Some(dir) = self.tab().path.as_deref().and_then(|p| p.parent()) {
            dialog = dialog.set_directory(dir);
        }
        for path in dialog.pick_files().unwrap_or_default() {
            self.open_path(path);
        }
    }

    fn write(&mut self, i: usize, path: PathBuf) -> bool {
        let tab = &self.tabs[i];
        match std::fs::write(&path, files::encode(&tab.text, tab.crlf, tab.bom)) {
            Ok(()) => {
                let tab = &mut self.tabs[i];
                tab.saved = Some(tab.text.clone());
                tab.path = Some(path);
                self.touch();
                true
            }
            Err(e) => {
                let name = path.file_name().map_or_else(
                    || path.display().to_string(),
                    |n| n.to_string_lossy().into_owned(),
                );
                self.toast(fill(self.t().save_failed, &[&name, &e]));
                false
            }
        }
    }

    fn save(&mut self, i: usize) -> bool {
        match self.tabs[i].path.clone() {
            Some(path) => self.write(i, path),
            None => self.save_as(i),
        }
    }

    /// Suggests `.md` while Markdown is on and `.txt` otherwise.
    fn save_as(&mut self, i: usize) -> bool {
        let t = self.t();
        let tab = &self.tabs[i];
        let ext = if tab.md { "md" } else { "txt" };
        let name = match &tab.path {
            Some(path) => {
                let swap = path.extension().and_then(|e| e.to_str()).is_some_and(|e| {
                    ["txt", "md", "markdown"].contains(&e.to_ascii_lowercase().as_str())
                });
                let base = if swap {
                    path.with_extension(ext)
                } else {
                    path.clone()
                };
                base.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()
            }
            None => format!("{}.{ext}", files::safe_file_name(&tab.title(t.untitled))),
        };
        let text_filter: (&str, &[&str]) = (t.text_files, &["txt"]);
        let md_filter: (&str, &[&str]) = (t.md_files, &["md", "markdown"]);
        let filters = if tab.md {
            [md_filter, text_filter]
        } else {
            [text_filter, md_filter]
        };
        let mut dialog = rfd::FileDialog::new().set_file_name(name);
        for (label, exts) in filters {
            dialog = dialog.add_filter(label, exts);
        }
        dialog = dialog.add_filter(t.all_files, &["*"]);
        if let Some(dir) = tab.path.as_deref().and_then(|p| p.parent()) {
            dialog = dialog.set_directory(dir);
        }
        let Some(mut path) = dialog.save_file() else {
            return false;
        };
        if path.extension().is_none() {
            path.set_extension(ext);
        }
        self.write(i, path)
    }

    fn close_tab(&mut self, ctx: &Context, id: u64) {
        let Some(i) = self.tabs.iter().position(|t| t.id == id) else {
            return;
        };
        if self.tabs[i].dirty() {
            self.dialog = Some(Dialog::SavePrompt(id));
        } else {
            self.remove_tab(ctx, i);
        }
    }

    /// Closing the last tab closes the window, as in Notepad.
    fn remove_tab(&mut self, ctx: &Context, i: usize) {
        self.tabs.remove(i);
        if self.tabs.is_empty() {
            self.save_session();
            ctx.send_viewport_cmd(ViewportCommand::Close);
            let tab = self.blank();
            self.tabs.push(tab);
            return;
        }
        if self.active > i || self.active == self.tabs.len() {
            self.active -= 1;
        }
        self.focus_editor = true;
        self.touch();
    }

    fn open_link(&mut self, target: String) {
        let web = ["http://", "https://", "mailto:"]
            .iter()
            .any(|p| target.to_ascii_lowercase().starts_with(p));
        if web {
            if let Err(e) = open::that_detached(&target) {
                self.toast(fill(self.t().open_failed, &[&target, &e]));
            }
        } else if let Some(doc) = self.tab().path.clone() {
            self.open_path(files::resolve(&doc, &target));
        }
    }

    // ── Actions ───────────────────────────────────────────────────────

    fn run(&mut self, ctx: &Context, action: Action) {
        let t = self.t();
        match action {
            Action::NewTab => {
                let tab = self.blank();
                self.tabs.push(tab);
                self.active = self.tabs.len() - 1;
                self.focus_editor = true;
                self.touch();
            }
            Action::Open => self.open_dialog(),
            Action::Save => {
                self.save(self.active);
            }
            Action::SaveAs => {
                self.save_as(self.active);
            }
            Action::Print => {
                let tab = self.tab();
                if let Err(e) = print::print(
                    &tab.text,
                    tab.md,
                    &tab.title(t.untitled),
                    tab.path.as_deref(),
                ) {
                    self.toast(fill(t.print_failed, &[&e]));
                }
            }
            Action::Close(id) => self.close_tab(ctx, id),
            Action::Activate(i) => {
                self.active = i.min(self.tabs.len() - 1);
                self.focus_editor = true;
                self.touch();
            }
            Action::Undo | Action::Redo => {
                let key = if matches!(action, Action::Undo) {
                    Key::Z
                } else {
                    Key::Y
                };
                self.inject.push(Event::Key {
                    key,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: Modifiers::COMMAND,
                });
                self.focus_editor = true;
            }
            Action::Cut | Action::Copy | Action::Paste => {
                ctx.send_viewport_cmd(match action {
                    Action::Cut => ViewportCommand::RequestCut,
                    Action::Copy => ViewportCommand::RequestCopy,
                    _ => ViewportCommand::RequestPaste,
                });
                self.focus_editor = true;
            }
            Action::SelectAll => {
                let tab = self.tab();
                editor::set_selection(ctx, tab.editor_id(), 0..tab.chars);
                self.focus_editor = true;
            }
            Action::Find(replacing) => {
                let tab = &mut self.tabs[self.active];
                if tab.shown_view() == View::Preview {
                    tab.view = View::Split;
                }
                let selected = {
                    let (a, b) = (
                        char_to_byte(&tab.text, tab.cursor.start),
                        char_to_byte(&tab.text, tab.cursor.end),
                    );
                    tab.text[a..b].to_owned()
                };
                let find = self.find.get_or_insert_with(|| Find {
                    query: Query::default(),
                    replace: String::new(),
                    replacing,
                    focus: true,
                    cache: (0, Vec::new()),
                });
                find.replacing |= replacing;
                find.focus = true;
                if !selected.is_empty() && !selected.contains('\n') {
                    find.query.needle = selected;
                }
            }
            Action::GoTo => {
                self.dialog = Some(Dialog::GoTo {
                    line: String::new(),
                    focus: true,
                })
            }
            Action::Zoom(delta) => {
                self.settings.zoom = (self.settings.zoom as i32 + delta).clamp(10, 500) as u32;
                self.save_settings();
            }
            Action::ZoomReset => {
                self.settings.zoom = 100;
                self.save_settings();
            }
            Action::Wrap => {
                self.settings.wrap = !self.settings.wrap;
                self.save_settings();
            }
            Action::Fullscreen => {
                self.fullscreen = !self.fullscreen;
                ctx.send_viewport_cmd(ViewportCommand::Fullscreen(self.fullscreen));
            }
            Action::Settings => self.dialog = Some(Dialog::Settings),
            Action::Format(f) => self.format(ctx, f),
            Action::SetView(view) => {
                self.tabs[self.active].view = view;
                self.touch();
            }
            Action::Markdown(on) => {
                let tab = &mut self.tabs[self.active];
                tab.md = on;
                if on {
                    tab.view = View::Split;
                }
                self.touch();
            }
        }
    }

    fn format(&mut self, ctx: &Context, f: Fmt) {
        let t = self.t();
        let tab = &mut self.tabs[self.active];
        let (text, sel) = (&tab.text, tab.cursor.clone());
        let edit = match f {
            Fmt::Heading(level) => format::set_heading(text, sel, level),
            Fmt::List(kind) => format::toggle_list(text, sel, kind),
            Fmt::Wrap(marker) => format::toggle_wrap(text, sel, marker),
            Fmt::Link => format::insert_link(text, sel, t.link_text),
            Fmt::Table => format::insert_table(text, sel, |n| fill(t.column, &[&n])),
        };
        tab.set_text(edit.text);
        tab.cursor = edit.sel.clone();
        editor::set_selection(ctx, tab.editor_id(), edit.sel);
        self.focus_editor = true;
        self.scroll_to_cursor = true;
        self.touch();
    }

    fn shortcuts(&mut self, ctx: &Context) {
        let zoom = ctx.input(|i| i.zoom_delta());
        if zoom != 1.0 {
            // Ctrl+wheel: one step per notch, however finely the wheel or touchpad reports it.
            self.zoom_wheel *= zoom;
            if self.zoom_wheel > 1.08 || self.zoom_wheel < 0.92 {
                self.actions
                    .push(Action::Zoom(if self.zoom_wheel > 1.0 { 10 } else { -10 }));
                self.zoom_wheel = 1.0;
            }
        }
        if self.dialog.is_some() {
            return;
        }
        let c = Modifiers::COMMAND;
        let cs = Modifiers::COMMAND | Modifiers::SHIFT;
        let n = self.tabs.len();
        // The most specific first: consume_key ignores extra Shift.
        let keys = [
            (cs, Key::S, Action::SaveAs),
            (c, Key::S, Action::Save),
            (c, Key::N, Action::NewTab),
            (c, Key::T, Action::NewTab),
            (c, Key::O, Action::Open),
            (c, Key::P, Action::Print),
            (c, Key::W, Action::Close(self.tab().id)),
            (c, Key::F, Action::Find(false)),
            (c, Key::H, Action::Find(true)),
            (c, Key::G, Action::GoTo),
            (c, Key::Plus, Action::Zoom(10)),
            (c, Key::Equals, Action::Zoom(10)),
            (c, Key::Minus, Action::Zoom(-10)),
            (c, Key::Num0, Action::ZoomReset),
            (Modifiers::NONE, Key::F11, Action::Fullscreen),
            (cs, Key::Tab, Action::Activate((self.active + n - 1) % n)),
            (c, Key::Tab, Action::Activate((self.active + 1) % n)),
        ];
        for (mods, key, action) in keys {
            if ctx.input_mut(|i| i.consume_key(mods, key)) {
                self.actions.push(action);
            }
        }
        if self.find.is_some()
            && !Popup::is_any_open(ctx)
            && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))
        {
            self.find = None;
            self.focus_editor = true;
        }
    }

    fn receive(&mut self, ctx: &Context) {
        while let Ok(files) = self.inbox.try_recv() {
            for file in files {
                self.open_path(file);
            }
            ctx.send_viewport_cmd(ViewportCommand::Minimized(false));
            ctx.send_viewport_cmd(ViewportCommand::Focus);
        }
        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_path_buf())
                .collect()
        });
        for file in dropped {
            self.open_path(file);
        }
    }

    // ── Find ──────────────────────────────────────────────────────────

    fn matches(&mut self) -> Vec<Range<usize>> {
        let Some(find) = &mut self.find else {
            return Vec::new();
        };
        let text = &self.tabs[self.active].text;
        let key = {
            use std::hash::{Hash, Hasher};
            let mut h = std::hash::DefaultHasher::new();
            (
                text,
                &find.query.needle,
                find.query.match_case,
                find.query.whole_word,
            )
                .hash(&mut h);
            h.finish()
        };
        if find.cache.0 != key {
            find.cache = (key, search::find_all(text, &find.query));
        }
        find.cache.1.clone()
    }

    /// The match the selection is on.
    fn current_match(&self, matches: &[Range<usize>]) -> Option<usize> {
        let tab = self.tab();
        let sel =
            char_to_byte(&tab.text, tab.cursor.start)..char_to_byte(&tab.text, tab.cursor.end);
        matches.iter().position(|m| *m == sel)
    }

    fn find_step(&mut self, ctx: &Context, forward: bool) {
        let matches = self.matches();
        if matches.is_empty() {
            return;
        }
        let tab = &mut self.tabs[self.active];
        let (start, end) = (
            char_to_byte(&tab.text, tab.cursor.start),
            char_to_byte(&tab.text, tab.cursor.end),
        );
        let i = if forward {
            matches
                .iter()
                .position(|m| m.start >= end && m.start != start || m.start > start)
                .unwrap_or(0)
        } else {
            matches
                .iter()
                .rposition(|m| m.end <= start && m.start != start)
                .unwrap_or(matches.len() - 1)
        };
        let m = &matches[i];
        let sel = byte_to_char(&tab.text, m.start)..byte_to_char(&tab.text, m.end);
        tab.cursor = sel.clone();
        editor::set_selection(ctx, tab.editor_id(), sel);
        self.scroll_to_cursor = true;
    }

    fn replace_one(&mut self, ctx: &Context) {
        let matches = self.matches();
        let with = self
            .find
            .as_ref()
            .map(|f| f.replace.clone())
            .unwrap_or_default();
        if let Some(m) = self.current_match(&matches).map(|i| matches[i].clone()) {
            let tab = &mut self.tabs[self.active];
            let mut text = tab.text.clone();
            text.replace_range(m.clone(), &with);
            let after = byte_to_char(&text, m.start + with.len());
            tab.set_text(text);
            tab.cursor = after..after;
            editor::set_selection(ctx, tab.editor_id(), after..after);
            self.touch();
        }
        self.find_step(ctx, true);
    }

    fn replace_all(&mut self) {
        let Some(find) = &self.find else { return };
        let tab = &mut self.tabs[self.active];
        let (text, n) = search::replace_all(&tab.text, &find.query, &find.replace);
        if n > 0 {
            tab.set_text(text);
            let end = tab.cursor.end.min(tab.chars);
            tab.cursor = end..end;
            self.touch();
        }
    }

    fn find_bar(&mut self, ui: &mut Ui) {
        let t = self.t();
        let ctx = ui.ctx().clone();
        let matches = self.matches();
        let current = self.current_match(&matches);
        let Some(find) = &mut self.find else { return };
        let mut step = None;
        let mut replace = None;
        let mut close = false;
        let before = find.query.clone();
        Frame::new()
            .fill(theme::POPUP)
            .inner_margin(Margin {
                left: 12,
                right: 8,
                top: 8,
                bottom: 8,
            })
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
                // Wraps in a narrow pane instead of overlapping.
                ui.horizontal_wrapped(|ui| {
                    let chevron = if find.replacing {
                        &icons::CHEVRON
                    } else {
                        &icons::CHEVRON_RIGHT
                    };
                    if theme::icon_button(ui, chevron, t.replace_hint, true, false, false).clicked()
                    {
                        find.replacing = !find.replacing;
                    }
                    let field = theme::field(
                        ui,
                        TextEdit::singleline(&mut find.query.needle)
                            .hint_text(t.find_hint)
                            .desired_width(200.0),
                    );
                    if std::mem::take(&mut find.focus) {
                        field.request_focus();
                    }
                    if field.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                        step = Some(!ui.input(|i| i.modifiers.shift));
                        field.request_focus();
                    }
                    let count = match (find.query.needle.is_empty(), matches.len(), current) {
                        (true, ..) => String::new(),
                        (false, 0, _) => t.no_matches.to_owned(),
                        (false, n, Some(i)) => fill(t.matches, &[&(i + 1), &n]),
                        (false, n, None) => fill(t.matches, &[&"–", &n]),
                    };
                    ui.add_sized(
                        vec2(64.0, 20.0),
                        egui::Label::new(RichText::new(count).size(12.0).color(MUTED)),
                    );
                    if theme::icon_button(ui, &icons::ARROW_UP, t.previous, true, false, false)
                        .clicked()
                    {
                        step = Some(false);
                    }
                    if theme::icon_button(ui, &icons::ARROW_DOWN, t.next, true, false, false)
                        .clicked()
                    {
                        step = Some(true);
                    }
                    ui.add_space(6.0);
                    theme::switch(ui, &mut find.query.match_case, t.match_case);
                    ui.add_space(6.0);
                    theme::switch(ui, &mut find.query.whole_word, t.whole_word);
                    ui.add_space(6.0);
                    if theme::icon_button(ui, &icons::X, t.close, true, false, false).clicked() {
                        close = true;
                    }
                });
                if find.replacing {
                    ui.horizontal_wrapped(|ui| {
                        ui.add_space(38.0);
                        theme::field(
                            ui,
                            TextEdit::singleline(&mut find.replace)
                                .hint_text(t.replace_hint)
                                .desired_width(200.0),
                        );
                        if ui.button(t.replace_one).clicked() {
                            replace = Some(false);
                        }
                        if ui.button(t.replace_all).clicked() {
                            replace = Some(true);
                        }
                    });
                }
            });
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
        ui.painter().rect_filled(rect, 0.0, theme::hair());
        // Typing jumps to the first match from the cursor on.
        let typed = find.query != before && !find.query.needle.is_empty();
        if close {
            self.find = None;
            self.focus_editor = true;
        } else if let Some(all) = replace {
            if all {
                self.replace_all()
            } else {
                self.replace_one(&ctx)
            }
        } else if let Some(forward) = step {
            self.find_step(&ctx, forward);
        } else if typed {
            let tab = &mut self.tabs[self.active];
            tab.cursor = tab.cursor.start..tab.cursor.start;
            self.find_step(&ctx, true);
        }
    }

    // ── Layout ────────────────────────────────────────────────────────

    fn tab_strip(&mut self, ui: &mut Ui) {
        let t = self.t();
        let height = 36.0;
        let n = self.tabs.len() as f32;
        let room = ui.available_width() - 44.0;
        let width = ((room - 4.0 * (n - 1.0)) / n).clamp(120.0, 220.0);
        let mut active_rect = None;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ScrollArea::horizontal()
                .max_width(room)
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for (i, tab) in self.tabs.iter().enumerate() {
                            let (rect, response) =
                                ui.allocate_exact_size(vec2(width, height), Sense::click());
                            let close_rect = Rect::from_center_size(
                                pos2(rect.right() - 18.0, rect.center().y),
                                vec2(24.0, 24.0),
                            );
                            let close = ui.interact(
                                close_rect,
                                ui.scope_id().with(("close", tab.id)),
                                Sense::click(),
                            );
                            let active = i == self.active;
                            let hovered = response.hovered() || close.hovered();
                            let painter = ui.painter();
                            let top = CornerRadius {
                                nw: 10,
                                ne: 10,
                                sw: 0,
                                se: 0,
                            };
                            if active {
                                active_rect = Some(rect);
                                painter.add(theme::glow(LINE, 10, 0.35).as_shape(rect, top));
                                painter.rect_filled(rect, top, theme::BG);
                                painter.rect_filled(rect, top, fade(LINE, 0.07));
                                painter.rect_filled(
                                    Rect::from_min_size(
                                        rect.min,
                                        vec2(rect.width(), rect.height() / 2.0),
                                    ),
                                    top,
                                    fade(LINE, 0.06),
                                );
                                // The bottom edge sits below the strip, so the tab opens into the page.
                                painter.rect_stroke(
                                    rect.with_max_y(rect.bottom() + 12.0),
                                    top,
                                    Stroke::new(1.0, tint(LINE, 0.6)),
                                    egui::StrokeKind::Inside,
                                );
                            } else if hovered {
                                painter.rect_filled(rect, top, fade(LINE, 0.08));
                            }
                            let color = if active {
                                tint(LINE, 0.25)
                            } else if hovered {
                                TEXT
                            } else {
                                MUTED
                            };
                            let mut job = LayoutJob::simple_singleline(
                                tab.title(t.untitled),
                                FontId::proportional(14.0),
                                color,
                            );
                            job.wrap = TextWrapping {
                                max_width: rect.width() - 14.0 - 36.0,
                                max_rows: 1,
                                break_anywhere: true,
                                overflow_character: Some('…'),
                            };
                            let galley = painter.layout_job(job);
                            let pos =
                                pos2(rect.left() + 14.0, rect.center().y - galley.size().y / 2.0);
                            if active {
                                theme::paint_glow_text(painter, pos, &galley, LINE, color);
                            } else {
                                painter.galley(pos, galley, color);
                            }
                            // Unsaved: a blue dot where the × is, which comes back on hover.
                            if tab.dirty() && !hovered {
                                painter.circle_filled(close_rect.center(), 6.0, fade(LINE, 0.25));
                                painter.circle_filled(close_rect.center(), 4.0, LINE);
                            } else {
                                if close.hovered() {
                                    painter.rect_filled(close_rect, 6.0, fade(LINE, 0.18));
                                    ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                                }
                                let x = if close.hovered() { TEXT } else { MUTED };
                                icons::paint(
                                    ui,
                                    Rect::from_center_size(close_rect.center(), vec2(14.0, 14.0)),
                                    &icons::X,
                                    x,
                                );
                            }
                            if close.clicked() || response.middle_clicked() {
                                self.actions.push(Action::Close(tab.id));
                            } else if response.clicked() {
                                self.actions.push(Action::Activate(i));
                            }
                            if active && self.frames < 3 {
                                response.scroll_to_me(None);
                            }
                        }
                    });
                });
            let (rect, response) = ui.allocate_exact_size(vec2(36.0, height), Sense::click());
            if response.hovered() {
                ui.painter().rect_filled(rect, 8.0, fade(LINE, 0.14));
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }
            icons::paint(
                ui,
                Rect::from_center_size(rect.center(), vec2(18.0, 18.0)),
                &icons::PLUS,
                LINE,
            );
            if response.on_hover_text(t.new_tab).clicked() {
                self.actions.push(Action::NewTab);
            }
        });
        // The strip's bottom line, open under the active tab.
        let full = ui.max_rect();
        let y = full.bottom() - 0.5;
        let painter = ui.painter();
        match active_rect.map(|r| r.intersect(ui.clip_rect())) {
            Some(r) if r.width() > 0.0 => {
                painter.hline(full.left()..=r.left(), y, Stroke::new(1.0, theme::hair()));
                painter.hline(r.right()..=full.right(), y, Stroke::new(1.0, theme::hair()));
            }
            _ => {
                painter.hline(full.x_range(), y, Stroke::new(1.0, theme::hair()));
            }
        }
    }

    fn menubar(&mut self, ui: &mut Ui) {
        let t = self.t();
        let (md, view) = (self.tab().md, self.tab().shown_view());
        let mut actions = Vec::new();
        let item = |ui: &mut Ui,
                    actions: &mut Vec<Action>,
                    label: &str,
                    keys: &str,
                    action: Action| {
            if ui
                .add(Button::new(label).shortcut_text(RichText::new(keys).size(12.0).color(MUTED)))
                .clicked()
            {
                actions.push(action);
            }
        };
        ui.horizontal_centered(|ui| {
            MenuBar::new().ui(ui, |ui| {
                MenuButton::new(t.file).ui(ui, |ui| {
                    ui.set_min_width(250.0);
                    item(ui, &mut actions, t.new_tab, "Ctrl+N", Action::NewTab);
                    item(ui, &mut actions, t.open, "Ctrl+O", Action::Open);
                    item(ui, &mut actions, t.save, "Ctrl+S", Action::Save);
                    item(ui, &mut actions, t.save_as, "Ctrl+Shift+S", Action::SaveAs);
                    ui.separator();
                    item(ui, &mut actions, t.print, "Ctrl+P", Action::Print);
                    ui.separator();
                    item(
                        ui,
                        &mut actions,
                        t.close_tab,
                        "Ctrl+W",
                        Action::Close(self.tabs[self.active].id),
                    );
                });
                MenuButton::new(t.edit).ui(ui, |ui| {
                    ui.set_min_width(250.0);
                    item(ui, &mut actions, t.undo, "Ctrl+Z", Action::Undo);
                    item(ui, &mut actions, t.redo, "Ctrl+Y", Action::Redo);
                    ui.separator();
                    item(ui, &mut actions, t.cut, "Ctrl+X", Action::Cut);
                    item(ui, &mut actions, t.copy, "Ctrl+C", Action::Copy);
                    item(ui, &mut actions, t.paste, "Ctrl+V", Action::Paste);
                    ui.separator();
                    item(ui, &mut actions, t.find, "Ctrl+F", Action::Find(false));
                    item(ui, &mut actions, t.replace, "Ctrl+H", Action::Find(true));
                    item(ui, &mut actions, t.go_to, "Ctrl+G", Action::GoTo);
                    ui.separator();
                    item(ui, &mut actions, t.select_all, "Ctrl+A", Action::SelectAll);
                });
                MenuButton::new(t.view).ui(ui, |ui| {
                    ui.set_min_width(250.0);
                    item(ui, &mut actions, t.zoom_in, "Ctrl+Plus", Action::Zoom(10));
                    item(
                        ui,
                        &mut actions,
                        t.zoom_out,
                        "Ctrl+Minus",
                        Action::Zoom(-10),
                    );
                    item(ui, &mut actions, t.zoom_reset, "Ctrl+0", Action::ZoomReset);
                    ui.separator();
                    let mut wrap = self.settings.wrap;
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.add_space(8.0);
                        if theme::switch(ui, &mut wrap, t.word_wrap).changed() {
                            actions.push(Action::Wrap);
                        }
                    });
                    ui.add_space(4.0);
                    item(ui, &mut actions, t.fullscreen, "F11", Action::Fullscreen);
                });
                // The menu bar takes the whole row, so the rest of the row goes in it.
                if md {
                    Self::format_bar(t, ui, view != View::Preview, &mut actions);
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if theme::icon_button(ui, &icons::GEAR, t.settings, true, false, false)
                        .clicked()
                    {
                        actions.push(Action::Settings);
                    }
                    if md {
                        ui.add_space(8.0);
                        // The switch is one widget, so it reads left to right in this right-to-left row.
                        let views = [View::Editor, View::Split, View::Preview];
                        let labels = [t.view_editor, t.view_split, t.view_preview];
                        let selected = views.iter().position(|v| *v == view).unwrap_or(1);
                        if let Some(i) = theme::pills(ui, &labels, selected) {
                            actions.push(Action::SetView(views[i]));
                        }
                    }
                });
            });
        });
        self.actions.extend(actions);
    }

    fn format_bar(t: &Dict, ui: &mut Ui, enabled: bool, actions: &mut Vec<Action>) {
        ui.add_space(14.0);
        let (rect, _) = ui.allocate_exact_size(vec2(1.0, 28.0), Sense::hover());
        ui.painter().rect_filled(rect, 0.0, theme::hair());
        ui.add_space(10.0);
        ui.spacing_mut().item_spacing.x = 2.0;
        let heading = theme::icon_button(ui, &icons::HEADING, t.heading, enabled, true, false);
        Popup::menu(&heading).show(|ui| {
            ui.set_min_width(190.0);
            for level in 1..=6 {
                if ui.button(fill(t.heading_n, &[&level])).clicked() {
                    actions.push(Action::Format(Fmt::Heading(level)));
                }
            }
            ui.separator();
            if ui.button(t.normal_text).clicked() {
                actions.push(Action::Format(Fmt::Heading(0)));
            }
        });
        let list = theme::icon_button(ui, &icons::LIST, t.list, enabled, true, false);
        Popup::menu(&list).show(|ui| {
            ui.set_min_width(190.0);
            for (label, kind) in [
                (t.bulleted, ListKind::Bullet),
                (t.numbered, ListKind::Numbered),
                (t.checklist, ListKind::Task),
            ] {
                if ui.button(label).clicked() {
                    actions.push(Action::Format(Fmt::List(kind)));
                }
            }
        });
        let buttons = [
            (
                &icons::BOLD,
                format!("{} (Ctrl+B)", t.bold),
                Fmt::Wrap("**"),
            ),
            (
                &icons::ITALIC,
                format!("{} (Ctrl+I)", t.italic),
                Fmt::Wrap("*"),
            ),
            (&icons::STRIKE, t.strike.to_owned(), Fmt::Wrap("~~")),
            (&icons::LINK, format!("{} (Ctrl+K)", t.link), Fmt::Link),
            (&icons::TABLE, t.table.to_owned(), Fmt::Table),
        ];
        for (icon, tip, f) in buttons {
            if theme::icon_button(ui, icon, &tip, enabled, false, false).clicked() {
                actions.push(Action::Format(f));
            }
        }
    }

    fn status_bar(&mut self, ui: &mut Ui) {
        let t = self.t();
        let tab = self.tab();
        let small = |s: String| {
            RichText::new(s)
                .size(12.0)
                .color(MUTED)
                .extra_letter_spacing(0.5)
        };
        let (line, col) = line_col(&tab.text, tab.cursor.end);
        let selected = tab.cursor.len();
        let chars = if selected > 0 {
            fill(t.chars_selected, &[&selected, &tab.chars])
        } else {
            fill(t.chars, &[&tab.chars])
        };
        let mut md = tab.md;
        let zoom = format!("{}%", self.settings.zoom);
        ui.horizontal_centered(|ui| {
            ui.spacing_mut().item_spacing.x = 14.0;
            ui.add_space(4.0);
            ui.label(small(fill(t.position, &[&line, &col])));
            ui.separator();
            ui.label(small(chars));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(4.0);
                if theme::switch(ui, &mut md, t.markdown).changed() {
                    self.actions.push(Action::Markdown(md));
                }
                ui.separator();
                ui.label(small(zoom));
                ui.separator();
            });
        });
    }

    fn workspace(&mut self, ui: &mut Ui) {
        let rect = ui.max_rect();
        match self.tab().shown_view() {
            View::Editor => self.editor_pane(ui, rect),
            View::Preview => self.preview_pane(ui, rect),
            View::Split => {
                let x = (rect.left() + rect.width() * self.settings.split).round();
                // The preview first: the editor's scroll-to-cursor would otherwise scroll it too.
                self.preview_pane(ui, rect.with_min_x(x + 1.0));
                self.editor_pane(ui, rect.with_max_x(x));
                // The divider, on top of both panes: drag it to share the width differently.
                let handle =
                    Rect::from_center_size(pos2(x, rect.center().y), vec2(9.0, rect.height()));
                let response = ui.interact(handle, Id::unique("split"), Sense::drag());
                if let (true, Some(p)) = (response.dragged(), response.interact_pointer_pos()) {
                    self.settings.split = ((p.x - rect.left()) / rect.width()).clamp(0.15, 0.85);
                }
                if response.drag_stopped() {
                    self.save_settings();
                }
                let lit = response.hovered() || response.dragged();
                if lit {
                    ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
                    ui.painter().rect_filled(
                        Rect::from_center_size(handle.center(), vec2(3.0, rect.height())),
                        0.0,
                        fade(LINE, 0.25),
                    );
                }
                ui.painter().vline(
                    x + 0.5,
                    rect.y_range(),
                    Stroke::new(1.0, if lit { LINE } else { theme::hair() }),
                );
            }
        }
    }

    fn editor_pane(&mut self, ui: &mut Ui, rect: Rect) {
        let mut ui = ui.new_child(UiBuilder::new().max_rect(rect));
        ui.set_clip_rect(rect.intersect(ui.clip_rect()));
        ui.painter().rect_filled(rect, 0.0, theme::EDITOR_BG);
        if self.find.is_some() {
            self.find_bar(&mut ui);
        }
        self.editor(&mut ui);
    }

    fn editor(&mut self, ui: &mut Ui) {
        let t = self.t();
        let ctx = ui.ctx().clone();
        let size = 15.0 * self.zoom();
        let wrap = self.settings.wrap;
        let matches = self.matches();
        let current = self.current_match(&matches);
        let i = self.active;
        let id = self.tabs[i].editor_id();
        if self.tabs[i].restore_cursor {
            self.tabs[i].restore_cursor = false;
            editor::set_selection(&ctx, id, self.tabs[i].cursor.clone());
            self.scroll_to_cursor = true;
        }
        if std::mem::take(&mut self.focus_editor) {
            ctx.memory_mut(|m| m.request_focus(id));
        }
        let inject = std::mem::take(&mut self.inject);
        if !inject.is_empty() {
            ctx.input_mut(|input| input.events.extend(inject));
        }
        let mut changed = false;
        if let Some(edit) = editor::keys(ui, id, &self.tabs[i].text, self.tabs[i].md, t.link_text) {
            self.tabs[i].set_text(edit.text);
            editor::set_selection(&ctx, id, edit.sel);
            changed = true;
            self.scroll_to_cursor = true;
        }
        let pane = ui.available_size();
        let scroll = if wrap {
            ScrollArea::vertical()
        } else {
            ScrollArea::both()
        };
        let Tab {
            text,
            hl,
            md,
            cursor,
            id: tab_id,
            ..
        } = &mut self.tabs[i];
        let md = *md;
        let mut scroll_to_cursor = std::mem::take(&mut self.scroll_to_cursor);
        scroll
            .id_salt(("editor", *tab_id))
            .auto_shrink(false)
            .show(ui, |ui| {
                let mut layouter = |ui: &Ui, buffer: &dyn egui::TextBuffer, width: f32| {
                    hl.layout(
                        ui,
                        buffer.as_str(),
                        if wrap { width } else { f32::INFINITY },
                        size,
                        md,
                        &matches,
                        current,
                    )
                };
                let output = TextEdit::multiline(text)
                    .id(id)
                    .font(FontId::monospace(size))
                    .frame(Frame::NONE.inner_margin(Margin {
                        left: 20,
                        right: 20,
                        top: 14,
                        bottom: 0,
                    }))
                    .desired_width(f32::INFINITY)
                    .min_size(vec2(pane.x, pane.y))
                    .lock_focus(true)
                    .layouter(&mut layouter)
                    .show(ui);
                changed |= output.response.changed();
                if let Some(range) = output.cursor_range {
                    let [a, b] = range.sorted_cursors();
                    *cursor = a.index.into()..b.index.into();
                }
                hl.paint_strikes(ui, &output.galley, output.galley_pos);
                if std::mem::take(&mut scroll_to_cursor) {
                    let at = output
                        .galley
                        .pos_from_cursor(CCursor::new(cursor.end))
                        .translate(output.galley_pos.to_vec2());
                    ui.scroll_to_rect(at.expand2(vec2(size, size * 2.0)), None);
                }
                // Room to scroll the last line up to the middle.
                ui.add_space(pane.y * 0.4);
            });
        if changed {
            let tab = &mut self.tabs[i];
            tab.chars = tab.text.chars().count();
            self.touch();
        }
    }

    fn preview_pane(&mut self, ui: &mut Ui, rect: Rect) {
        let mut ui = ui.new_child(UiBuilder::new().max_rect(rect));
        ui.set_clip_rect(rect.intersect(ui.clip_rect()));
        let zoom = self.zoom();
        let Tab {
            preview,
            text,
            path,
            id,
            ..
        } = &mut self.tabs[self.active];
        let clicked = preview.show(&mut ui, text, path.as_deref(), zoom, ("preview", *id));
        if let Some(target) = clicked {
            self.open_link(target);
        }
    }

    // ── Dialogs ───────────────────────────────────────────────────────

    fn heading(ui: &mut Ui, text: &str) {
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new(text)
                    .font(FontId::new(22.0, family(theme::NEON)))
                    .color(TEXT),
            )
        });
    }

    fn dialogs(&mut self, ctx: &Context) {
        let Some(dialog) = self.dialog.take() else {
            return;
        };
        let t = self.t();
        let modal = |id: &str| {
            Modal::new(Id::unique(id))
                .frame(theme::neon_box(LINE))
                .backdrop_color(Color32::from_rgba_unmultiplied(1, 1, 6, 184))
        };
        self.dialog = match dialog {
            Dialog::SavePrompt(id) => {
                let Some(i) = self.tabs.iter().position(|tab| tab.id == id) else {
                    return;
                };
                let name = self.tabs[i].title(t.untitled);
                let mut choice = None;
                let response = modal("save-prompt").show(ctx, |ui| {
                    ui.set_width(380.0);
                    ui.spacing_mut().item_spacing.y = 14.0;
                    Self::heading(ui, &fill(t.save_prompt, &[&name]));
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new(t.save_prompt_hint).size(13.0).color(MUTED))
                    });
                    choice = theme::button_row(
                        ui,
                        &[
                            (t.save, ButtonKind::Primary),
                            (t.dont_save, ButtonKind::Danger),
                            (t.cancel, ButtonKind::Normal),
                        ],
                    );
                });
                match choice {
                    Some(0) => {
                        if self.save(i) {
                            self.remove_tab(ctx, i);
                        }
                        None
                    }
                    Some(1) => {
                        self.remove_tab(ctx, i);
                        None
                    }
                    Some(_) => None,
                    None if response.should_close() => None,
                    None => Some(Dialog::SavePrompt(id)),
                }
            }
            Dialog::Settings => {
                let mut lang = self.settings.lang;
                let mut wrap = self.settings.wrap;
                let mut changelog = false;
                let mut done = false;
                let response = modal("settings").show(ctx, |ui| {
                    ui.set_width(340.0);
                    ui.spacing_mut().item_spacing.y = 14.0;
                    ui.vertical_centered(|ui| theme::neon_text(ui, "DYSTOPAD", 30.0, theme::LOGO));
                    ui.horizontal(|ui| {
                        ui.label(t.language);
                        ComboBox::from_id_salt("language")
                            .selected_text(lang.name())
                            .width(160.0)
                            .show_ui(ui, |ui| {
                                for l in Lang::ALL {
                                    ui.selectable_value(&mut lang, l, l.name());
                                }
                            });
                    });
                    theme::switch(ui, &mut wrap, t.word_wrap);
                    ui.vertical_centered(|ui| {
                        ui.horizontal(|ui| {
                            let version =
                                RichText::new(format!("{} · ", fill(t.version, &[&VERSION])))
                                    .size(13.0)
                                    .color(MUTED);
                            let link = RichText::new(t.changelog)
                                .size(13.0)
                                .color(tint(LINE, 0.5))
                                .underline();
                            let width = ui
                                .painter()
                                .layout_no_wrap(
                                    format!("{} · {}", fill(t.version, &[&VERSION]), t.changelog),
                                    FontId::proportional(13.0),
                                    MUTED,
                                )
                                .size()
                                .x;
                            ui.add_space(((ui.available_width() - width) / 2.0).max(0.0));
                            ui.spacing_mut().item_spacing.x = 0.0;
                            ui.label(version);
                            if ui
                                .add(egui::Label::new(link).sense(Sense::click()))
                                .on_hover_cursor(CursorIcon::PointingHand)
                                .clicked()
                            {
                                changelog = true;
                            }
                        });
                    });
                    done = theme::button_row(ui, &[(t.done, ButtonKind::Primary)]).is_some();
                });
                if lang != self.settings.lang || wrap != self.settings.wrap {
                    (self.settings.lang, self.settings.wrap) = (lang, wrap);
                    self.save_settings();
                }
                if changelog {
                    Some(Dialog::Changelog(Box::default()))
                } else if done || response.should_close() {
                    self.focus_editor = true;
                    None
                } else {
                    Some(Dialog::Settings)
                }
            }
            Dialog::Changelog(mut preview) => {
                let screen = ctx.content_rect();
                let mut close = false;
                let response = modal("changelog").show(ctx, |ui| {
                    ui.set_width((screen.width() * 0.9).min(720.0));
                    ui.spacing_mut().item_spacing.y = 14.0;
                    Self::heading(ui, t.changelog);
                    let (rect, _) = ui.allocate_exact_size(
                        vec2(ui.available_width(), (screen.height() * 0.62).min(560.0)),
                        Sense::hover(),
                    );
                    let mut inner = ui.new_child(UiBuilder::new().max_rect(rect));
                    inner.set_clip_rect(rect);
                    let _ = preview.show(&mut inner, CHANGELOG, None, 0.875, "changelog");
                    close = theme::button_row(ui, &[(t.close, ButtonKind::Primary)]).is_some();
                });
                if close || response.should_close() {
                    self.focus_editor = true;
                    None
                } else {
                    Some(Dialog::Changelog(preview))
                }
            }
            Dialog::GoTo { mut line, focus } => {
                let mut go = false;
                let mut cancel = false;
                let response = modal("go-to").show(ctx, |ui| {
                    ui.set_width(300.0);
                    ui.spacing_mut().item_spacing.y = 14.0;
                    Self::heading(ui, t.go_to.trim_end_matches('…'));
                    ui.vertical_centered(|ui| {
                        let field = theme::field(
                            ui,
                            TextEdit::singleline(&mut line)
                                .hint_text(t.line_number)
                                .desired_width(200.0),
                        );
                        if focus {
                            field.request_focus();
                        }
                        go = field.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
                    });
                    match theme::button_row(
                        ui,
                        &[(t.go, ButtonKind::Primary), (t.cancel, ButtonKind::Normal)],
                    ) {
                        Some(0) => go = true,
                        Some(_) => cancel = true,
                        None => {}
                    }
                });
                if go {
                    if let Ok(n) = line.trim().parse::<usize>() {
                        let tab = &mut self.tabs[self.active];
                        let start: usize = tab
                            .text
                            .split('\n')
                            .take(n.max(1) - 1)
                            .map(|l| l.chars().count() + 1)
                            .sum();
                        let start = start.min(tab.chars);
                        tab.cursor = start..start;
                        editor::set_selection(ctx, tab.editor_id(), start..start);
                        self.scroll_to_cursor = true;
                    }
                    self.focus_editor = true;
                    None
                } else if cancel || response.should_close() {
                    self.focus_editor = true;
                    None
                } else {
                    Some(Dialog::GoTo { line, focus: false })
                }
            }
        };
    }

    fn toasts(&mut self, ctx: &Context) {
        self.toasts
            .retain(|(_, at)| at.elapsed() < Duration::from_secs(6));
        if self.toasts.is_empty() {
            return;
        }
        Area::new(Id::unique("toasts"))
            .anchor(Align2::CENTER_BOTTOM, vec2(0.0, -48.0))
            .order(Order::Tooltip)
            .interactable(false)
            .show(ctx, |ui| {
                for (message, _) in &self.toasts {
                    Frame::new()
                        .fill(Color32::from_rgb(0x0b, 0x07, 0x10))
                        .stroke(Stroke::new(1.0, theme::DANGER))
                        .corner_radius(10)
                        .inner_margin(Margin::symmetric(16, 8))
                        .shadow(theme::glow(theme::DANGER, 14, 0.5))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(message).color(Color32::from_rgb(0xff, 0xd5, 0xdd)),
                            )
                        });
                }
            });
        ctx.request_repaint_after(Duration::from_secs(1));
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.frames = self.frames.saturating_add(1);
        if self.frames < 5 {
            // Start maximized on screens where the default size would not fit.
            if let Some(screen) = ctx.input(|i| i.viewport().monitor_size) {
                if screen.x < 1480.0 || screen.y < 960.0 {
                    ctx.send_viewport_cmd(ViewportCommand::Maximized(true));
                }
                self.frames = 5;
            }
        }
        self.receive(&ctx);
        self.shortcuts(&ctx);
        for action in std::mem::take(&mut self.actions) {
            self.run(&ctx, action);
        }
        if ctx.input(|i| i.viewport().close_requested()) {
            self.save_session();
        }

        let bar = |top: i8| {
            Frame::new().fill(theme::BG).inner_margin(Margin {
                left: 10,
                right: 10,
                top,
                bottom: 0,
            })
        };
        Panel::top("tabs")
            .frame(bar(8))
            .resizable(false)
            .show_separator_line(false)
            .show(ui, |ui| self.tab_strip(ui));
        Panel::top("menu")
            .frame(bar(0))
            .resizable(false)
            .exact_size(46.0)
            .show(ui, |ui| self.menubar(ui));
        Panel::bottom("status")
            .frame(bar(0))
            .resizable(false)
            .exact_size(32.0)
            .show(ui, |ui| self.status_bar(ui));
        CentralPanel::no_frame().show(ui, |ui| self.workspace(ui));
        self.dialogs(&ctx);
        self.toasts(&ctx);

        let title = format!("{} - DystoPad", self.tab().title(self.t().untitled));
        if title != self.title {
            ctx.send_viewport_cmd(ViewportCommand::Title(title.clone()));
            self.title = title;
        }
        if let Some(due) = self.session_due {
            let now = Instant::now();
            if now >= due {
                self.save_session();
            } else {
                ctx.request_repaint_after(due - now);
            }
        }
        if !self.actions.is_empty() {
            ctx.request_repaint();
        }
    }

    fn on_exit(&mut self) {
        self.save_session();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsaved_tabs_are_named_after_their_first_words() {
        assert_eq!(
            first_words("\n\n  12 Okay   some insight\nmore").as_deref(),
            Some("12 Okay some insight")
        );
        assert_eq!(
            first_words("# Weekend plans\n- a").as_deref(),
            Some("Weekend plans")
        );
        assert_eq!(first_words("- [ ] buy milk").as_deref(), Some("buy milk"));
        assert_eq!(first_words("  \n\n"), None);
        assert_eq!(
            first_words(&"long ".repeat(20)).unwrap().chars().count(),
            40
        );
    }

    #[test]
    fn positions() {
        assert_eq!(line_col("ab\ncd", 4), (2, 2));
        assert_eq!(line_col("äö", 2), (1, 3));
        assert_eq!(char_to_byte("äb", 1), 2);
        assert_eq!(byte_to_char("äb", 2), 1);
    }
}
