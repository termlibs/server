# Termlibs Server Refactoring Plan

## Scope
- This plan covers the Rust server codebase under `src/`.
- Per request, `termlibs` submodules are excluded.
- This file focuses only on refactoring strategy and architecture.
- Bug findings are tracked separately in `BUG_AUDIT.md`.

## Executive Summary
The current codebase has significant structural issues: oversized multi-purpose modules, mixed responsibilities across layers, duplicated route logic, and inconsistent error handling. Several panic paths can crash request handling in production.

Top priorities:
1. Introduce clear module boundaries (domain, services, http, infra).
2. Eliminate panic-based control flow in request path.
3. Extract business logic out of `main.rs` handlers.
4. Consolidate duplicated install-script rendering flow.
5. Resolve API contract drift between modeled query options and actual behavior.
6. Separate startup hardening from request-path hardening.

---

## Findings: Refactoring Targets

### 1) Oversized and mixed-responsibility module
- File: `src/app_downloader.rs`
- Issue: ~743 lines combining:
  - platform detection (`TargetOs`, `TargetArch`, `TargetDeployment`)
  - file-type detection (`ArchiveType`, `InstallerType`, `ScriptType`, `Filetype`)
  - target inference (`Target`)
- Impact: weak cohesion, hard-to-test logic, high change risk.

### 2) Type ownership and layering are blurred
- File: `src/types.rs:1`
- Issue: re-exporting domain types (`TargetArch`, `TargetOs`) from another module while also defining HTTP response wrappers (`ScriptResponse`, `StringList`) and query options.
- Impact: unclear ownership and cross-module coupling.

### 3) Route handlers contain business logic
- File: `src/main.rs`
- Issue: handlers do orchestration, transformation, target resolution, GitHub calls, and template decisions directly.
- Impact: low testability; web concerns and domain concerns are tightly coupled.

### 4) Duplicate handler logic
- File: `src/main.rs:95-109`, `src/main.rs:139-153`
- Issue: nearly identical template context creation and script selection in two install handlers.
- Impact: drift risk and unnecessary maintenance cost.

### 5) Template setup concerns
- File: `src/templates.rs`
- Issues:
  - logs full template content on startup (`info!("Content: {}", content)`)
  - mixed error style (`unwrap` in one add call, ignored result in another)
  - unfinished `Powershell` quoting API with `todo!()`
- Impact: noisy logs, inconsistent initialization reliability, latent panic risk.

### 6) Static-site loading is rebuilt per request
- File: `src/static_site.rs`
- Issue: `BTreeMap::from(STATIC_FILES)` rebuilt on each call.
- Impact: minor inefficiency and avoidable allocation churn.

### 7) Dead or misleading abstractions remain in active modules
- Files: `src/types.rs`, `src/main.rs`
- Issues:
  - `InstallQueryOptions::template_globals()` exists but is not used by handlers.
  - `QueryOptions` trait is effectively a placeholder with non-functional implementation.
  - `StringList` response type appears unused in routing layer.
  - `TERMLIBS_ROOT` is logged but otherwise not part of runtime behavior.
- Impact: increases cognitive load and obscures the true execution path.

### 8) Operational and security defaults are too loose
- File: `src/main.rs`
- Issues:
  - startup/bind/serve paths rely on `unwrap` and can abort abruptly on config errors.
  - CORS policy is fully permissive (`CorsLayer::permissive()`).
- Impact: weaker operational resilience and future security risk as API surface grows.

---

## Target Architecture (Refactor End State)

Proposed layout:

```text
src/
  main.rs                 # bootstrap only
  error.rs                # AppError + IntoResponse mapping
  domain/
    mod.rs
    platform.rs           # TargetOs/TargetArch/TargetDeployment
    artifact.rs           # Filetype + archive/installer/script enums
    app.rs                # SupportedApp, Repo
    download.rs           # DownloadInfo, Target
  services/
    mod.rs
    github.rs             # release fetching/filtering
    installer.rs          # use-case orchestration (load + render)
    templating.rs         # tera context + script selection
  http/
    mod.rs
    handlers.rs           # route handlers only
    query.rs              # InstallQueryOptions
    responses.rs          # ScriptResponse, StringList
  infrastructure/
    mod.rs
    templates.rs          # template registration + filters
    static_site.rs        # static markdown rendering/cache
```

Design principles:
- HTTP layer converts request/response only.
- Service layer owns workflow and business decisions.
- Domain layer is framework-agnostic and pure.
- Infrastructure holds concrete adapters (Tera, GitHub client wrappers).
- Startup/bootstrap should fail with structured diagnostics, not panic traces.

---

## Phased Refactor Plan

## Phase 1 — Stabilize runtime behavior (must-do first)
1. ~~Introduce `AppError` enum and implement `IntoResponse`.~~
2. ~~Remove panic points from request path:~~
   - ~~Replace handler `unwrap()` calls with `?` and mapped errors.~~
   - ~~Replace `panic!()` branches in repo logic with typed errors.~~
3. ~~Return explicit status codes:~~
   - ~~400 for invalid user input~~
   - ~~404 for unsupported app/no matching assets~~
   - ~~502/503 for upstream GitHub failures~~

Deliverable: ~~no panics in normal HTTP request execution path.~~ ✅ Completed.

## Phase 1.5 — Fix contract drift and dead abstractions
1. ~~Either use `InstallQueryOptions::template_globals()` end-to-end or remove it.~~ ✅ Completed.
2. ~~Remove/replace placeholder `QueryOptions` trait if it has no production purpose.~~ ✅ Completed.
3. ~~Remove or wire currently unused response/domain wrappers.~~ ✅ Completed (`StringList` removed).
4. ~~Audit fields in `InstallQueryOptions` and enforce parity with template context.~~ ✅ Completed for handler/template context construction.

Deliverable: ~~API model and behavior are aligned, with no misleading dead abstractions.~~ ✅ Completed for current Phase 1.5 scope.

## Phase 2 — Extract use-case services
1. ~~Move `load_app` and install orchestration from `main.rs` into `services/installer.rs`.~~ ✅ Completed.
2. ~~Create a single `render_install_script(...)` API in `services/templating.rs`.~~ ✅ Completed.
3. ~~Refactor both install handlers to call shared service APIs.~~ ✅ Completed.

Deliverable: ~~handlers become thin adapters with minimal branching.~~ ✅ Completed.

## Phase 3 — Split `app_downloader.rs` by concern
1. ~~Create `domain/platform.rs` and move platform detection.~~ ✅ Completed.
2. ~~Create `domain/artifact.rs` and move filetype detection.~~ ✅ Completed.
3. ~~Create `domain/download.rs` for target + download models.~~ ✅ Completed.
4. ~~Keep backward-compatible type names during migration with temporary `pub use`.~~ ✅ Completed.

Deliverable: ~~each module has one clear responsibility and smaller file size.~~ ✅ Completed for Phase 3 scope.

## Phase 4 — Clarify API surface and ownership
1. ~~Move HTTP-specific structs out of `types.rs` into `http/`.~~ ✅ Completed.
2. ~~Keep domain types in `domain/` only (no mixed re-exports).~~ ✅ Completed.
3. ~~Tighten visibility (`pub(crate)`/private) to reduce accidental coupling.~~ ✅ Completed.

Deliverable: ~~predictable module boundaries and ownership.~~ ✅ Completed.

## Phase 5 — Normalize template/infrastructure behavior
1. Make template loading fail-fast and consistent (`Result` handling, no ignored add failures).
2. Remove template content logging in production logs.
3. Implement or remove unfinished PowerShell quote helper.
4. Cache static page map once (`LazyLock`) instead of per-request rebuild.

Deliverable: deterministic startup and cleaner operational behavior.

## Phase 5.5 — Startup and security hardening
1. Convert `main` startup path to return rich errors (`anyhow::Result<()>` or equivalent).
2. Remove startup `unwrap` for env parsing/address parsing/bind/serve.
3. Replace permissive CORS with explicit allowlist policy.
4. Replace `print!` diagnostics in service code with structured logging.

Deliverable: predictable startup behavior and explicit security posture.

## Phase 6 — Test coverage and safety net
1. Unit tests for platform/filetype identification edge cases.
2. Service tests for GitHub filtering behavior.
3. Handler/integration tests for:
   - unsupported app
   - upstream GitHub failure
   - successful script generation for Linux/Windows

Deliverable: regression resistance for future refactors.

---

## Concrete Cleanup Backlog
- [x] ~~Replace all panic/unwrap in request-path code.~~
- [ ] Replace startup/config `unwrap` calls with context-rich errors.
- [ ] Remove dead/unfinished code (`todo!` in runtime paths).
- [x] ~~Consolidate duplicated install handler logic.~~
- [x] ~~Split `app_downloader.rs` into cohesive modules.~~
- [x] ~~Move `types.rs` HTTP/domain mix into dedicated modules.~~
- [x] ~~Remove unused abstractions (`QueryOptions`, unused wrappers) or wire them properly.~~
- [ ] Standardize logging (no large template dumps).
- [ ] Restrict CORS policy to explicit origins/methods.
- [ ] Document module ownership in `README` or `ARCHITECTURE.md`.

---

## Suggested Delivery Sequence (2-3 weeks)

### Week 1
- Phase 1 + Phase 2 (stability + extraction)

### Week 2
- Phase 1.5 + Phase 3 + Phase 4 (contract cleanup + module split + ownership)

### Week 3
- Phase 5 + Phase 5.5 + Phase 6 (infra + hardening + tests)

---

## Success Criteria
- No panic-based control flow in handlers/services.
- No panic-based startup path for invalid config or bind failures.
- `main.rs` limited to bootstrap/routing wiring.
- Largest module under ~250 lines (guideline).
- Shared install flow used by both install endpoints.
- Error responses are typed and documented.
- Query model and template behavior are contract-consistent.
- CORS is explicit (not permissive).
- Integration tests cover primary happy path + key failures.
