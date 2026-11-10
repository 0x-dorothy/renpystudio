#![cfg_attr(all(not(debug_assertions), not(feature = "cli")), windows_subsystem = "windows")]

mod app;
mod ai;
mod assets;
mod bridge;
mod cli;
mod commands;
mod diff;
mod exporter;
mod generator;
mod importer;
mod mcp;
mod model;
mod plugins;
mod sdk;
mod secret;
mod settings;
mod store;
mod templates;
mod ui_templates;
mod util;
mod validation;
mod watcher;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        if let Some(code) = cli::run(&args) {
            std::process::exit(code);
        }
    }
    app::run();
}

