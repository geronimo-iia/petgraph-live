use std::path::PathBuf;

use serde::{Serialize, de::DeserializeOwned};

use crate::snapshot::{
    config::{Compression, SnapshotConfig, SnapshotFormat, sanitize_key},
    error::SnapshotError,
    meta::SnapshotMeta,
    rotation,
};

fn extension(format: &SnapshotFormat, compression: &Compression) -> &'static str {
    match (format, compression) {
        (SnapshotFormat::Bincode, Compression::None) => ".snap",
        (SnapshotFormat::Json, Compression::None) => ".json",
        #[cfg(feature = "snapshot-zstd")]
        (SnapshotFormat::Bincode, Compression::Zstd { .. }) => ".snap.zst",
        #[cfg(feature = "snapshot-zstd")]
        (SnapshotFormat::Json, Compression::Zstd { .. }) => ".json.zst",
    }
}

fn snapshot_path(cfg: &SnapshotConfig, sanitized: &str) -> PathBuf {
    let ext = extension(&cfg.format, &cfg.compression);
    cfg.dir.join(format!("{}-{}{}", cfg.name, sanitized, ext))
}

/// Serialize `graph` to `{cfg.dir}/{cfg.name}-{sanitized_key}.{ext}`.
///
/// Writes atomically (temp file + rename). After writing, the oldest snapshots
/// beyond `cfg.keep` are deleted by mtime.
///
/// # Errors
///
/// Returns [`SnapshotError::InvalidKey`] if `cfg.key` is `None` or sanitizes to empty.
///
/// # Examples
///
/// ```rust
/// # use petgraph::Graph;
/// # use petgraph_live::snapshot::{Compression, SnapshotConfig, SnapshotFormat, save};
/// # let dir = tempfile::tempdir().unwrap();
/// let cfg = SnapshotConfig {
///     dir: dir.path().to_path_buf(),
///     name: "g".into(),
///     key: Some("v1".into()),
///     format: SnapshotFormat::Bincode,
///     compression: Compression::None,
///     keep: 3,
/// };
/// let mut g: Graph<String, ()> = Graph::new();
/// g.add_node("A".into());
/// save(&cfg, &g)?;
/// # Ok::<_, petgraph_live::snapshot::SnapshotError>(())
/// ```
pub fn save<G>(cfg: &SnapshotConfig, graph: &G) -> Result<(), SnapshotError>
where
    G: Serialize + petgraph::visit::NodeCount + petgraph::visit::EdgeCount,
{
    let key = cfg
        .key
        .as_deref()
        .ok_or_else(|| SnapshotError::InvalidKey("None key in save".into()))?;
    let sanitized = sanitize_key(key)?;

    let meta = SnapshotMeta::new(
        key,
        cfg.format.clone(),
        cfg.compression.clone(),
        graph.node_count(),
        graph.edge_count(),
    );

    let bytes = serialize_graph(cfg, &meta, graph)?;
    let bytes = compress(cfg, bytes)?;

    let final_path = snapshot_path(cfg, &sanitized);
    let tmp_path = PathBuf::from(format!("{}.tmp", final_path.to_string_lossy()));

    std::fs::write(&tmp_path, &bytes)?;
    std::fs::rename(&tmp_path, &final_path)?;

    rotation::keep_n(&cfg.dir, &cfg.name, cfg.keep)?;
    Ok(())
}

/// Like [`save`] but does not require [`NodeCount`]/[`EdgeCount`]; stores 0 for both counts.
/// Used internally by `GraphState` which may hold arbitrary graph types.
pub(crate) fn save_any<G: Serialize>(cfg: &SnapshotConfig, graph: &G) -> Result<(), SnapshotError> {
    let key = cfg
        .key
        .as_deref()
        .ok_or_else(|| SnapshotError::InvalidKey("None key in save".into()))?;
    let sanitized = sanitize_key(key)?;

    let meta = SnapshotMeta::new(key, cfg.format.clone(), cfg.compression.clone(), 0, 0);

    let bytes = serialize_graph(cfg, &meta, graph)?;
    let bytes = compress(cfg, bytes)?;

    let final_path = snapshot_path(cfg, &sanitized);
    let tmp_path = PathBuf::from(format!("{}.tmp", final_path.to_string_lossy()));

    std::fs::write(&tmp_path, &bytes)?;
    std::fs::rename(&tmp_path, &final_path)?;

    rotation::keep_n(&cfg.dir, &cfg.name, cfg.keep)?;
    Ok(())
}

fn serialize_graph<G: Serialize>(
    cfg: &SnapshotConfig,
    meta: &SnapshotMeta,
    graph: &G,
) -> Result<Vec<u8>, SnapshotError> {
    match cfg.format {
        SnapshotFormat::Bincode => {
            let meta_bytes = bincode::serde::encode_to_vec(meta, bincode::config::standard())
                .map_err(|e| SnapshotError::ParseError(e.to_string()))?;
            let graph_bytes = bincode::serde::encode_to_vec(graph, bincode::config::standard())
                .map_err(|e| SnapshotError::ParseError(e.to_string()))?;
            let meta_len = meta_bytes.len() as u64;
            let mut out = Vec::with_capacity(8 + meta_bytes.len() + graph_bytes.len());
            out.extend_from_slice(&meta_len.to_le_bytes());
            out.extend_from_slice(&meta_bytes);
            out.extend_from_slice(&graph_bytes);
            Ok(out)
        }
        SnapshotFormat::Json => {
            let val = serde_json::json!({"meta": meta, "graph": graph});
            serde_json::to_vec(&val).map_err(|e| SnapshotError::ParseError(e.to_string()))
        }
    }
}

fn compress(cfg: &SnapshotConfig, bytes: Vec<u8>) -> Result<Vec<u8>, SnapshotError> {
    match &cfg.compression {
        Compression::None => Ok(bytes),
        #[cfg(feature = "snapshot-zstd")]
        Compression::Zstd { level } => zstd::encode_all(std::io::Cursor::new(bytes), *level)
            .map_err(|e| SnapshotError::CompressionError(e.to_string())),
    }
}

fn decompress(path: &std::path::Path, bytes: Vec<u8>) -> Result<Vec<u8>, SnapshotError> {
    if path.to_string_lossy().ends_with(".zst") {
        #[cfg(feature = "snapshot-zstd")]
        {
            return zstd::decode_all(std::io::Cursor::new(bytes))
                .map_err(|e| SnapshotError::CompressionError(e.to_string()));
        }
        #[cfg(not(feature = "snapshot-zstd"))]
        return Err(SnapshotError::CompressionError(
            "zstd feature not enabled".into(),
        ));
    }
    Ok(bytes)
}

#[cfg(feature = "snapshot-zstd")]
const SNAP_EXTENSIONS: &[&str] = &[".snap", ".json", ".snap.zst", ".json.zst"];
#[cfg(not(feature = "snapshot-zstd"))]
const SNAP_EXTENSIONS: &[&str] = &[".snap", ".json"];

fn find_snapshot_file(cfg: &SnapshotConfig) -> Result<Option<PathBuf>, SnapshotError> {
    if let Some(key) = &cfg.key {
        let sanitized = sanitize_key(key)?;
        for ext in SNAP_EXTENSIONS {
            let path = cfg.dir.join(format!("{}-{}{}", cfg.name, sanitized, ext));
            if path.exists() {
                return Ok(Some(path));
            }
        }
        Err(SnapshotError::KeyNotFound {
            key: key.to_string(),
        })
    } else {
        let files = rotation::list_snapshot_files(&cfg.dir, &cfg.name)?;
        Ok(files.into_iter().last())
    }
}

fn read_meta_from_bytes(
    path: &std::path::Path,
    bytes: &[u8],
) -> Result<SnapshotMeta, SnapshotError> {
    let pname = path.to_string_lossy();
    if pname.contains(".json") {
        let val: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|e| SnapshotError::ParseError(e.to_string()))?;
        serde_json::from_value(
            val.get("meta")
                .ok_or_else(|| SnapshotError::ParseError("missing 'meta' field".into()))?
                .clone(),
        )
        .map_err(|e| SnapshotError::ParseError(e.to_string()))
    } else {
        if bytes.len() < 8 {
            return Err(SnapshotError::ParseError("file too short".into()));
        }
        let meta_len = u64::from_le_bytes(bytes[..8].try_into().unwrap()) as usize;
        if bytes.len() < 8 + meta_len {
            return Err(SnapshotError::ParseError("file truncated".into()));
        }
        let (meta, _) = bincode::serde::decode_from_slice::<SnapshotMeta, _>(
            &bytes[8..8 + meta_len],
            bincode::config::standard(),
        )
        .map_err(|e| SnapshotError::ParseError(e.to_string()))?;
        Ok(meta)
    }
}

/// Deserialize and return the snapshot matching `cfg.key`, or the most recent
/// snapshot when `cfg.key` is `None`.
///
/// Returns `Ok(None)` when no matching file exists (key absent or directory empty).
///
/// # Examples
///
/// ```rust
/// # use petgraph::Graph;
/// # use petgraph_live::snapshot::{Compression, SnapshotConfig, SnapshotFormat, save, load};
/// # let dir = tempfile::tempdir().unwrap();
/// # let cfg = SnapshotConfig {
/// #     dir: dir.path().to_path_buf(),
/// #     name: "g".into(),
/// #     key: Some("v1".into()),
/// #     format: SnapshotFormat::Bincode,
/// #     compression: Compression::None,
/// #     keep: 3,
/// # };
/// # let mut g: Graph<String, ()> = Graph::new();
/// # g.add_node("A".into());
/// # save(&cfg, &g).unwrap();
/// let loaded: Option<Graph<String, ()>> = load(&cfg)?;
/// assert_eq!(loaded.unwrap().node_count(), 1);
/// # Ok::<_, petgraph_live::snapshot::SnapshotError>(())
/// ```
pub fn load<G>(cfg: &SnapshotConfig) -> Result<Option<G>, SnapshotError>
where
    G: DeserializeOwned,
{
    let path = match find_snapshot_file(cfg)? {
        Some(p) => p,
        None => return Ok(None),
    };

    let raw = std::fs::read(&path)?;
    let bytes = decompress(&path, raw)?;
    let pname = path.to_string_lossy();

    let graph = if pname.contains(".json") {
        let val: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|e| SnapshotError::ParseError(e.to_string()))?;
        serde_json::from_value(
            val.get("graph")
                .ok_or_else(|| SnapshotError::ParseError("missing 'graph' field".into()))?
                .clone(),
        )
        .map_err(|e| SnapshotError::ParseError(e.to_string()))?
    } else {
        if bytes.len() < 8 {
            return Err(SnapshotError::ParseError("file too short".into()));
        }
        let meta_len = u64::from_le_bytes(bytes[..8].try_into().unwrap()) as usize;
        let graph_start = 8 + meta_len;
        if bytes.len() < graph_start {
            return Err(SnapshotError::ParseError("file truncated".into()));
        }
        let (graph, _) = bincode::serde::decode_from_slice::<G, _>(
            &bytes[graph_start..],
            bincode::config::standard(),
        )
        .map_err(|e| SnapshotError::ParseError(e.to_string()))?;
        graph
    };

    Ok(Some(graph))
}

/// Load the matching snapshot, or call `build` and save the result if absent.
///
/// Falls back to `build` on [`SnapshotError::KeyNotFound`] and
/// [`SnapshotError::NoSnapshotFound`]. Save failures are logged to stderr but
/// do not propagate — the freshly built graph is still returned.
///
/// # Examples
///
/// ```rust
/// # use petgraph::Graph;
/// # use petgraph_live::snapshot::{Compression, SnapshotConfig, SnapshotFormat, load_or_build};
/// # let dir = tempfile::tempdir().unwrap();
/// let cfg = SnapshotConfig {
///     dir: dir.path().to_path_buf(),
///     name: "g".into(),
///     key: Some("v1".into()),
///     format: SnapshotFormat::Bincode,
///     compression: Compression::None,
///     keep: 3,
/// };
/// let mut calls = 0u32;
/// let graph: Graph<String, ()> = load_or_build(&cfg, || {
///     calls += 1;
///     let mut g: Graph<String, ()> = Graph::new();
///     g.add_node("A".into());
///     Ok(g)
/// })?;
/// assert_eq!(calls, 1);
/// assert_eq!(graph.node_count(), 1);
/// # Ok::<_, petgraph_live::snapshot::SnapshotError>(())
/// ```
pub fn load_or_build<G, F>(cfg: &SnapshotConfig, build: F) -> Result<G, SnapshotError>
where
    G: Serialize + DeserializeOwned + petgraph::visit::NodeCount + petgraph::visit::EdgeCount,
    F: FnOnce() -> Result<G, SnapshotError>,
{
    match load(cfg) {
        Ok(Some(g)) => Ok(g),
        Ok(None) | Err(SnapshotError::KeyNotFound { .. }) | Err(SnapshotError::NoSnapshotFound) => {
            let g = build()?;
            if let Err(e) = save(cfg, &g) {
                eprintln!("warn: snapshot save failed: {}", e);
            }
            Ok(g)
        }
        Err(e) => Err(e),
    }
}

/// Read snapshot metadata without deserializing the graph.
///
/// For bincode files only the length-prefixed header bytes are parsed.
/// For JSON files only the `"meta"` field is extracted.
/// Returns `Ok(None)` when no matching file exists.
///
/// # Examples
///
/// ```rust
/// # use petgraph::Graph;
/// # use petgraph_live::snapshot::{Compression, SnapshotConfig, SnapshotFormat, save, inspect};
/// # let dir = tempfile::tempdir().unwrap();
/// # let cfg = SnapshotConfig {
/// #     dir: dir.path().to_path_buf(),
/// #     name: "g".into(),
/// #     key: Some("v1".into()),
/// #     format: SnapshotFormat::Bincode,
/// #     compression: Compression::None,
/// #     keep: 3,
/// # };
/// # let mut g: Graph<String, ()> = Graph::new();
/// # g.add_node("A".into());
/// # save(&cfg, &g).unwrap();
/// let meta = inspect(&cfg)?.expect("meta present");
/// assert_eq!(meta.node_count, 1);
/// assert_eq!(meta.key, "v1");
/// # Ok::<_, petgraph_live::snapshot::SnapshotError>(())
/// ```
pub fn inspect(cfg: &SnapshotConfig) -> Result<Option<SnapshotMeta>, SnapshotError> {
    let path = match find_snapshot_file(cfg)? {
        Some(p) => p,
        None => return Ok(None),
    };

    let raw = std::fs::read(&path)?;
    let bytes = decompress(&path, raw)?;
    Ok(Some(read_meta_from_bytes(&path, &bytes)?))
}

/// Return all snapshots in `cfg.dir` matching `cfg.name`, ordered oldest first by mtime.
///
/// Each entry is `(path, meta)`. The key is `None` in `cfg` for listing all files.
///
/// # Examples
///
/// ```rust
/// # use petgraph::Graph;
/// # use petgraph_live::snapshot::{Compression, SnapshotConfig, SnapshotFormat, save, list};
/// # let dir = tempfile::tempdir().unwrap();
/// # let mut cfg = SnapshotConfig {
/// #     dir: dir.path().to_path_buf(),
/// #     name: "g".into(),
/// #     key: Some("v1".into()),
/// #     format: SnapshotFormat::Bincode,
/// #     compression: Compression::None,
/// #     keep: 5,
/// # };
/// # let mut g: Graph<String, ()> = Graph::new();
/// # g.add_node("A".into());
/// # save(&cfg, &g).unwrap();
/// # cfg.key = Some("v2".into());
/// # save(&cfg, &g).unwrap();
/// let cfg_all = SnapshotConfig { key: None, ..cfg };
/// let entries = list(&cfg_all)?;
/// assert_eq!(entries.len(), 2);
/// # Ok::<_, petgraph_live::snapshot::SnapshotError>(())
/// ```
pub fn list(cfg: &SnapshotConfig) -> Result<Vec<(PathBuf, SnapshotMeta)>, SnapshotError> {
    let files = rotation::list_snapshot_files(&cfg.dir, &cfg.name)?;
    let mut result = Vec::new();
    for path in files {
        let raw = std::fs::read(&path)?;
        let bytes = decompress(&path, raw)?;
        let meta = read_meta_from_bytes(&path, &bytes)?;
        result.push((path, meta));
    }
    Ok(result)
}

/// Delete all snapshot files in `cfg.dir` matching `cfg.name`. Returns the count deleted.
///
/// # Examples
///
/// ```rust
/// # use petgraph::Graph;
/// # use petgraph_live::snapshot::{Compression, SnapshotConfig, SnapshotFormat, save, purge};
/// # let dir = tempfile::tempdir().unwrap();
/// # let mut cfg = SnapshotConfig {
/// #     dir: dir.path().to_path_buf(),
/// #     name: "g".into(),
/// #     key: Some("v1".into()),
/// #     format: SnapshotFormat::Bincode,
/// #     compression: Compression::None,
/// #     keep: 5,
/// # };
/// # let mut g: Graph<String, ()> = Graph::new();
/// # g.add_node("A".into());
/// # save(&cfg, &g).unwrap();
/// # cfg.key = Some("v2".into());
/// # save(&cfg, &g).unwrap();
/// let cfg_all = SnapshotConfig { key: None, ..cfg };
/// let deleted = purge(&cfg_all)?;
/// assert_eq!(deleted, 2);
/// # Ok::<_, petgraph_live::snapshot::SnapshotError>(())
/// ```
pub fn purge(cfg: &SnapshotConfig) -> Result<usize, SnapshotError> {
    let files = rotation::list_snapshot_files(&cfg.dir, &cfg.name)?;
    let count = files.len();
    for path in files {
        std::fs::remove_file(path)?;
    }
    Ok(count)
}
