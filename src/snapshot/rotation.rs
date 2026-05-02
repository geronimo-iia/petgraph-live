use std::path::{Path, PathBuf};

use crate::snapshot::error::SnapshotError;

/// Return all snapshot files for `name` in `dir`, sorted ascending by mtime (oldest first).
pub fn list_snapshot_files(dir: &Path, name: &str) -> Result<Vec<PathBuf>, SnapshotError> {
    let prefix = format!("{}-", name);
    let extensions = [
        ".snap",
        ".snap.zst",
        ".snap.lz4",
        ".json",
        ".json.zst",
        ".json.lz4",
    ];

    let mut entries: Vec<(std::time::SystemTime, PathBuf)> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let path = e.path();
            if !path.is_file() {
                return None;
            }
            let fname = path.file_name()?.to_str()?.to_owned();
            if !fname.starts_with(&prefix) {
                return None;
            }
            let has_ext = extensions
                .iter()
                .any(|ext| fname.ends_with(ext) && !fname.ends_with(".tmp"));
            if !has_ext {
                return None;
            }
            let mtime = e.metadata().ok()?.modified().ok()?;
            Some((mtime, path))
        })
        .collect();

    entries.sort_by_key(|(mtime, _)| *mtime);
    Ok(entries.into_iter().map(|(_, p)| p).collect())
}

/// Keep the `n` newest snapshot files for `name` in `dir` by mtime; delete the rest.
pub fn keep_n(dir: &Path, name: &str, n: usize) -> Result<(), SnapshotError> {
    let files = list_snapshot_files(dir, name)?;
    if files.len() <= n {
        return Ok(());
    }
    let to_delete = files.len() - n;
    for path in files.into_iter().take(to_delete) {
        std::fs::remove_file(path)?;
    }
    Ok(())
}
