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
    pub api_key: String,
    pub model: String,
}

impl Default for AiSettings {
    fn default() -> Self {
        AiSettings {
            base_url: "https://api.openai.com/v1".into(),
            api_key: String::new(),
            model: String::new(),
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
        serde_json::from_str(clean).unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), String> {
        let dir = Self::dir();
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let mut json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
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
