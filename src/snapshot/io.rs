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

pub fn inspect(cfg: &SnapshotConfig) -> Result<Option<SnapshotMeta>, SnapshotError> {
    let path = match find_snapshot_file(cfg)? {
        Some(p) => p,
        None => return Ok(None),
    };

    let raw = std::fs::read(&path)?;
    let bytes = decompress(&path, raw)?;
    Ok(Some(read_meta_from_bytes(&path, &bytes)?))
}

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

pub fn purge(cfg: &SnapshotConfig) -> Result<usize, SnapshotError> {
    let files = rotation::list_snapshot_files(&cfg.dir, &cfg.name)?;
    let count = files.len();
    for path in files {
        std::fs::remove_file(path)?;
    }
    Ok(count)
}
