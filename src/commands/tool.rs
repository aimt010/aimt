use std::path::PathBuf;

use aimt::core::data::store::Store;
use aimt::workflows::read::ReadWorkflow;

fn print_aimt_help() {
    println!("AIMT — AI Mapping Taxonomy");
    println!();
    println!("USAGE:");
    println!("  aimt search [project.aimt] --query <q>");
    println!("  aimt read [project.aimt] <id>");
    println!("  aimt follow-parent [project.aimt] <id>");
    println!("  aimt follow-file [project.aimt] <id>");
    println!("  aimt follow-relation [project.aimt] <id>");
    println!("  aimt validate [project.aimt]");
    println!();
    println!("ACTIONS:");
    println!("  search          Search entities by query");
    println!("  read            Read entity by id");
    println!("  follow-parent   Follow parent hierarchy");
    println!("  follow-file     Follow file/source for @frame");
    println!("  follow-relation Follow relations");
    println!("  validate        Validate AIMT package");
    println!();
    println!("OPTIONS:");
    println!("  --query <q>     Search query for search action");
    println!("  -h, --help      Show this help");
    println!();
    println!(
        "Project discovery: if [project.aimt] is omitted, AIMT discovers *.aimt in the current directory (0→ error, 1→ auto, 2+→ prompt). Explicit project always wins."
    );
}

pub fn run(args: &[String]) {
    if args.is_empty() || args[0] == "-h" || args[0] == "--help" || args[0] == "help" {
        print_aimt_help();
        return;
    }

    // Normalize hyphenated names to underscored for internal matching
    let raw_action = args[0].as_str();
    let action = match raw_action {
        "follow-parent" => "follow_parent",
        "follow-file" => "follow_file",
        "follow-relation" => "follow_relation",
        _ => raw_action,
    };

    // Extract optional positional project (first arg after action that ends with .aimt)
    // Also support deprecated --aimt flag for backward compat
    let mut aimt_path: Option<PathBuf> = None;
    let mut query: Option<String> = None;
    let mut id: Option<String> = None;
    let mut positional_id: Option<String> = None;
    let mut i = 1;

    // Check for positional project as first arg after action (ends with .aimt)
    if i < args.len()
        && !args[i].starts_with('-')
        && crate::commands::project::is_project_arg(&args[i])
    {
        aimt_path = Some(PathBuf::from(&args[i]));
        i += 1;
    }

    while i < args.len() {
        match args[i].as_str() {
            "--aimt" => {
                // Deprecated, but still support for backward compat
                if i + 1 >= args.len() {
                    eprintln!("error: --aimt requires a value");
                    std::process::exit(1);
                }
                aimt_path = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--query" => {
                if i + 1 >= args.len() {
                    eprintln!("error: --query requires a value");
                    std::process::exit(1);
                }
                query = Some(args[i + 1].clone());
                i += 2;
            }
            "--id" => {
                // Deprecated positional id flag, still support
                if i + 1 >= args.len() {
                    eprintln!("error: --id requires a value");
                    std::process::exit(1);
                }
                id = Some(args[i + 1].clone());
                i += 2;
            }
            "-h" | "--help" => {
                print_aimt_help();
                return;
            }
            s if s.starts_with('-') => {
                eprintln!("error: unknown option '{}'", s);
                print_aimt_help();
                std::process::exit(1);
            }
            _ => {
                // Positional id for read/follow
                if positional_id.is_some() || id.is_some() {
                    eprintln!("error: unexpected argument '{}'", args[i]);
                    print_aimt_help();
                    std::process::exit(1);
                }
                // For search, positional after project should not happen except --query
                // For read/follow, this is the id
                positional_id = Some(args[i].clone());
                // Also set id for backward compat if not already
                if id.is_none() {
                    id = Some(args[i].clone());
                }
                i += 1;
            }
        }
    }

    // Prefer positional_id over --id, but keep --id for compat
    if positional_id.is_some() && id.is_none() {
        id = positional_id.clone();
    } else if positional_id.is_some() && id.is_some() && positional_id != id {
        // Both provided but differ; use positional
        id = positional_id.clone();
    }

    let aimt_path = match crate::commands::project::resolve_aimt_project(aimt_path.as_deref()) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    };

    // Use the workflow engine and ReadWorkflow for read-only operations
    // This proves: OpenCode tool -> Rust bridge -> WorkflowEngine -> ReadWorkflow -> Store
    let result = match action {
        "search" => {
            let q = query.unwrap_or_default();
            handle_search(&aimt_path, &q)
        }
        "read" => {
            let id = id.unwrap_or_else(|| {
                eprintln!("error: --id is required for read action");
                std::process::exit(1);
            });
            handle_read(&aimt_path, &id)
        }
        "follow_parent" => {
            let id = id.unwrap_or_else(|| {
                eprintln!("error: --id is required for follow_parent action");
                std::process::exit(1);
            });
            handle_follow_parent(&aimt_path, &id)
        }
        "follow_file" => {
            let id = id.unwrap_or_else(|| {
                eprintln!("error: --id is required for follow_file action");
                std::process::exit(1);
            });
            handle_follow_file(&aimt_path, &id)
        }
        "follow_relation" => {
            let id = id.unwrap_or_else(|| {
                eprintln!("error: --id is required for follow_relation action");
                std::process::exit(1);
            });
            handle_follow_relation(&aimt_path, &id)
        }
        "validate" => handle_validate(&aimt_path),
        _ => {
            eprintln!("error: unknown action '{}'", action);
            print_aimt_help();
            std::process::exit(1);
        }
    };

    match result {
        Ok(json) => {
            println!("{}", json);
        }
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    }
}

fn handle_search(aimt_path: &std::path::Path, query: &str) -> Result<String, String> {
    let store = ReadWorkflow::new()
        .open(aimt_path)
        .map_err(|e| e.to_string())?;
    let wf = ReadWorkflow::new();
    let results = wf.search(&store, query);
    // Return structured JSON
    let mut out = String::from("{\"action\":\"search\",\"query\":");
    out.push_str(&format!("\"{}\"", query.replace('"', "\\\"")));
    out.push_str(",\"count\":");
    out.push_str(&results.len().to_string());
    out.push_str(",\"entities\":[");
    for (i, e) in results.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        let id = e.id().map(|id| id.as_str().to_string()).unwrap_or_default();
        let title = e
            .field("title")
            .map(|f| f.value.clone())
            .unwrap_or_default();
        let level = e.level.as_str();
        out.push_str(&format!(
            "{{\"id\":\"{}\",\"level\":\"{}\",\"title\":\"{}\"}}",
            id.replace('"', "\\\""),
            level,
            title.replace('"', "\\\"")
        ));
    }
    out.push_str("]}");
    Ok(out)
}

fn handle_read(aimt_path: &std::path::Path, id: &str) -> Result<String, String> {
    let store = ReadWorkflow::new()
        .open(aimt_path)
        .map_err(|e| e.to_string())?;
    let wf = ReadWorkflow::new();
    let entity = wf
        .read(&store, id)
        .ok_or_else(|| format!("entity not found: {}", id))?;
    let level = entity.level.as_str();
    let title = entity
        .field("title")
        .map(|f| f.value.clone())
        .unwrap_or_default();
    let mut fields_json = String::new();
    fields_json.push('[');
    let mut first = true;
    for f in entity.all_fields() {
        if !first {
            fields_json.push(',');
        }
        first = false;
        fields_json.push_str(&format!(
            "{{\"name\":\"{}\",\"value\":\"{}\"}}",
            f.name.replace('"', "\\\""),
            f.value.replace('"', "\\\"").replace('\n', "\\n")
        ));
    }
    fields_json.push(']');
    Ok(format!(
        "{{\"action\":\"read\",\"id\":\"{}\",\"level\":\"{}\",\"title\":\"{}\",\"fields\":{}}}",
        id.replace('"', "\\\""),
        level,
        title.replace('"', "\\\""),
        fields_json
    ))
}

fn handle_follow_parent(aimt_path: &std::path::Path, id: &str) -> Result<String, String> {
    let store = ReadWorkflow::new()
        .open(aimt_path)
        .map_err(|e| e.to_string())?;
    let wf = ReadWorkflow::new();
    let entity = wf
        .read(&store, id)
        .ok_or_else(|| format!("entity not found: {}", id))?;
    let parent = wf
        .follow_parent(&store, entity)
        .ok_or_else(|| format!("no parent for {}", id))?;
    let parent_id = parent
        .id()
        .map(|id| id.as_str().to_string())
        .unwrap_or_default();
    let level = parent.level.as_str();
    Ok(format!(
        "{{\"action\":\"follow_parent\",\"from\":\"{}\",\"to\":\"{}\",\"level\":\"{}\"}}",
        id.replace('"', "\\\""),
        parent_id.replace('"', "\\\""),
        level
    ))
}

fn handle_follow_file(aimt_path: &std::path::Path, id: &str) -> Result<String, String> {
    let store = ReadWorkflow::new()
        .open(aimt_path)
        .map_err(|e| e.to_string())?;
    let wf = ReadWorkflow::new();
    let entity = wf
        .read(&store, id)
        .ok_or_else(|| format!("entity not found: {}", id))?;
    let file = wf
        .follow_file(&store, entity)
        .ok_or_else(|| format!("no file for {}", id))?;
    let file_id = file
        .id()
        .map(|id| id.as_str().to_string())
        .unwrap_or_default();
    Ok(format!(
        "{{\"action\":\"follow_file\",\"from\":\"{}\",\"to\":\"{}\"}}",
        id.replace('"', "\\\""),
        file_id.replace('"', "\\\"")
    ))
}

fn handle_follow_relation(aimt_path: &std::path::Path, id: &str) -> Result<String, String> {
    let store = ReadWorkflow::new()
        .open(aimt_path)
        .map_err(|e| e.to_string())?;
    let wf = ReadWorkflow::new();
    let relations = wf.follow_relations(&store, id);
    let mut out = String::from("{\"action\":\"follow_relation\",\"from\":\"");
    out.push_str(&id.replace('"', "\\\""));
    out.push_str("\",\"relations\":[");
    for (i, e) in relations.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        let rid = e.id().map(|id| id.as_str().to_string()).unwrap_or_default();
        let level = e.level.as_str();
        out.push_str(&format!(
            "{{\"id\":\"{}\",\"level\":\"{}\"}}",
            rid.replace('"', "\\\""),
            level
        ));
    }
    out.push_str("]}");
    Ok(out)
}

fn handle_validate(aimt_path: &std::path::Path) -> Result<String, String> {
    let store = Store::open(aimt_path).map_err(|e| e.to_string())?;
    match store.validate() {
        Ok(()) => Ok(format!(
            "{{\"action\":\"validate\",\"valid\":true,\"count\":{}}}",
            store.len()
        )),
        Err(errs) => {
            let mut out = String::from("{\"action\":\"validate\",\"valid\":false,\"errors\":[");
            for (i, e) in errs.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&format!(
                    "{{\"id\":\"{}\",\"message\":\"{}\"}}",
                    e.id.replace('"', "\\\""),
                    e.message.replace('"', "\\\"")
                ));
            }
            out.push_str("]}");
            Ok(out)
        }
    }
}
