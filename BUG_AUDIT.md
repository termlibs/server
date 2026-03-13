# Termlibs Server Bug Audit

## Scope
- Covers the Rust server codebase under `src/`.
- Excludes `termlibs` submodules.
- Derived from the refactor review with focus on runtime and behavioral risks.

## Severity Key
- **High**: likely to break request handling or crash execution.
- **Medium**: meaningful stability/correctness risk.
- **Low**: likely correctness drift or maintainability hazard with indirect impact.

## High Severity

### ~~1) Panic in request path when app loading fails~~ ✅ Fixed in Phase 1
- File: `src/main.rs:93`
- File: `src/main.rs:137`
- Code: `load_app(...).await.unwrap()`
- Risk: upstream GitHub errors, filtering failures, or missing assets can panic during request handling.
- Suggested fix: return typed errors from handlers and map to HTTP responses.

### ~~2) Panic on unsupported app lookup~~ ✅ Fixed in Phase 1
- File: `src/main.rs:135`
- Code: `supported_apps::get_app(&app).unwrap()`
- Risk: unknown app names trigger panic instead of controlled 4xx response.
- Suggested fix: convert to `Result` and return `404` or `400` with structured error body.

### ~~3) Repository parsing assumes owner/repo format~~ ✅ Fixed in Phase 1
- File: `src/gh.rs:15`
- Code: `repo_string.split_once('/').unwrap()`
- Risk: malformed repo strings panic.
- Suggested fix: validate format and return typed validation error.

## Medium Severity

### 4) Unfinished `todo!()` in runtime-adjacent template code
- File: `src/templates.rs:58`
- Code: `todo!()` in `Powershell::quote`
- Risk: panic if called.
- Suggested fix: implement now or remove dead code path.

### 5) UTF-8 conversion uses `unwrap`
- File: `src/templates.rs:28`
- Code: `String::from_utf8(...).unwrap()`
- Risk: panic on invalid byte sequence.
- Suggested fix: return `tera::Error`, or use safe conversion strategy if lossy behavior is acceptable.

### ~~6) Non-GitHub repo paths panic~~ ✅ Fixed in Phase 1
- File: `src/supported_apps.rs:90`
- File: `src/supported_apps.rs:91`
- Code: `panic!("... is not a github repo")`
- Risk: enum misuse or expanded feature paths can crash process.
- Suggested fix: replace panic with typed domain error.

### 7) Startup path has multiple panic exits
- File: `src/main.rs:217`
- File: `src/main.rs:229`
- File: `src/main.rs:230`
- File: `src/main.rs:232`
- Code: `.parse::<u16>().unwrap()`, `.parse::<SocketAddr>().unwrap()`, `bind(...).await.unwrap()`, `serve(...).await.unwrap()`
- Risk: bad env/config or bind failures crash process instead of failing with actionable diagnostics and exit codes.
- Suggested fix: return `anyhow::Result<()>` from `main`, bubble context-rich errors with `?`, and log fatal startup failures once.

## Low Severity / Correctness Drift

### 8) Architecture alias ambiguity
- File: `src/app_downloader.rs`
- Issue: separate `Arm64` and `Aarch64` variants can fragment matching logic.
- Risk: inconsistent target resolution for equivalent architecture labels.
- Suggested fix: normalize to a single canonical variant with aliases at parse time.

### 9) Query fields modeled but not consistently applied
- Files: `src/types.rs`, `src/main.rs`
- Issue: several options are parsed but only partially represented in template context and behavior.
- Risk: API behavior drifts from user expectation.
- Suggested fix: either implement all declared fields end-to-end or remove/deprecate unused fields.

### 10) Open CORS policy in production route stack
- File: `src/main.rs:248`
- Code: `CorsLayer::permissive()`
- Risk: broad cross-origin access can be abused if endpoints evolve to include stateful or sensitive operations.
- Suggested fix: switch to explicit allowlist + allowed methods/headers.

### 11) Verbose template content logging
- File: `src/templates.rs:12`
- Code: `info!("Content: {}", content)`
- Risk: noisy logs and potential leakage of full script internals in centralized logging.
- Suggested fix: log template names only, optionally include content hash for diagnostics.

### 12) Test scaffold provides no assertion coverage
- File: `src/gh.rs:82`
- Issue: `base_test` iterates repos but all meaningful assertions are commented out.
- Risk: false confidence; regressions in asset filtering can ship undetected.
- Suggested fix: replace with deterministic unit tests and opt-in integration tests.

## Priority Fix Order
1. Remove panic/unwrap from request handlers and core request path.
2. Add typed error model and HTTP mapping (`400/404/502/503`).
3. Eliminate `panic!`/`todo!` runtime paths.
4. Harden startup/config error path (`main` should not unwrap).
5. Normalize target architecture modeling.
6. Align query model with actual behavior.
7. Lock down CORS and reduce log leakage.

## Verification Checklist
- [x] ~~Unsupported app returns structured `4xx`, no panic.~~
- [x] ~~Upstream GitHub failures return structured `5xx`, no panic.~~
- [x] ~~No `unwrap()` in request path.~~
- [ ] No reachable `todo!()` or `panic!()` in request-serving flow.
- [ ] Startup failures return clean, context-rich errors without panic.
- [ ] Architecture aliases resolve to canonical target consistently.
- [ ] Query options are either fully implemented or removed/documented.
- [ ] CORS policy is explicit and least-privilege.
