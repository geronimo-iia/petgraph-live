# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `cache::GenerationCache<G>` — thread-safe generation-keyed graph cache with
  `get_or_build`, `invalidate`, `current_generation`; integration tests and
  `examples/cache_basic` included
- Design docs: `api-design.md`, `roadmap.md`, per-feature specifications
  (`spec-cache`, `spec-algorithms`, `spec-snapshot`, `spec-live`), and
  implementation plans for all four modules

