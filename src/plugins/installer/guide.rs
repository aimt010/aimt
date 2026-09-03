use std::path::Path;

use super::aimt::MODULES;
use super::filesystem::{io_error, write_if_changed};
use super::report::{InstallError, InstallReport};
use super::template::{AIMT_HOST_GUIDE_MD, AIMT_MARKER};

pub fn ensure_aimt_prompts(
    project_dir: &Path,
    report: &mut InstallReport,
) -> Result<(), InstallError> {
    for (name, content) in MODULES {
        let dest = project_dir.join(".agents").join("aimt").join(name);
        write_if_changed(&dest, content, report)?;
    }
    // Ensure project ownership manifest after writing AIMT prompts
    sync_project_manifest_for_install(project_dir);
    Ok(())
}

fn sync_project_manifest_for_install(project_dir: &Path) {
    // Best-effort: create/update project manifest to reflect current MODULES hashes.
    // Failures are non-fatal for install; upgrade/uninstall will handle missing manifest safely.
    let version = env!("CARGO_PKG_VERSION").to_string();
    let mut files = std::collections::BTreeMap::new();
    for (name, content) in MODULES {
        let hash = crate::install::hash_content(content);
        files.insert(
            name.to_string(),
            crate::install::manifest::FileEntry { hash, owned: true },
        );
    }
    let manifest = crate::install::ProjectManifest {
        version,
        installed_at: {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                .to_string()
        },
        files,
    };
    let _ = crate::install::save_project_manifest(project_dir, &manifest);
    // Also ensure global manifest exists
    let _ = crate::install::ensure_global_manifest();
}

fn ensure_guide_file(
    project_dir: &Path,
    filename: &str,
    report: &mut InstallReport,
) -> Result<(), InstallError> {
    let guide_path = project_dir.join(filename);
    if guide_path.exists() {
        let existing =
            std::fs::read_to_string(&guide_path).map_err(|e| io_error(&guide_path, e))?;
        if existing.contains(AIMT_MARKER) {
            let has_minimal_index = existing.contains(".agents/aimt/index.md");
            let has_old_gate =
                existing.contains("before answering") || existing.contains("before reading source");
            let has_old_refs = existing.contains(".agents/aimt/README.md");
            let is_minimal_concise = existing.contains("primary knowledge map")
                && existing.contains("evidence")
                && (existing.contains("missing")
                    || existing.contains("stale")
                    || existing.contains("insufficient"));
            if has_minimal_index && !has_old_gate && !has_old_refs && is_minimal_concise {
                report.skipped.push(guide_path);
                return Ok(());
            }
            // Update to minimal host guide while preserving user content before marker
            let before = existing.split(AIMT_MARKER).next().unwrap_or(&existing);
            let new_content = format!("{}\n\n{}\n", before.trim_end(), AIMT_HOST_GUIDE_MD.trim());
            std::fs::write(&guide_path, &new_content).map_err(|e| io_error(&guide_path, e))?;
            report.updated.push(guide_path);
            return Ok(());
        }
        let new_content = format!("{}\n\n{}\n", existing.trim_end(), AIMT_HOST_GUIDE_MD);
        std::fs::write(&guide_path, &new_content).map_err(|e| io_error(&guide_path, e))?;
        report.updated.push(guide_path);
    } else {
        if let Some(parent) = guide_path.parent()
            && !parent.exists()
        {
            std::fs::create_dir_all(parent).map_err(|e| io_error(parent, e))?;
        }
        std::fs::write(&guide_path, format!("{}\n", AIMT_HOST_GUIDE_MD))
            .map_err(|e| io_error(&guide_path, e))?;
        report.created.push(guide_path);
    }
    Ok(())
}

pub fn ensure_guide(project_dir: &Path, report: &mut InstallReport) -> Result<(), InstallError> {
    ensure_aimt_prompts(project_dir, report)?;
    ensure_guide_file(project_dir, "AGENTS.md", report)
}

pub fn ensure_claude_guide(
    project_dir: &Path,
    report: &mut InstallReport,
) -> Result<(), InstallError> {
    ensure_aimt_prompts(project_dir, report)?;
    ensure_guide_file(project_dir, "CLAUDE.md", report)
}

pub fn ensure_copilot_guide(
    project_dir: &Path,
    report: &mut InstallReport,
) -> Result<(), InstallError> {
    ensure_aimt_prompts(project_dir, report)?;
    ensure_guide_file(project_dir, ".github/copilot-instructions.md", report)
}

pub fn ensure_gemini_guide(
    project_dir: &Path,
    report: &mut InstallReport,
) -> Result<(), InstallError> {
    ensure_aimt_prompts(project_dir, report)?;
    ensure_guide_file(project_dir, "GEMINI.md", report)
}
