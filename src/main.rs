mod commands;

use commands::print_help;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        print_help();
        std::process::exit(0);
    }
    match args[1].as_str() {
        "-h" | "--help" | "help" => {
            print_help();
        }
        "-V" | "--version" | "version" => {
            println!("aimt {}", env!("CARGO_PKG_VERSION"));
        }
        "init" => {
            commands::init::run(&args[2..]);
        }
        "view" => {
            commands::view::run(&args[2..]);
        }
        "search" => {
            let mut a = vec!["search".to_string()];
            a.extend_from_slice(&args[2..]);
            commands::tool::run(&a);
        }
        "read" => {
            let mut a = vec!["read".to_string()];
            a.extend_from_slice(&args[2..]);
            commands::tool::run(&a);
        }
        "follow-parent" | "follow_parent" => {
            let mut a = vec!["follow_parent".to_string()];
            a.extend_from_slice(&args[2..]);
            commands::tool::run(&a);
        }
        "follow-file" | "follow_file" => {
            let mut a = vec!["follow_file".to_string()];
            a.extend_from_slice(&args[2..]);
            commands::tool::run(&a);
        }
        "follow-relation" | "follow_relation" => {
            let mut a = vec!["follow_relation".to_string()];
            a.extend_from_slice(&args[2..]);
            commands::tool::run(&a);
        }
        "validate" => {
            let mut a = vec!["validate".to_string()];
            a.extend_from_slice(&args[2..]);
            commands::tool::run(&a);
        }
        "aimt" => {
            // Deprecated: `aimt aimt search ...` kept for backward compat
            commands::tool::run(&args[2..]);
        }
        "install" => {
            commands::install::run(&args[2..]);
        }
        "upgrade" => {
            commands::upgrade::run(&args[2..]);
        }
        "uninstall" => {
            commands::uninstall::run(&args[2..]);
        }
        "login" => {
            commands::login::run_login(&args[2..]);
        }
        "logout" => {
            commands::login::run_logout(&args[2..]);
        }
        "status" => {
            commands::status::run(&args[2..]);
        }
        "key" => {
            if args.len() < 3 {
                eprintln!("error: missing key subcommand");
                eprintln!("Usage: aimt key <backup|rotate> ...");
                std::process::exit(1);
            }
            match args[2].as_str() {
                "backup" => commands::key::backup::run(&args[3..]),
                "rotate" => commands::key::rotate::run(&args[3..]),
                "-h" | "--help" | "help" => {
                    println!("AIMT — AI Mapping Taxonomy (key)");
                    println!();
                    println!("USAGE:");
                    println!("  aimt key backup [project.aimt] --output <path>");
                    println!("  aimt key rotate [project.aimt]");
                    println!();
                    println!("OPTIONS:");
                    println!("  -h, --help    Show this help");
                }
                other => {
                    eprintln!("error: unknown key subcommand '{}'", other);
                    eprintln!("Usage: aimt key <backup|rotate> ...");
                    std::process::exit(1);
                }
            }
        }
        other => {
            eprintln!("error: unknown command '{}'", other);
            print_help();
            std::process::exit(1);
        }
    }
}
