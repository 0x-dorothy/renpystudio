//! Ren'Py SDK adapter. Every SDK call lives here so version changes stay contained.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SdkInfo {
    pub root: String,
    pub exe: String,
    pub version: String,
    pub launcher: bool,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LintProblem {
    pub file: String,
    pub line: u32,
    pub message: String,
    pub severity: String,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct LintReport {
    pub ok: bool,
    pub problems: Vec<LintProblem>,
    pub raw: String,
    pub ran: bool,
    pub message: String,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct BuildReport {
    pub ok: bool,
    pub log: String,
    pub artifact: Option<String>,
}

pub struct SdkService {
    cached: Mutex<Option<SdkInfo>>,
    running: Mutex<Option<Child>>,
}

impl Default for SdkService {
    fn default() -> Self {
        SdkService {
            cached: Mutex::new(None),
            running: Mutex::new(None),
        }
    }
}

fn is_sdk_root(path: &Path) -> bool {
    path.join("renpy.exe").is_file() && path.join("renpy").is_dir() && path.join("launcher").is_dir()
}

fn read_version(root: &Path) -> String {
    let exe = root.join("renpy.exe");
    let mut cmd = Command::new(&exe);
    cmd.arg("--version")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .stdin(Stdio::null());
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    match cmd.output() {
        Ok(out) => {
            let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
            text.lines().next().unwrap_or("").trim().to_string()
        }
        Err(_) => String::new(),
    }
}

fn make_info(root: &Path, version: Option<String>) -> SdkInfo {
    SdkInfo {
        root: root.display().to_string(),
        exe: root.join("renpy.exe").display().to_string(),
        version: version.unwrap_or_else(|| read_version(root)),
        launcher: root.join("launcher").is_dir(),
    }
}

/// Typical places the SDK ends up on Windows, cheapest checks first.
fn candidate_roots(preferred: Option<&str>, project_root: Option<&Path>) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    if let Some(p) = preferred.filter(|p| !p.trim().is_empty()) {
        out.push(PathBuf::from(p));
    }
    if let Ok(env) = std::env::var("RENPY_SDK") {
        if !env.trim().is_empty() {
            out.push(PathBuf::from(env));
        }
    }
    // The studio often lives *inside* the SDK folder (as in this repository).
    if let Ok(exe) = std::env::current_exe() {
        let mut dir = exe.parent();
        for _ in 0..6 {
            let Some(current) = dir else { break };
            out.push(current.to_path_buf());
            dir = current.parent();
        }
    }
    if let Some(project) = project_root {
        // The SDK folder often *contains* the working project.
        if let Some(parent) = project.parent() {
            out.push(parent.to_path_buf());
        }
        out.push(project.to_path_buf());
    }
    for base in [
        dirs::home_dir(),
        dirs::data_local_dir(),
        dirs::download_dir(),
        dirs::desktop_dir(),
        dirs::document_dir(),
    ]
    .into_iter()
    .flatten()
    {
        for name in ["renpy", "renpy-sdk", "RenPy", "RenPy SDK"] {
            out.push(base.join(name));
        }
    }
    for drive in ["C:\\", "D:\\", "E:\\", "F:\\", "G:\\"] {
        let root = PathBuf::from(drive);
        if !root.exists() {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(&root) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_lowercase();
                if (name.starts_with("renpy")) && is_sdk_root(&entry.path()) {
                    out.push(entry.path());
                }
            }
        }
    }
    out
}

impl SdkService {
    /// Finds and validates the SDK; caches the result until `invalidate`.
    pub fn detect(
        &self,
        preferred: Option<&str>,
        project_root: Option<&Path>,
        force: bool,
    ) -> Option<SdkInfo> {
        if !force {
            if let Ok(guard) = self.cached.lock() {
                if let Some(info) = guard.as_ref() {
                    if Path::new(&info.exe).is_file() {
                        return Some(info.clone());
                    }
                }
            }
        }
        let mut found: Option<SdkInfo> = None;
        for candidate in candidate_roots(preferred, project_root) {
            if is_sdk_root(&candidate) {
                found = Some(make_info(&candidate, None));
                break;
            }
        }
        if let Ok(mut guard) = self.cached.lock() {
            *guard = found.clone();
        }
        found
    }

    pub fn invalidate(&self) {
        if let Ok(mut guard) = self.cached.lock() {
            *guard = None;
        }
    }

    #[allow(dead_code)]
    pub fn cached_info(&self) -> Option<SdkInfo> {
        self.cached.lock().ok().and_then(|g| g.clone())
    }

    #[allow(dead_code)]
    pub fn running(&self) -> bool {
        self.running
            .lock()
            .map(|mut g| match g.as_mut() {
                Some(child) => !matches!(child.try_wait(), Ok(Some(_))),
                None => false,
            })
            .unwrap_or(false)
    }

    /// Launches the game through the SDK; a previous run is terminated first.
    pub fn run_game(&self, sdk: &SdkInfo, project: &Path) -> Result<String, String> {
        self.stop_game();
        let mut cmd = Command::new(&sdk.exe);
        cmd.arg(project)
            .arg("run")
            .current_dir(project)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .stdin(Stdio::null());
        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);
        let child = cmd
            .spawn()
            .map_err(|e| format!("Не удалось запустить игру: {e}"))?;
        let pid = child.id();
        if let Ok(mut guard) = self.running.lock() {
            *guard = Some(child);
        }
        Ok(format!("{pid}"))
    }

    pub fn stop_game(&self) -> bool {
        if let Ok(mut guard) = self.running.lock() {
            if let Some(mut child) = guard.take() {
                let _ = child.kill();
                let _ = child.wait();
                return true;
            }
        }
        false
    }

    /// Runs `renpy.exe <project> lint <report> --error-code` and parses the report.
    pub fn lint(&self, sdk: &SdkInfo, project: &Path) -> LintReport {
        let mut report = LintReport::default();
        let studio_dir = project.join(crate::store::STUDIO_DIR);
        let _ = std::fs::create_dir_all(&studio_dir);
        let report_path = studio_dir.join("lint-report.txt");
        let _ = std::fs::remove_file(&report_path);
        let mut cmd = Command::new(&sdk.exe);
        cmd.arg(project)
            .arg("lint")
            .arg(&report_path)
            .arg("--error-code")
            .arg("--all-problems")
            .current_dir(project)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null());
        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);
        let output = match cmd.output() {
            Ok(out) => out,
            Err(e) => {
                report.message = format!("Не удалось запустить lint: {e}");
                return report;
            }
        };
        report.ran = true;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let file_text = std::fs::read_to_string(&report_path).unwrap_or_default();
        let raw = if file_text.trim().is_empty() { stdout } else { file_text };
        report.raw = raw.clone();
        report.ok = output.status.success();
        // Ren'Py writes lint problems as `game/file.rpy:LINE message`.
        for line in raw.lines() {
            let trimmed = line.trim_end();
            if trimmed.is_empty() || trimmed.starts_with("Lint") || trimmed.starts_with("Statistics") {
                continue;
            }
            // Syntax errors are printed as: File "game/x.rpy", line 12: message
            if let Some(rest) = trimmed.strip_prefix("File \"") {
                if let Some((file, rest2)) = rest.split_once("\", line ") {
                    let digits: String = rest2.chars().take_while(|c| c.is_ascii_digit()).collect();
                    if !digits.is_empty() {
                        report.problems.push(LintProblem {
                            file: file.replace('\\', "/"),
                            line: digits.parse::<u32>().unwrap_or(0),
                            message: rest2[digits.len()..]
                                .trim_start_matches(':')
                                .trim()
                                .to_string(),
                            severity: "error".into(),
                        });
                        continue;
                    }
                }
            }
            if let Some(colon) = trimmed.find(".rpy:") {
                let file = trimmed[..colon + 4].to_string();
                let rest = &trimmed[colon + 5..];
                let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
                let line_no = digits.parse::<u32>().unwrap_or(0);
                let message = rest[digits.len()..].trim_start_matches(':').trim().to_string();
                let lower = message.to_lowercase();
                let severity = if lower.contains("error")
                    || lower.contains("not defined")
                    || lower.contains("undefined")
                    || lower.contains("invalid")
                    || lower.contains("could not")
                    || lower.contains("exception")
                    || lower.contains("syntax")
                {
                    "error"
                } else {
                    "warning"
                };
                report.problems.push(LintProblem {
                    file: file.replace('\\', "/"),
                    line: line_no,
                    message,
                    severity: severity.into(),
                });
            }
        }
        if !report.ok && report.problems.is_empty() {
            report.message = format!(
                "Lint завершился с ошибкой. {}",
                raw.lines().rev().take(3).collect::<Vec<_>>().join(" ")
            );
        } else {
            report.message = if report.problems.is_empty() {
                "Ошибок не найдено".into()
            } else {
                format!("Проблем: {}", report.problems.len())
            };
        }
        // Clean up so a stale file never confuses the next run.
        let _ = std::fs::remove_file(&report_path);
        report
    }

    /// Builds a distributable through the SDK launcher.
    pub fn build(
        &self,
        sdk: &SdkInfo,
        project: &Path,
        package: &str,
        destination: &Path,
    ) -> BuildReport {
        let mut report = BuildReport::default();
        std::fs::create_dir_all(destination).ok();
        let mut cmd = Command::new(&sdk.exe);
        cmd.arg("launcher")
            .arg("distribute")
            .arg(project)
            .arg("--package")
            .arg(package)
            .arg("--destination")
            .arg(destination)
            .current_dir(&sdk.root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null());
        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);
        match cmd.output() {
            Ok(out) => {
                let mut log = String::from_utf8_lossy(&out.stdout).to_string();
                log.push_str(&String::from_utf8_lossy(&out.stderr));
                report.ok = out.status.success();
                report.log = log;
                if let Ok(entries) = std::fs::read_dir(destination) {
                    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
                    for entry in entries.flatten() {
                        if let Ok(meta) = entry.metadata() {
                            if let Ok(modified) = meta.modified() {
                                if newest.as_ref().map(|(t, _)| modified > *t).unwrap_or(true) {
                                    newest = Some((modified, entry.path()));
                                }
                            }
                        }
                    }
                    report.artifact = newest.map(|(_, p)| p.display().to_string());
                }
            }
            Err(e) => {
                report.ok = false;
                report.log = format!("Не удалось запустить сборку: {e}");
            }
        }
        report
    }
}

/// Writes the SDK path from the settings into a project-local `studio.json`.
pub fn write_project_settings(project_root: &Path, sdk_path: Option<&str>) -> Result<(), String> {
    let dir = project_root.join(crate::store::STUDIO_DIR);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join("studio.json");
    let payload = serde_json::json!({
        "sdkPath": sdk_path,
        "studioVersion": env!("CARGO_PKG_VERSION"),
    });
    let mut json = serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?;
    json.push('\n');
    crate::store::write_atomic(&path, json.as_bytes())
}
