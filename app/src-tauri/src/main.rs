// Hide the console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Err(err) = rufplan_studio_lib::run() {
        eprintln!("Rufplan Studio failed to start: {err:#}");
        std::process::exit(1);
    }
}
