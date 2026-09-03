use std::path::PathBuf;

fn print_rotate_help() {
    println!("AIMT — AI Mapping Taxonomy (key rotate)");
    println!();
    println!("USAGE:");
    println!("  aimt key rotate <path>");
    println!();
    println!("ARGS:");
    println!("  <path>    Path to the .aimt package to rotate owner key for");
    println!();
    println!("OPTIONS:");
    println!("  -h, --help    Show this help");
}

pub fn run(args: &[String]) {
    if args.len() == 1 && (args[0] == "-h" || args[0] == "--help" || args[0] == "help") {
        print_rotate_help();
        return;
    }
    if args.len() > 1 {
        eprintln!("error: too many arguments for key rotate");
        eprintln!("Usage: aimt key rotate <path>");
        std::process::exit(1);
    }
    if !args.is_empty() && (args[0] == "-h" || args[0] == "--help") {
        print_rotate_help();
        return;
    }
    let path = if args.is_empty() {
        match crate::commands::project::resolve_aimt_project(None) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("error: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        match crate::commands::project::resolve_aimt_project(Some(
            PathBuf::from(&args[0]).as_path(),
        )) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("error: {}", e);
                std::process::exit(1);
            }
        }
    };
    match aimt::core::security::rotate::rotate(&path) {
        Ok((new_priv, new_pub)) => {
            println!(
                "Rotated owner key for {} (new owner {})",
                path.display(),
                &new_pub[..8]
            );
            // Do not print private key; it's stored securely
            let _ = new_priv;
        }
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    }
}
