pub fn print_help() {
    println!("AIMT — AI Mapping Taxonomy");
    println!();
    println!("USAGE:");
    println!("  aimt init [name]");
    println!("  aimt search [project.aimt] --query <q>");
    println!("  aimt read [project.aimt] <id>");
    println!("  aimt follow-parent [project.aimt] <id>");
    println!("  aimt follow-file [project.aimt] <id>");
    println!("  aimt follow-relation [project.aimt] <id>");
    println!("  aimt validate [project.aimt]");
    println!("  aimt status [project.aimt]");
    println!("  aimt login [project.aimt] [--key <private>]");
    println!("  aimt logout [project.aimt]");
    println!("  aimt key backup [project.aimt] --output <path>");
    println!("  aimt key rotate [project.aimt]");
    println!("  aimt view [project.aimt] [--port <PORT>] [--no-open]");
    println!(
        "  aimt install <tool> [--path <DIR>]  Install AIMT for an AI tool (opencode, claude, codex, antigravity, kilo, copilot, aider, cursor, gemini, kimi)"
    );
    println!(
        "  aimt upgrade [--path <DIR>] [--target <VER>]  Upgrade AIMT Engine/runtime (preserves .aimt)"
    );
    println!(
        "  aimt uninstall [--path <DIR>] [--yes]  Uninstall AIMT Engine/runtime (preserves .aimt)"
    );
    println!();
    println!("COMMANDS:");
    println!("  init            Create a new .aimt package");
    println!("  search          Search entities by query");
    println!("  read            Read entity by id");
    println!("  follow-parent   Follow parent hierarchy");
    println!("  follow-file     Follow file for @frame");
    println!("  follow-relation Follow relations");
    println!("  validate        Validate AIMT package");
    println!("  status          Show authentication status (without exposing secrets)");
    println!("  login           Authenticate as owner (stores private key securely)");
    println!("  logout          Clear stored credential(s)");
    println!("  key             Owner key lifecycle (backup, rotate)");
    println!("  view            Open an .aimt package in the visual explorer");
    println!(
        "  install         Install AIMT for an AI tool (opencode, claude, codex, antigravity, kilo, copilot, aider, cursor, gemini, kimi)"
    );
    println!(
        "  upgrade         Upgrade AIMT Engine/runtime and owned integration (preserves .aimt and source)"
    );
    println!("  uninstall       Remove AIMT installation/runtime (preserves .aimt and source)");
    println!();
    println!("OPTIONS:");
    println!("  --port <PORT>   Port to listen on (default: 3000, 0 = auto)");
    println!("  --no-open       Do not open browser automatically");
    println!("  -h, --help      Show this help");
    println!("  -V, --version   Show version");
}
