# Codebase Review (Blunt)

This review is intentionally direct. It focuses on bugs, dead/unused logic, non-idiomatic Rust, organization drift, and potential deprecation/legacy risks.

## Executive Summary

- The biggest problems are in the installer templates: shell and PowerShell behavior are inconsistent, and there are real logic bugs in `templates/install.sh` and major feature gaps in `templates/install.ps1`.
- The Rust code compiles, but it has accumulated dead code and “half-finished abstraction” drift (`Repo::Url`, `Repo::Python`, unused fields/functions).
- Tests are mixing integration/network concerns into normal test runs and are using a sync mutex across `await` points (clippy flagged).
- There are multiple avoidable panic points (`unwrap`) in response construction and parsing paths.
- No Rust compiler “deprecated API” warnings were emitted, but there is legacy pattern usage and stale metadata.

---

## High Severity Findings

### 1) `install.sh` contains real logic bugs in choice handling
- File: `templates/install.sh`
- Problems:
  - In `_ask_choices`, loop is `for c in $choice; do` but switch checks `case "$choice"` instead of `case "$c"`; this is wrong variable usage.
  - Executable count check uses `-e` instead of `-eq`:  
    `if [ "${#executable_files[@]}" -e 0 ]; then`
  - This is invalid numeric comparison and can break behavior.
- Impact:
  - User selection parsing is unreliable.
  - Archive extraction flow can fail unpredictably.

### 2) Debian installer path in `install.sh` is effectively broken by case mismatch
- File: `templates/install.sh`
- Problem:
  - It matches `"Deb installer"` in `case`, but Rust `Display` emits `"deb installer"` in lowercase (`src/domain/artifact.rs`).
- Impact:
  - Debian install branch is never hit; it falls into invalid type handling.

### 3) PowerShell installer is functionally incomplete compared to bash installer
- File: `templates/install.ps1`
- Problem:
  - Only `tar.gz` is handled in switch.
  - No handling for `binary`, `deb installer`, or other filetypes.
- Impact:
  - Windows users receive scripts that cannot handle many valid assets the backend can return.
  - This is behavioral inconsistency across platforms and likely a production bug.

### 4) Shell escaping strategy is wrong for PowerShell context
- Files: `src/templates.rs`, `templates/install.ps1`
- Problem:
  - Shared `escape_shell` filter uses Bash quoting (`shell_quote::Bash`) and is injected into PowerShell template assignments.
- Impact:
  - Quoting semantics are shell-specific; this is fragile and potentially unsafe for special characters.
  - At minimum this is non-idiomatic templating; at worst it’s malformed script output / injection surface.

---

## Medium Severity Findings

### 5) Dead code and abandoned abstraction in supported app/repo modeling
- File: `src/supported_apps.rs`
- Compiler/clippy warnings:
  - `source` field never read.
  - `Repo::Url` and `Repo::Python` variants never constructed.
  - `Repo::url`, `Repo::python`, and `Repo::get_download_link` never used.
- Impact:
  - Noise and maintenance burden.
  - Signals partially implemented direction that was abandoned.

### 6) Non-idiomatic API shape and avoidable allocations in provider logic
- File: `src/providers/gh.rs`
- Clippy findings:
  - `calc_all_widths(download_infos: &Vec<DownloadInfo>)` should take slice `&[DownloadInfo]`.
  - Needless late init for `release`.
- Additional issue:
  - Debug formatting builds many temporary strings purely for logs.
- Impact:
  - Unnecessary allocations and reduced readability in a hot path.

### 7) Test strategy is brittle and polluted with network dependence
- File: `src/providers/gh.rs` (tests)
- Problems:
  - “Sanity” tests hit live GitHub and external HashiCorp checkpoint APIs.
  - Sync `std::sync::MutexGuard` held across `await` points (`await_holding_lock` clippy warning).
  - Uses `eprintln!` and `panic!` retry loops in test code.
- Impact:
  - Flaky CI, non-deterministic failures, hard-to-debug test behavior.

### 8) Panic-prone response construction and parsing
- File: `src/http/responses.rs`
- Problems:
  - `filename.split('.').last().unwrap()` for shell detection.
  - `Response::builder().body(...).unwrap()`.
  - Clippy also flags `last()` on double-ended iterator (`next_back()` is preferable).
- Impact:
  - Panic surface in runtime path where robust fallback is expected.

---

## Low Severity / Organizational Drift

### 9) Unused imports/functions/fields in domain layer
- Files:
  - `src/domain/download.rs`: unused import `ArchiveType`; `Target::new` unused.
  - `src/domain/artifact.rs`: `ScriptType::identify` unused.
- Impact:
  - Signals either missing feature integration or dead code that should be removed.

### 10) Metadata/version drift
- Files:
  - `Cargo.toml` package version: `0.3.0`
  - `src/main.rs` OpenAPI annotation version: `0.4.0`
- Impact:
  - Docs/API metadata mismatch confuses users and downstream tooling.

### 11) Suspicious dead environment plumbing
- File: `src/main.rs`
- Problem:
  - `TERMLIBS_ROOT` is logged but otherwise unused in behavior.
- Impact:
  - Dead config surface; pointless noise in startup logs.

### 12) Style/idiom rough edges
- Files: `src/static_site.rs`, `src/main.rs`, others
- Examples:
  - Manual `match` returning `Option` in places where combinators are clearer.
  - Repeated `unwrap_or("".to_string())` style that can be cleaner and more explicit.
- Impact:
  - Readability debt; low but cumulative.

---

## Deprecated Usage Check

- Rust build + clippy did **not** report direct deprecated Rust API usage.
- However, there are legacy patterns and stale abstractions that should be treated as technical debt before they become breakage.

---

## What To Fix Next (Plan)

### Phase 1 — Stop behavioral bugs (highest priority)
1. Fix `templates/install.sh` selection logic (`$c` vs `$choice`, `-eq` check, Debian type string consistency).
2. Bring `templates/install.ps1` to feature parity with bash path for `binary` + installer flows.
3. Replace shared Bash escaping for PowerShell with a PowerShell-specific escaping/filter strategy.

### Phase 2 — Stabilize API/runtime correctness
1. Remove panic paths in `ScriptResponse` (`unwrap` usage) and return safe fallbacks/errors.
2. Tighten file extension detection (`next_back` or explicit parser) and handle missing extension safely.
3. Clean up provider log formatting and avoid excessive allocation-heavy debug plumbing.

### Phase 3 — Kill dead code and simplify model
1. Either fully implement or remove `Repo::Url` / `Repo::Python` and related methods.
2. Remove unused fields/imports/functions (`source`, `Target::new`, `ScriptType::identify`, etc.).
3. Align OpenAPI version metadata with `Cargo.toml` versioning strategy.

### Phase 4 — Fix test architecture
1. Move network sanity tests behind ignored/integration gates (or separate suite).
2. Stop holding sync mutex guards across `await`; use async-aware sync or redesign sequencing.
3. Replace panic-heavy retries with deterministic test helpers and clear assertion boundaries.

### Phase 5 — Idiomatic polish (final pass)
1. Apply clippy suggestions (`&[T]`, needless late init, iterator improvement).
2. Refactor minor Option/string handling to reduce noise and improve readability.
3. Re-run `cargo clippy --all-targets --all-features` until warning-free or with explicit justified allows.
