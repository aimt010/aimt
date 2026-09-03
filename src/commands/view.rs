use std::path::PathBuf;

use aimt::visualizer;

use super::print_help;

pub fn run(args: &[String]) {
    let mut path: Option<PathBuf> = None;
    let mut port: u16 = 3000;
    let mut no_open = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--port" => {
                if i + 1 >= args.len() {
                    eprintln!("error: --port requires a value");
                    std::process::exit(1);
                }
                port = args[i + 1].parse().unwrap_or_else(|_| {
                    eprintln!("error: invalid port '{}'", args[i + 1]);
                    std::process::exit(1);
                });
                i += 2;
            }
            "--no-open" => {
                no_open = true;
                i += 1;
            }
            "-h" | "--help" => {
                print_help();
                return;
            }
            s if s.starts_with('-') => {
                eprintln!("error: unknown option '{}'", s);
                eprintln!("Usage: aimt view [project.aimt] [--port <PORT>] [--no-open]");
                std::process::exit(1);
            }
            p => {
                if path.is_some() {
                    eprintln!("error: multiple paths given");
                    std::process::exit(1);
                }
                path = Some(PathBuf::from(p));
                i += 1;
            }
        }
    }
    let path = match crate::commands::project::resolve_aimt_project(path.as_deref()) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    };
    if let Err(e) = visualizer::run(&path, port, no_open) {
        eprintln!("error: {}", e);
        std::process::exit(1);
    }
}
