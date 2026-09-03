use std::path::{Path, PathBuf};

use aimt::core::security::auth::{delete_credential, is_valid_for, store_credential};
use aimt::store::Store;

fn print_login_help() {
    println!("AIMT — AI Mapping Taxonomy (login)");
    println!();
    println!("USAGE:");
    println!("  aimt login [project.aimt] [--key <private>]");
    println!();
    println!("ARGS:");
    println!("  [project.aimt]  Path to the .aimt package (auto-discovers if omitted)");
    println!();
    println!("OPTIONS:");
    println!("  --key <private>  Provide private key hex directly (for testing/scripting)");
    println!("  -h, --help       Show this help");
}

fn print_logout_help() {
    println!("AIMT — AI Mapping Taxonomy (logout)");
    println!();
    println!("USAGE:");
    println!("  aimt logout [path]");
    println!();
    println!("ARGS:");
    println!("  [path]    Optional path to .aimt package to clear credential for");
    println!("            If no path given, clears all stored credentials");
    println!();
    println!("OPTIONS:");
    println!("  -h, --help    Show this help");
}

fn read_owner_public_key(path: &Path) -> Result<String, String> {
    let store = Store::open(path).map_err(|e| format!("failed to open .aimt: {}", e))?;
    store
        .owner_public_key()
        .ok_or_else(|| "no owner_public_key in .aimt (legacy package)".to_string())
}

pub fn run_login(args: &[String]) {
    if args.len() == 1 && (args[0] == "-h" || args[0] == "--help" || args[0] == "help") {
        print_login_help();
        return;
    }
    let mut aimt_path: Option<PathBuf> = None;
    let mut key_opt: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--key" => {
                if i + 1 >= args.len() {
                    eprintln!("error: --key requires a value");
                    std::process::exit(1);
                }
                key_opt = Some(args[i + 1].clone());
                i += 2;
            }
            "-h" | "--help" => {
                print_login_help();
                return;
            }
            s if s.starts_with('-') => {
                eprintln!("error: unknown option '{}'", s);
                print_login_help();
                std::process::exit(1);
            }
            _ => {
                if aimt_path.is_some() {
                    eprintln!("error: too many arguments for login");
                    eprintln!("Usage: aimt login <path> [--key <private>]");
                    std::process::exit(1);
                }
                aimt_path = Some(PathBuf::from(&args[i]));
                i += 1;
            }
        }
    }
    let path = match aimt_path {
        Some(p) => match crate::commands::project::resolve_aimt_project(Some(p.as_path())) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("error: {}", e);
                std::process::exit(1);
            }
        },
        None => match crate::commands::project::resolve_aimt_project(None) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("error: {}", e);
                std::process::exit(1);
            }
        },
    };

    let public_key = match read_owner_public_key(&path) {
        Ok(k) => k,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    };

    let private_hex = match key_opt {
        Some(k) => k,
        None => {
            // Prompt securely without echo
            match rpassword::prompt_password("Enter private key: ") {
                Ok(p) => p.trim().to_string(),
                Err(e) => {
                    eprintln!("error: failed to read private key: {}", e);
                    std::process::exit(1);
                }
            }
        }
    };

    if private_hex.is_empty() {
        eprintln!("error: private key must not be empty");
        std::process::exit(1);
    }

    if !is_valid_for(&public_key, &private_hex) {
        eprintln!("error: private key does not match owner_public_key");
        std::process::exit(1);
    }

    match store_credential(&public_key, &private_hex) {
        Ok(()) => {
            println!(
                "Authenticated for {} (owner {})",
                path.display(),
                &public_key[..8]
            );
        }
        Err(e) => {
            eprintln!("error: failed to store credential: {}", e);
            std::process::exit(1);
        }
    }
}

pub fn run_logout(args: &[String]) {
    if args.len() == 1 && (args[0] == "-h" || args[0] == "--help" || args[0] == "help") {
        print_logout_help();
        return;
    }
    if args.is_empty() {
        // Global logout: delete all credentials
        use aimt::core::security::auth::delete_all_credentials;
        match delete_all_credentials() {
            Ok(()) => {
                println!("Logged out (all credentials cleared)");
            }
            Err(e) => {
                eprintln!("error: failed to clear credentials: {}", e);
                std::process::exit(1);
            }
        }
        return;
    }
    if args.len() != 1 {
        eprintln!("error: too many arguments for logout");
        eprintln!("Usage: aimt logout [path]");
        std::process::exit(1);
    }
    if args[0] == "-h" || args[0] == "--help" {
        print_logout_help();
        return;
    }
    let raw = PathBuf::from(&args[0]);
    let path = match crate::commands::project::resolve_aimt_project(Some(raw.as_path())) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    };
    let public_key = match read_owner_public_key(&path) {
        Ok(k) => k,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    };
    match delete_credential(&public_key) {
        Ok(()) => {
            println!(
                "Logged out for {} (owner {})",
                path.display(),
                &public_key[..8]
            );
        }
        Err(e) => {
            eprintln!("error: failed to clear credential: {}", e);
            std::process::exit(1);
        }
    }
}
