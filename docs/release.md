# Release

## Distribution

Published to [crates.io](https://crates.io/crates/petgraph-live).

```toml
[dependencies]
petgraph-live = "0.3"

# With snapshot support:
petgraph-live = { version = "0.3", features = ["snapshot"] }

# With zstd compression:
petgraph-live = { version = "0.3", features = ["snapshot-zstd"] }

# With LZ4 compression:
petgraph-live = { version = "0.3", features = ["snapshot-lz4"] }
```

## Branch Strategy

`main` is always releasable — tagged commits only. Feature work lands on a
`release/vX.Y.Z` integration branch, not directly on `main`.

```
feat/xxx  ─┐
feat/yyy  ─┼─▶  release/vX.Y.Z  ─▶  main  (tag vX.Y.Z)
feat/zzz  ─┘
```

1. Open `release/vX.Y.Z` from `main` at the start of the milestone.
2. Each `feat/...` PR targets `release/vX.Y.Z`, not `main`.
3. Run the pre-release checklist as commits on `release/vX.Y.Z`.
4. One final PR merges `release/vX.Y.Z` → `main`; tag on the merge commit.

Hotfixes branch from the relevant tag and merge back to both `main` and the
active `release/` branch if one is open.

## Pre-Release Checklist

### Feature matrix — all combinations must pass

```bash
cargo test
cargo test --features snapshot
cargo test --features snapshot-zstd
cargo test --features snapshot-lz4
cargo test --features snapshot-zstd,snapshot-lz4
```

### Code quality

- [ ] All tests pass (all three feature combinations above)
- [ ] Formatted: `cargo fmt -- --check`
- [ ] No lint issues: `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] Examples compile: `cargo build --examples --all-features`
- [ ] Doc tests pass: `cargo test --doc --all-features`

### Documentation

- [ ] All spec files in `docs/specifications/` have `status: implemented`
- [ ] `CHANGELOG.md` section dated and complete
- [ ] Public types have `///` rustdoc comments
- [ ] `docs/api-design.md` matches actual public API

### Version

- [ ] Version bumped in `Cargo.toml`
- [ ] `Cargo.lock` updated: `cargo update -p petgraph-live`

## Release

```bash
# 1. Bump version in Cargo.toml, update CHANGELOG date

# 2. Commit on release branch, push, open PR
git commit -am "chore: release vx.y.z"
git push origin release/vx.y.z
gh pr create --title "chore: release vx.y.z" --base main

# 3. Wait for PR CI to pass, then merge to main
git checkout main
git merge --no-ff release/vx.y.z

# 4. Tag the merge commit and push — GitHub Actions handles publish
git tag -a vx.y.z -m "Release vx.y.z"
git push origin main
git push origin vx.y.z
# GitHub Actions handles publish on tag push
```

Tags containing `-rc` (e.g. `v0.3.0-rc1`) follow the same steps but the
publish workflow will not trigger (RC tags are excluded).

## Hotfix

```bash
git checkout -b hotfix/vx.y.z+1 vx.y.z
# Apply fix, bump patch version in Cargo.toml
git commit -am "fix: description"
git tag -a vx.y.z+1 -m "Hotfix vx.y.z+1"
git push origin hotfix/vx.y.z+1 vx.y.z+1
# Merge back to main
git checkout main
git merge --no-ff hotfix/vx.y.z+1
git push origin main
```

## CHANGELOG Format

Move `[Unreleased]` entries to a versioned section:

```markdown
## [0.2.0] — 2026-MM-DD

### Added
- …

### Fixed
- …

## [Unreleased]
```
