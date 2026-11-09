#!/usr/bin/env bash
set -u
B="$HOME/benchmarks/rndr-liquid-canvas-20261108"
D="$B/baseline"
while pgrep -x dnf >/dev/null; do sleep 5; done
export CARGO_TARGET_DIR=/srv/benchmarks/rndr-liquid-canvas-20261108/build/baseline
export CARGO_BUILD_JOBS=2
export CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
cd "$B/worktrees/medousa" || exit 1
run() {
 name="$1"; shift
 printf '%s START %s\n' "$(date -Is)" "$name" >> "$D/status.tsv"
 "$@" > "$D/$name.log" 2>&1
 rc=$?
 printf '%s END %s %s\n' "$(date -Is)" "$name" "$rc" >> "$D/status.tsv"
}
run format cargo fmt --check
run metadata cargo metadata --locked --format-version 1 --no-deps
run core-tests cargo test --locked -p rndr_core -p rndr_liquid -p rndr_liquid_ui -p rndr_canvas -p rndr_canvas_ui
run facade-tests cargo test --locked -p rndr --no-default-features --features liquid,canvas --lib
run default-facade-tests cargo test --locked -p rndr --features liquid,canvas --lib
run clippy cargo clippy --locked -p rndr_core -p rndr_liquid -p rndr_liquid_ui -p rndr_canvas -p rndr_canvas_ui -p rndr --features liquid,canvas --all-targets -- -D warnings
run examples cargo build --locked -p liquid-view -p canvas-editor
run workspace-check cargo check --locked --workspace --all-targets
for a in medousa hermes pi; do
 { echo "=== $a ==="; git -C "$B/worktrees/$a" rev-parse HEAD; git -C "$B/worktrees/$a" status --porcelain; sha256sum "$B/worktrees/$a/Cargo.lock"; } >> "$D/final-workspaces.log"
done
date -Is > "$D/DONE"
