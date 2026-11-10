//! Headless commands, used by CI and by the end-to-end generator test:
//!   renpy-studio --new <dir> <title> [--sdk <path>]
//!   renpy-studio --export <dir> [--force]
//!   renpy-studio --lint <dir> [--sdk <path>]
//!   renpy-studio --preview <dir> <screen> [--sdk <path>] [--hold <секунды>]

use crate::generator::GenOptions;
use crate::model::Project;
use crate::sdk::SdkService;
use crate::store::Store;
use std::path::{Path, PathBuf};
use std::time::Duration;

fn arg_value(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|index| args.get(index + 1))
        .cloned()
}

pub fn run(args: &[String]) -> Option<i32> {
    if let Some(index) = args.iter().position(|a| a == "--new") {
        let dir = args.get(index + 1)?;
        let title = args.get(index + 2)?.clone();
        let sdk_path = arg_value(args, "--sdk");
        let sdk = SdkService::default();
        let root = PathBuf::from(dir);
        let info = sdk.detect(sdk_path.as_deref(), Some(&root), true);
        let slug = crate::model::slugify(&title);
        match crate::templates::scaffold(&root, info.as_ref().map(|i| Path::new(&i.root)), &title, &slug) {
            Ok(()) => {}
            Err(err) => {
                eprintln!("scaffold: {err}");
                return Some(1);
            }
        }
        let mut store = match Store::create(&root, &title) {
            Ok(store) => store,
            Err(err) => {
                eprintln!("create: {err}");
                return Some(1);
            }
        };
        if let Some(info) = info {
            store.project.meta.renpy_version = info.version;
        }
        if let Err(err) = store.save() {
            eprintln!("save: {err}");
            return Some(1);
        }
        println!("created {}", root.display());
        return Some(0);
    }

    if let Some(index) = args.iter().position(|a| a == "--export") {
        let dir = args.get(index + 1)?;
        let force = args.iter().any(|a| a == "--force");
        let store = match Store::open(Path::new(dir)) {
            Ok(store) => store,
            Err(err) => {
                eprintln!("open: {err}");
                return Some(1);
            }
        };
        let opts = GenOptions {
            locale: store.project.meta.default_locale.clone(),
            force,
        };
        match crate::exporter::export(&store.project, Path::new(dir), &opts) {
            Ok((report, _, _)) => {
                for file in &report.written {
                    println!("wrote {file}");
                }
                for conflict in &report.conflicts {
                    println!("conflict {} — {}", conflict.path, conflict.reason);
                }
                for warning in &report.warnings {
                    println!("warning {warning}");
                }
                println!(
                    "done: {} written, {} skipped, {} removed",
                    report.written.len(),
                    report.skipped.len(),
                    report.removed.len()
                );
                return Some(if report.conflicts.is_empty() { 0 } else { 2 });
            }
            Err(err) => {
                eprintln!("export: {err}");
                return Some(1);
            }
        }
    }

    if let Some(index) = args.iter().position(|a| a == "--lint") {
        let dir = args.get(index + 1)?;
        let sdk_path = arg_value(args, "--sdk");
        let service = SdkService::default();
        let info = match service.detect(sdk_path.as_deref(), Some(Path::new(dir)), true) {
            Some(info) => info,
            None => {
                eprintln!("SDK not found");
                return Some(1);
            }
        };
        let report = service.lint(&info, Path::new(dir));
        println!("{}", report.raw);
        for problem in &report.problems {
            println!("{}:{} [{}] {}", problem.file, problem.line, problem.severity, problem.message);
        }
        return Some(if report.problems.iter().all(|p| p.severity != "error") && report.ok { 0 } else { 2 });
    }

    if let Some(index) = args.iter().position(|a| a == "--preview") {
        // Проверка моста IDE↔игра без графической студии: экспорт, временный
        // harness и запуск игры с теми же env-переменными, что ставит GUI.
        let dir = args.get(index + 1)?;
        let screen = args.get(index + 2)?.clone();
        let hold: u64 = arg_value(args, "--hold")
            .and_then(|value| value.parse().ok())
            .unwrap_or(12);
        let root = PathBuf::from(dir);
        let store = match Store::open(&root) {
            Ok(store) => store,
            Err(err) => {
                eprintln!("open: {err}");
                return Some(1);
            }
        };
        let opts = GenOptions {
            locale: store.project.meta.default_locale.clone(),
            force: false,
        };
        if let Err(err) = crate::exporter::export(&store.project, &root, &opts) {
            eprintln!("export: {err}");
            return Some(1);
        }
        let service = SdkService::default();
        let info = match service.detect(arg_value(args, "--sdk").as_deref(), Some(&root), true) {
            Some(info) => info,
            None => {
                eprintln!("SDK not found");
                return Some(1);
            }
        };
        let ru = store
            .project
            .meta
            .default_locale
            .trim()
            .to_lowercase()
            .starts_with("ru");
        let bridge = match crate::bridge::Bridge::start() {
            Ok(bridge) => bridge,
            Err(err) => {
                eprintln!("bridge: {err}");
                return Some(1);
            }
        };
        let harness = crate::commands::preview_harness(&screen, ru);
        if let Err(err) = crate::store::write_atomic(
            &root.join("game").join("zz_studio_preview.rpy"),
            harness.as_bytes(),
        ) {
            eprintln!("harness: {err}");
            return Some(1);
        }
        let envs = vec![
            ("RENPY_STUDIO_PREVIEW", "1".to_string()),
            ("RENPY_STUDIO_BRIDGE", bridge.url()),
        ];
        match service.run_game_with_env(&info, &root, &envs) {
            Ok(pid) => println!("game pid {pid}"),
            Err(err) => {
                eprintln!("run: {err}");
                return Some(1);
            }
        }
        println!("screen {screen}");
        println!("bridge {}", bridge.url());
        for _ in 0..hold {
            std::thread::sleep(Duration::from_secs(1));
        }
        let polls = bridge.polls();
        let ready = bridge.ready();
        println!("polls {polls}");
        println!("ready {ready}");
        if !ready {
            eprintln!("экран предпросмотра не показался за {hold} с (опросов: {polls})");
        }
        service.stop_game();
        return Some(if ready { 0 } else { 2 });
    }

    if let Some(index) = args.iter().position(|a| a == "--import") {
        let dir = args.get(index + 1)?;
        let title = arg_value(args, "--title").unwrap_or_else(|| {
            Path::new(dir)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "Imported".into())
        });
        let root = PathBuf::from(dir);
        let (inputs, skipped) = match crate::importer::collect_sources(&root) {
            Ok(value) => value,
            Err(err) => {
                eprintln!("collect: {err}");
                return Some(1);
            }
        };
        if inputs.is_empty() {
            eprintln!("no .rpy sources found in {}", root.display());
            return Some(1);
        }
        let (project, report) = crate::importer::import(&inputs, &title, &crate::model::slugify(&title));
        if let Err(err) = crate::commands::backup_sources(&root, &inputs) {
            eprintln!("backup: {err}");
            return Some(1);
        }
        let mut store = match Store::create(&root, &title) {
            Ok(store) => store,
            Err(err) => {
                eprintln!("create: {err}");
                return Some(1);
            }
        };
        store.project = project;
        if store.project.meta.start_scene.is_none() {
            store.project.meta.start_scene = store.project.scenes.first().map(|s| s.id.clone());
        }
        if let Err(err) = store.save() {
            eprintln!("save: {err}");
            return Some(1);
        }
        let opts = GenOptions {
            locale: store.project.meta.default_locale.clone(),
            force: true,
        };
        match crate::exporter::export(&store.project, &root, &opts) {
            Ok((export, _, _)) => {
                println!(
                    "imported {} files: {} scenes, {} blocks, {} code blocks, {} warnings",
                    report.files,
                    report.scenes,
                    report.recognized,
                    report.code_blocks,
                    report.warnings.len()
                );
                if !skipped.is_empty() {
                    println!("skipped: {}", skipped.join(", "));
                }
                println!("exported {} files", export.written.len());
                for warning in report.warnings.iter().take(8) {
                    println!("warning {warning}");
                }
                return Some(0);
            }
            Err(err) => {
                eprintln!("export: {err}");
                return Some(1);
            }
        }
    }

    if let Some(index) = args.iter().position(|a| a == "--print-missing") {
        // Debug helper: list references that would break in Ren'Py.
        let dir = args.get(index + 1)?;
        let store = match Store::open(Path::new(dir)) {
            Ok(store) => store,
            Err(err) => {
                eprintln!("open: {err}");
                return Some(1);
            }
        };
        let assets = crate::assets::scan(Path::new(dir));
        let report = crate::validation::validate(&store.project, &assets);
        for issue in report.issues {
            println!("[{}] {} {}", issue.severity, issue.code, issue.message);
        }
        return Some(0);
    }

    None
}

#[allow(dead_code)]
fn _unused(_: &Project) {}
