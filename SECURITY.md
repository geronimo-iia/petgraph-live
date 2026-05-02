# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.3.x   | ✓         |

## Reporting a Vulnerability

Please report security vulnerabilities by email to <jguibert@gmail.com>.

**Do not open a public issue.**

You should receive an acknowledgment within 48 hours. A fix will be
prioritized based on severity and released as a patch version.

## Scope

`petgraph-live` is a pure library crate. It does not make network requests,
run user-supplied code as subprocesses, or handle authentication. The main
attack surface is:

- **Snapshot deserialization** — untrusted `.snap` or `.snap.zst` files fed
  to `snapshot::load`. A malicious file could trigger unexpected behaviour
  in `bincode` or `serde` deserialization of the caller-defined graph type `G`.
  Validate snapshot files before loading from untrusted sources.

- **File-system paths** — `SnapshotConfig::dir` and key-derived filenames
  are not sanitized against path traversal beyond the key sanitizer
  (`[a-zA-Z0-9_.-]` only). Do not construct keys from untrusted user input.

- **Dependencies** — `bincode`, `serde`, `zstd` (optional). Dependency
  vulnerabilities are tracked via `cargo audit` in CI.

Out of scope: network exposure, privilege escalation, credential handling
(this crate has none).
