use std::path::Path;

use super::report::{InstallError, InstallReport, IoErrorDetails};

fn io_err(path: &Path, e: std::io::Error) -> InstallError {
    InstallError::Io(IoErrorDetails {
        path: path.to_path_buf(),
        kind: e.kind(),
        message: e.to_string(),
    })
}

pub fn write_if_changed(
    path: &Path,
    content: &str,
    report: &mut InstallReport,
) -> Result<(), InstallError> {
    if path.exists() {
        let existing = std::fs::read_to_string(path).map_err(|e| io_err(path, e))?;
        if existing == content {
            report.skipped.push(path.to_path_buf());
            return Ok(());
        }
        std::fs::write(path, content).map_err(|e| io_err(path, e))?;
        report.updated.push(path.to_path_buf());
    } else {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| io_err(parent, e))?;
        }
        std::fs::write(path, content).map_err(|e| io_err(path, e))?;
        report.created.push(path.to_path_buf());
    }
    Ok(())
}

pub fn io_error(path: &Path, err: std::io::Error) -> InstallError {
    io_err(path, err)
}

pub fn validate_project_dir(project_dir: &Path) -> Result<(), InstallError> {
    if !project_dir.exists() {
        return Err(InstallError::InvalidProject(format!(
            "project directory does not exist: {}",
            project_dir.display()
        )));
    }
    if !project_dir.is_dir() {
        return Err(InstallError::InvalidProject(format!(
            "project path is not a directory: {}",
            project_dir.display()
        )));
    }
    Ok(())
}

pub fn ensure_parent_exists(path: &Path) -> Result<(), InstallError> {
    if let Some(parent) = path.parent()
        && !parent.exists()
    {
        std::fs::create_dir_all(parent).map_err(|e| io_err(parent, e))?;
    }
    Ok(())
}
