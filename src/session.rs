//! What survives a restart: the open tabs (unsaved text included) and the settings.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::i18n::Lang;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "lowercase")]
pub enum View {
    Editor,
    #[default]
    Split,
    Preview,
}

/// One tab, in the same format as Dystonotepad's `session.json`, so its tabs can be taken over.
#[derive(Serialize, Deserialize, Default, Debug, PartialEq)]
#[serde(default)]
pub struct SavedTab {
    pub path: Option<PathBuf>,
    pub text: String,
    pub dirty: bool,
    pub md: bool,
    pub view: View,
    /// In chars.
    pub cursor: usize,
    pub crlf: bool,
    pub bom: bool,
}

#[derive(Serialize, Deserialize, Default, Debug, PartialEq)]
#[serde(default)]
pub struct Session {
    pub active: usize,
    pub tabs: Vec<SavedTab>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub lang: Lang,
    /// Percent, 10 to 500.
    pub zoom: u32,
    pub wrap: bool,
    /// The editor's share of the width in split view.
    pub split: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            lang: Lang::En,
            zoom: 100,
            wrap: true,
            split: 0.5,
        }
    }
}

/// `DYSTOPAD_DATA_DIR` moves it, so test runs leave the real session alone.
fn dir() -> Option<PathBuf> {
    std::env::var_os("DYSTOPAD_DATA_DIR")
        .map(PathBuf::from)
        .or_else(|| dirs::data_dir().map(|d| d.join("DystoPad")))
}

fn read<T: serde::de::DeserializeOwned>(path: Option<PathBuf>) -> Option<T> {
    serde_json::from_str(&std::fs::read_to_string(path?).ok()?).ok()
}

/// Writes a copy first, so a crash halfway through can't destroy the last good file.
fn write(name: &str, value: &impl Serialize) -> std::io::Result<()> {
    let dir = dir().ok_or_else(|| std::io::Error::other("no data folder"))?;
    std::fs::create_dir_all(&dir)?;
    let tmp = dir.join(format!("{name}.tmp"));
    std::fs::write(&tmp, serde_json::to_vec(value)?)?;
    std::fs::rename(tmp, dir.join(name))
}

pub fn load_session() -> Session {
    read(dir().map(|d| d.join("session.json")))
        // The first start takes over Dystonotepad's tabs, so nothing unsaved there gets lost.
        .or_else(|| {
            let legacy =
                dirs::data_dir().filter(|_| std::env::var_os("DYSTOPAD_DATA_DIR").is_none());
            read(legacy.map(|d| {
                d.join("com.yodelinggoethe.dystonotepad")
                    .join("session.json")
            }))
        })
        .unwrap_or_default()
}

pub fn save_session(session: &Session) -> std::io::Result<()> {
    write("session.json", session)
}

pub fn load_settings() -> Settings {
    read(dir().map(|d| d.join("settings.json"))).unwrap_or_default()
}

pub fn save_settings(settings: &Settings) -> std::io::Result<()> {
    write("settings.json", settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_dystonotepad_session() {
        let json = r#"{"active":1,"tabs":[
            {"path":"C:\\notes\\a.md","text":"x","dirty":false,"md":true,"view":"preview","cursor":0,"crlf":true,"bom":false},
            {"path":null,"text":"- one","dirty":true,"md":true,"view":"split","cursor":5,"crlf":true,"bom":false}]}"#;
        let s: Session = serde_json::from_str(json).unwrap();
        assert_eq!(s.active, 1);
        assert_eq!(s.tabs[0].view, View::Preview);
        assert_eq!(s.tabs[1].path, None);
        assert_eq!(s.tabs[1].text, "- one");
    }

    #[test]
    fn missing_fields_get_defaults() {
        let s: Settings = serde_json::from_str(r#"{"zoom":120}"#).unwrap();
        assert_eq!(
            s,
            Settings {
                zoom: 120,
                ..Settings::default()
            }
        );
    }
}
