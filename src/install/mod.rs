pub mod manifest;
pub use manifest::{
    GlobalManifest, ProjectManifest, compare_versions, ensure_global_manifest,
    global_manifest_path, hash_content, is_compatible, load_global_manifest, load_project_manifest,
    normalize_version, project_manifest_any_exists, project_manifest_path, save_global_manifest,
    save_project_manifest,
};
