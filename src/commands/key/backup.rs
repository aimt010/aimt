use std::path::{Path, PathBuf};

use aimt::core::security::backup::{create_backup, create_backup_for};

fn print_backup_help() {
    println!("AIMT — AI Mapping Taxonomy (key backup)");
    println!();
    println!("USAGE:");
    println!("  aimt key backup [project.aimt] --output <path>");
    println!();
    println!("ARGS:");
    println!("  [project.aimt]    Optional .aimt project to backup (auto-discovers if omitted)");
    println!("  --output <path>   Explicit output file for backup (required)");
    println!();
    println!("OPTIONS:");
    println!("  -h, --help        Show this help");
    println!();
    println!(
        "WARNING: Backup contains private key. Possession grants owner write capability. Keep secure."
    );
}

pub fn run(args: &[String]) {
    if args.is_empty() && false {
        print_backup_help();
        return;
    }
    let mut output: Option<PathBuf> = None;
    let mut aimt_path: Option<PathBuf> = None;
    // Support positional project: `aimt key backup [project.aimt] --output <path>`
    let mut positional_project: Option<PathBuf> = None;
    let mut args_slice = args;
    let mut owned: Vec<String> = Vec::new();
    if !args.is_empty()
        && !args[0].starts_with('-')
        && crate::commands::project::is_project_arg(&args[0])
    {
        positional_project = Some(PathBuf::from(&args[0]));
        owned.extend_from_slice(&args[1..]);
        args_slice = &owned;
    }
    let mut i = 0;
    while i < args_slice.len() {
        match args_slice[i].as_str() {
            "--output" => {
                if i + 1 >= args_slice.len() {
                    eprintln!("error: --output requires a value");
                    std::process::exit(1);
                }
                output = Some(PathBuf::from(&args_slice[i + 1]));
                i += 2;
            }
            "--aimt" => {
                // Deprecated, kept for backward compat
                if i + 1 >= args_slice.len() {
                    eprintln!("error: --aimt requires a value");
                    std::process::exit(1);
                }
                aimt_path = Some(PathBuf::from(&args_slice[i + 1]));
                i += 2;
            }
            "-h" | "--help" | "help" => {
                print_backup_help();
                return;
            }
            s if s.starts_with('-') => {
                eprintln!("error: unknown option '{}'", s);
                print_backup_help();
                std::process::exit(1);
            }
            _ => {
                eprintln!("error: unexpected argument '{}'", args_slice[i]);
                print_backup_help();
                std::process::exit(1);
            }
        }
    }
    // Positional project wins if no explicit --aimt
    if aimt_path.is_none() {
        aimt_path = positional_project;
    }

    let out_path = match output {
        Some(p) => p,
        None => {
            eprintln!("error: --output is required for backup");
            eprintln!("Usage: aimt key backup [project.aimt] --output <path>");
            std::process::exit(1);
        }
    };

    // Prevent storing backup inside project directory by default? We allow any path, but warn if inside .aimt parent?
    // Just ensure not inside .aimt itself and not silently overwriting
    if out_path.exists() {
        eprintln!(
            "error: backup file already exists at {} (will not overwrite)",
            out_path.display()
        );
        std::process::exit(1);
    }

    // Resolve --aimt if not provided: use discovery
    let resolved_aimt: Option<PathBuf> = match aimt_path {
        Some(p) => match crate::commands::project::resolve_aimt_project(Some(p.as_path())) {
            Ok(r) => Some(r),
            Err(e) => {
                eprintln!("error: {}", e);
                std::process::exit(1);
            }
        },
        None => None,
    };
    let res = if let Some(aimt) = resolved_aimt {
        let store = match aimt::store::Store::open(&aimt) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("error: failed to open .aimt: {}", e);
                std::process::exit(1);
            }
        };
        let pub_key = match store.owner_public_key() {
            Some(k) => k,
            None => {
                eprintln!("error: no owner_public_key in .aimt");
                std::process::exit(1);
            }
        };
        create_backup_for(&pub_key, &out_path)
    } else {
        create_backup(&out_path)
    };

    match res {
        Ok(()) => {
            println!("Backup created at {}", out_path.display());
            eprintln!("WARNING: Backup contains private key. Keep secure, do not share or commit.");
        }
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    }

    // Never log private key
    let _ = Path::new("");
}
