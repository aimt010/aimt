use std::path::{Path, PathBuf};

use aimt::core::security::auth::{list_stored_public_keys, load_credential};

fn print_status_help() {
    println!("AIMT — AI Mapping Taxonomy (status)");
    println!();
    println!("USAGE:");
    println!("  aimt status [project.aimt]");
    println!();
    println!("ARGS:");
    println!("  [project.aimt]  Optional .aimt package (auto-discovers if omitted)");
    println!();
    println!("OPTIONS:");
    println!("  -h, --help    Show this help");
}

pub fn run(args: &[String]) {
    if args.len() == 1 && (args[0] == "-h" || args[0] == "--help" || args[0] == "help") {
        print_status_help();
        return;
    }
    if args.len() > 1 {
        eprintln!("error: too many arguments for status");
        eprintln!("Usage: aimt status [path]");
        std::process::exit(1);
    }
    if args.len() == 1 && (args[0] == "-h" || args[0] == "--help") {
        print_status_help();
        return;
    }

    // Resolve project if explicit, otherwise try discovery (global status when 0 files)
    let explicit = args.first().map(|s| Path::new(s.as_str()));
    let resolved: Option<PathBuf> = if let Some(p) = explicit {
        match crate::commands::project::resolve_aimt_project(Some(p)) {
            Ok(r) => Some(r),
            Err(e) => {
                eprintln!("error: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        let projects = crate::commands::project::discover_aimt_projects(Path::new("."));
        match projects.len() {
            0 => None,
            1 => Some(projects.into_iter().next().unwrap()),
            _ => match crate::commands::project::resolve_aimt_project(None) {
                Ok(r) => Some(r),
                Err(e) => {
                    eprintln!("error: {}", e);
                    std::process::exit(1);
                }
            },
        }
    };

    let keys = list_stored_public_keys();
    // Also check keyring entries that have no fallback file? list_stored_public_keys only lists fallback files.
    // For status, we consider fallback as source of truth (since we always write fallback alongside keyring).
    println!("AIMT Authentication");
    if keys.is_empty() {
        println!("Status: Not logged in");
        if let Some(path) = resolved.as_ref() {
            // Try to open .aimt to show its owner, but still not logged in
            if let Ok(store) = aimt::store::Store::open(path) {
                if let Some(pub_key) = store.owner_public_key() {
                    println!("Project: {}", path.display());
                    println!(
                        "Owner: {} (not authenticated)",
                        &pub_key[..8.min(pub_key.len())]
                    );
                    println!("Credential: None");
                } else {
                    println!("Project: {} (legacy, no owner)", path.display());
                }
            }
        }
        return;
    }

    // Logged in
    println!("Status: Logged in");
    // Show first owner truncated, and count
    let first = &keys[0];
    println!("Owner: {}", &first[..8.min(first.len())]);
    println!("Credential: Secure storage");
    if keys.len() > 1 {
        println!("Owners: {} stored", keys.len());
    }

    if let Some(path) = resolved.as_ref() {
        match aimt::store::Store::open(path) {
            Ok(store) => {
                println!("Project: {}", path.display());
                if let Some(pub_key) = store.owner_public_key() {
                    let truncated = &pub_key[..8.min(pub_key.len())];
                    println!("Project owner: {}", truncated);
                    // Verify stored credential matches this project's owner
                    if let Some(private) = load_credential(&pub_key) {
                        if aimt::core::security::keys::verify_write_credential(&pub_key, &private) {
                            println!("Project auth: Verified");
                        } else {
                            println!(
                                "Project auth: Mismatch (stored credential does not match project)"
                            );
                        }
                    } else {
                        println!("Project auth: Not authenticated for this project");
                    }
                } else {
                    println!("Project owner: None (legacy)");
                }
            }
            Err(e) => {
                println!("Project: {} (failed to open: {})", path.display(), e);
            }
        }
    }
}
