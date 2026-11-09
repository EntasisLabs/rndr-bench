# Benchmark baseline — 2026-11-08

## Verdict
Ready for the scoped Liquid/Canvas implementation benchmark. Scoped tests and whole-workspace all-targets cargo check are green. Native interaction remains unverified; whole-workspace tests and whole-workspace Clippy were not run.

## Common source
All three workspaces are clean at b2a1c214cb9856a73efca25000a0fa5bf51c1b25, with identical Cargo.lock SHA-256 6c2508569be5ca1c33d63c2b69afa5c316deab094f26e2ebe43c45e9eb3dc33a.
Rust/cargo 1.99.0. No agent launched or benchmark source changed.

## Passed
- cargo fmt --check
- Locked dependency metadata
- Core/Liquid/Canvas package tests: 836 passed, 0 failed, 3 ignored (including doc tests)
- Facade library tests with no defaults + liquid,canvas: 3 passed
- Facade library tests with defaults + liquid,canvas: 3 passed
- Scoped all-targets Clippy with -D warnings
- Desktop examples liquid-view and canvas-editor built successfully

Exact commands are in check.sh; exit codes and logs are in this directory.

## Known exclusions and limitations
- RESOLVED: the original whole-workspace check failed on missing browser-stack development libraries. With user approval CRB and EPEL were enabled and webkit2gtk4.1-devel plus dependencies installed. pkg-config verifies libsoup 3.6.5 and WebKitGTK/JavaScriptCoreGTK 2.50.4. The rerun of cargo check --locked --workspace --all-targets passed (exit 0). See dependencies/install.log, pkg-config.log, workspace-check.log and result.txt. The original failure log is retained as historical evidence.
- SSH has no DISPLAY or WAYLAND_DISPLAY; no native GUI/IME interaction or rendered appearance was verified. Building an example is not equivalent to exercising it. Native interaction still needs a suitable desktop session.
- Baseline used a separate /srv build directory, two build jobs, CARGO_INCREMENTAL=0 and DEV/TEST debug info disabled. enter.sh sets the same two-job limit for agents but preserves default compiler profiles; baseline therefore does not prove default-profile compile time or peak resource usage. All three agents get identical settings.
- Agent build directories were empty at verification. Baseline did not warm any individual agent target directory; shared Cargo registry/dependency downloads may be warm for all.
- Prefer sequential measured runs on the shared 15 GiB RAM host; separate target directories do not isolate CPU/RAM or OS caches.

## Evidence
status.tsv, check.sh, core-tests.log, facade-tests.log, default-facade-tests.log, clippy.log, examples.log, workspace-check.log, final-workspaces.log, DONE.

## Dependency follow-up — 2026-11-08
CRB/EPEL enabled; WebKitGTK development dependencies installed successfully from compatible EL10 repositories. All three worktrees reverified clean at the original commit, with identical lockfiles and empty individual build directories; see dependencies/workspaces.log. No agents launched or source modified.
