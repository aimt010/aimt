use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};

/// Discover all `.aimt` files in the given directory (non-recursive, sorted).
/// Only regular files with extension `.aimt` (case-sensitive), not hidden, not symlinks.
pub fn discover_aimt_projects(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return out,
    };
    for entry in entries.flatten() {
        let Ok(ft) = entry.file_type() else {
            continue;
        };
        if ft.is_symlink() {
            continue;
        }
        if !ft.is_file() {
            continue;
        }
        let name = entry.file_name();
        let s = name.to_string_lossy();
        if s.starts_with('.') {
            continue;
        }
        let p = entry.path();
        if p.extension().and_then(|e| e.to_str()) == Some("aimt") {
            out.push(p);
        }
    }
    out.sort();
    out
}

/// Normalize a project name/path to exactly one `.aimt` extension.
/// Accepts `demo`, `demo.aimt`, `./path/demo`, `./path/demo.aimt`, etc.
/// Never produces `demo.aimt.aimt`.
pub fn normalize_project_name(input: &str) -> PathBuf {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return PathBuf::from(trimmed);
    }
    // Handle directory prefix manually to preserve it.
    let path = Path::new(trimmed);
    let parent = path.parent();
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or(trimmed);

    let mut name = file_name.to_string();
    // Strip all trailing ".aimt" then add one
    while name.to_lowercase().ends_with(".aimt") {
        // Only strip if lowercase matches ".aimt" suffix case-insensitively?
        // We want to treat "demo.AIMT" as needing normalization too, but we keep case of base.
        // For simplicity, check case-insensitive suffix.
        let lower = name.to_lowercase();
        if lower.ends_with(".aimt") {
            // Remove last 5 chars from original name (preserve original case of base)
            name.truncate(name.len() - 5);
            // If we stripped ".aimt" and now ends with ".", keep stripping?
            // Actually we want exactly one .aimt, so we loop to remove double.
            // Continue loop to handle "demo.aimt.aimt" -> "demo"
            if name.is_empty() {
                break;
            }
        } else {
            break;
        }
        // If original was "demo.aimt", first iteration gives "demo", second would not end with .aimt, break.
        if !name.to_lowercase().ends_with(".aimt") {
            break;
        }
    }
    // If we stripped everything (input was ".aimt"), fallback to "demo.aimt"? Keep as ".aimt"
    if name.is_empty() {
        name = "demo".to_string();
    }
    name.push_str(".aimt");
    if let Some(parent) = parent {
        if parent.as_os_str().is_empty() || parent == Path::new(".") {
            PathBuf::from(name)
        } else {
            parent.join(name)
        }
    } else {
        PathBuf::from(name)
    }
}

fn is_interactive() -> bool {
    io::stdin().is_terminal() && io::stdout().is_terminal()
}

fn prompt_select(projects: &[PathBuf]) -> Result<PathBuf, String> {
    if !is_interactive() {
        return Err("Multiple AIMT projects found. Please specify a project path.".to_string());
    }
    println!("Multiple AIMT projects found:\n");
    for (i, p) in projects.iter().enumerate() {
        println!("  {}. {}", i + 1, p.display());
    }
    println!();
    print!("Select a project [1-{}]: ", projects.len());
    io::stdout().flush().ok();
    let mut line = String::new();
    if io::stdin().read_line(&mut line).is_err() {
        return Err("Failed to read selection".to_string());
    }
    let trimmed = line.trim();
    let idx: usize = trimmed.parse().map_err(|_| {
        format!(
            "Invalid selection '{}'. Please enter a number between 1 and {}.",
            trimmed,
            projects.len()
        )
    })?;
    if idx == 0 || idx > projects.len() {
        return Err(format!(
            "Selection out of range. Please enter a number between 1 and {}.",
            projects.len()
        ));
    }
    Ok(projects[idx - 1].clone())
}

fn prompt_project_name() -> Result<PathBuf, String> {
    if !is_interactive() {
        return Err(
            "No AIMT project found in the current directory.\n\nCreate one with:\n\n  aimt init <name>\n\nExample:\n\n  aimt init demo.aimt".to_string()
        );
    }
    print!("Project name: ");
    io::stdout().flush().ok();
    let mut line = String::new();
    if io::stdin().read_line(&mut line).is_err() {
        return Err("Failed to read project name".to_string());
    }
    let trimmed = line.trim().to_string();
    if trimmed.is_empty() {
        return Err("Project name must not be empty".to_string());
    }
    Ok(normalize_project_name(&trimmed))
}

/// Central resolver: explicit path always wins, otherwise discover in current dir.
pub fn resolve_aimt_project(explicit: Option<&Path>) -> Result<PathBuf, String> {
    if let Some(p) = explicit {
        if p.as_os_str().is_empty() {
            return Err("Project path must not be empty".to_string());
        }
        if !p.exists() {
            return Err(format!(
                "AIMT project not found: {}\n\nPlease check the path or create a new project with:\n  aimt init <name>",
                p.display()
            ));
        }
        if p.is_dir() {
            return Err(format!(
                "Project path is a directory, expected .aimt file: {}",
                p.display()
            ));
        }
        return Ok(p.to_path_buf());
    }

    let projects = discover_aimt_projects(Path::new("."));
    match projects.len() {
        0 => Err(
            "No AIMT project found in the current directory.\n\nCreate one with:\n\n  aimt init <name>\n\nExample:\n\n  aimt init demo.aimt".to_string()
        ),
        1 => Ok(projects.into_iter().next().unwrap()),
        _ => prompt_select(&projects),
    }
}

/// For `init` when user may have omitted path, prompt for name.
/// If explicit is Some, normalize it; if None, prompt.
pub fn resolve_init_project(explicit: Option<&str>) -> Result<PathBuf, String> {
    if let Some(s) = explicit {
        if s.trim().is_empty() {
            return Err("Project name must not be empty".to_string());
        }
        Ok(normalize_project_name(s))
    } else {
        prompt_project_name()
    }
}

/// Returns true if `s` looks like a project path (ends with `.aimt`, not a flag).
pub fn is_project_arg(s: &str) -> bool {
    !s.starts_with('-') && s.ends_with(".aimt")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_adds_extension() {
        assert_eq!(normalize_project_name("demo"), PathBuf::from("demo.aimt"));
    }
    #[test]
    fn normalize_keeps_single() {
        assert_eq!(
            normalize_project_name("demo.aimt"),
            PathBuf::from("demo.aimt")
        );
    }
    #[test]
    fn normalize_strips_double() {
        assert_eq!(
            normalize_project_name("demo.aimt.aimt"),
            PathBuf::from("demo.aimt")
        );
    }
    #[test]
    fn normalize_with_dir() {
        assert_eq!(
            normalize_project_name("path/to/demo"),
            PathBuf::from("path/to/demo.aimt")
        );
        assert_eq!(
            normalize_project_name("path/to/demo.aimt"),
            PathBuf::from("path/to/demo.aimt")
        );
    }
    #[test]
    fn discover_sorts() {
        // Just test that function doesn't panic on missing dir
        let v = discover_aimt_projects(Path::new("/nonexistent_dir_for_test"));
        assert!(v.is_empty());
    }
}
