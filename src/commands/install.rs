use std::path::PathBuf;

use aimt::plugins::installer::registry::installer_for;

fn print_install_help() {
    println!("AIMT — AI Mapping Taxonomy (install)");
    println!();
    println!("USAGE:");
    println!("  aimt install <tool> [--path <DIR>]");
    println!();
    println!("TOOLS:");
    println!("  opencode    Install AIMT for OpenCode");
    println!("  claude      Install AIMT for Claude Code");
    println!("  codex       Install AIMT for Codex");
    println!("  antigravity Install AIMT for Antigravity");
    println!("  kilo        Install AIMT for Kilo Code");
    println!("  copilot     Install AIMT for GitHub Copilot CLI");
    println!("  aider       Install AIMT for Aider");
    println!("  cursor      Install AIMT for Cursor");
    println!("  gemini      Install AIMT for Gemini CLI");
    println!("  kimi        Install AIMT for Kimi Code");
    println!();
    println!("OPTIONS:");
    println!("  --path <DIR>  Project directory (default: .)");
    println!("  -h, --help    Show this help");
}

pub fn run(args: &[String]) {
    if args.is_empty() || args[0] == "-h" || args[0] == "--help" || args[0] == "help" {
        print_install_help();
        return;
    }
    let tool = args[0].as_str();
    let mut project_dir = PathBuf::from(".");
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--path" => {
                if i + 1 >= args.len() {
                    eprintln!("error: --path requires a value");
                    std::process::exit(1);
                }
                project_dir = PathBuf::from(&args[i + 1]);
                i += 2;
            }
            "-h" | "--help" => {
                print_install_help();
                return;
            }
            s if s.starts_with('-') => {
                eprintln!("error: unknown option '{}'", s);
                eprintln!("Usage: aimt install <tool> [--path <DIR>]");
                std::process::exit(1);
            }
            _ => {
                eprintln!("error: unexpected argument '{}'", args[i]);
                print_install_help();
                std::process::exit(1);
            }
        }
    }
    let registry = installer_for(tool);
    match registry {
        Some(installer) => match installer.install(&project_dir) {
            Ok(report) => {
                // Keep detailed report for tests/debug, but normal CLI output is concise
                let display = match report.tool.as_str() {
                    "opencode" => "OpenCode",
                    "claude" => "Claude Code",
                    "codex" => "Codex",
                    "antigravity" => "Antigravity",
                    "kilo" => "Kilo Code",
                    "copilot" => "GitHub Copilot",
                    "aider" => "Aider",
                    "cursor" => "Cursor",
                    "gemini" => "Gemini",
                    "kimi" => "Kimi",
                    other => other,
                };
                println!("AIMT installed for {}.", display);
            }
            Err(e) => {
                eprintln!("error: {}", e);
                std::process::exit(1);
            }
        },
        None => {
            eprintln!(
                "error: unknown tool '{}'. Supported: opencode, claude, codex, antigravity, kilo, copilot, aider, cursor, gemini, kimi",
                tool
            );
            std::process::exit(1);
        }
    }
}
