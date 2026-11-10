//! Global application settings (`%APPDATA%\RenPyStudio\settings.json`).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct RecentProject {
    pub path: String,
    pub title: String,
    pub opened_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct AiSettings {
    pub base_url: String,
    /// Plain key, kept in memory only. It is never serialised — neither to disk
    /// nor to the webview; `api_key_enc` is what actually travels.
    #[serde(default, skip_serializing)]
    pub api_key: String,
    /// DPAPI-sealed key (hex). Windows keeps the key readable only by this user.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key_enc: Option<String>,
    /// Plain fallback when the OS has no DPAPI (and for projects created before
    /// the key was sealed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key_plain: Option<String>,
    /// Whether a key is stored, so the UI can show "saved" without seeing it.
    #[serde(default)]
    pub has_key: bool,
    pub model: String,
}

impl Default for AiSettings {
    fn default() -> Self {
        AiSettings {
            base_url: "https://api.openai.com/v1".into(),
            api_key: String::new(),
            api_key_enc: None,
            api_key_plain: None,
            has_key: false,
            model: String::new(),
        }
    }
}

impl AiSettings {
    /// Brings the plain key back into memory after reading the file.
    pub fn unlock(&mut self) {
        if self.api_key.is_empty() {
            if let Some(enc) = self.api_key_enc.as_deref() {
                self.api_key = crate::secret::unprotect(enc).unwrap_or_default();
            }
            if self.api_key.is_empty() {
                if let Some(plain) = self.api_key_plain.as_deref() {
                    self.api_key = plain.to_string();
                }
            }
        }
        self.has_key = !self.api_key.is_empty();
    }

    /// Prepares a writable copy: seals the key and drops every plain copy.
    pub fn seal(&mut self) {
        self.has_key = !self.api_key.is_empty();
        if self.api_key.is_empty() {
            self.api_key_enc = None;
            self.api_key_plain = None;
            return;
        }
        match crate::secret::protect(&self.api_key) {
            Some(enc) => {
                self.api_key_enc = Some(enc);
                self.api_key_plain = None;
            }
            None => {
                self.api_key_enc = None;
                self.api_key_plain = Some(self.api_key.clone());
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub locale: String,
    pub theme: String,
    pub accent: String,
    pub sdk_path: Option<String>,
    pub autosave_seconds: u32,
    pub mcp_enabled: bool,
    pub mcp_port: u16,
    pub mcp_require_confirm: bool,
    pub graph_detail: String,
    pub last_project: Option<String>,
    pub recent_projects: Vec<RecentProject>,
    pub ai: AiSettings,
    pub editor_font_size: u32,
    pub show_block_ids: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            locale: "ru".into(),
            theme: "dark".into(),
            accent: "violet".into(),
            sdk_path: None,
            autosave_seconds: 30,
            mcp_enabled: false,
            mcp_port: 8791,
            mcp_require_confirm: true,
            graph_detail: "scenes".into(),
            last_project: None,
            recent_projects: Vec::new(),
            ai: AiSettings::default(),
            editor_font_size: 16,
            show_block_ids: false,
        }
    }
}

impl Settings {
    pub fn dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("RenPyStudio")
    }

    pub fn path() -> PathBuf {
        Self::dir().join("settings.json")
    }

    pub fn load() -> Self {
        let path = Self::path();
        let Ok(raw) = std::fs::read_to_string(&path) else {
            return Settings::default();
        };
        let clean = raw.trim_start_matches('\u{feff}');
        let mut settings: Settings = serde_json::from_str(clean).unwrap_or_default();
        settings.ai.unlock();
        settings
    }

    pub fn save(&self) -> Result<(), String> {
        let dir = Self::dir();
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let mut snapshot = self.clone();
        snapshot.ai.seal();
        let mut json = serde_json::to_string_pretty(&snapshot).map_err(|e| e.to_string())?;
        json.push('\n');
        crate::store::write_atomic(&Self::path(), json.as_bytes())
    }

    pub fn remember_project(&mut self, path: &str, title: &str) {
        self.last_project = Some(path.to_string());
        self.recent_projects.retain(|p| !p.path.eq_ignore_ascii_case(path));
        self.recent_projects.insert(
            0,
            RecentProject {
                path: path.to_string(),
                title: title.to_string(),
                opened_at: crate::model::now_iso(),
            },
        );
        self.recent_projects.truncate(12);
    }
}
