# rndr benchmarks

Archive of independent agent implementations against recorded rndr baselines.
`main` is the benchmark index and evidence catalog, not a merged implementation.
Original source history and exact agent commit hashes are preserved on dated branches.

## 2026-11-08 — Stateful Liquid & editable Canvas

- [Authoritative task](benchmarks/2026-11-08-liquid-canvas/TASK.md)
- [Baseline verification](benchmarks/2026-11-08-liquid-canvas/BASELINE_REPORT.md)
- [Machine-readable run manifest](benchmarks/2026-11-08-liquid-canvas/manifest.json)
- Baseline: `b2a1c214cb9856a73efca25000a0fa5bf51c1b25` on `benchmarks/2026-11-08/baseline`.

| Agent | Archived tip | Commits above baseline |
|---|---|---:|
| medousa | [`a22dfd0`](https://github.com/EntasisLabs/rndr-bench/tree/benchmarks/2026-11-08/medousa) | 3 |
| hermes | [`d73ace9`](https://github.com/EntasisLabs/rndr-bench/tree/benchmarks/2026-11-08/hermes) | 6 |
| pi | [`bf3fe58`](https://github.com/EntasisLabs/rndr-bench/tree/benchmarks/2026-11-08/pi) | 4 |

Each agent's BENCHMARK_REPORT.md is preserved on its source branch and copied into
this run's evidence directory. These are agent-authored claims, not independent
review results. No winner has been selected; implementation review is pending.
Native GUI/IME behavior is not established by successful compilation or headless tests.
Model, budget, elapsed-time and transcript records have not been collected here;
do not assume equivalent budgets or use commit count as a quality score.
Medousa's commits required coordinator assistance with Git identity and one trailing
space; no implementation logic was changed by that assistance.

## Inspect a result

```sh
git clone https://github.com/EntasisLabs/rndr-bench.git
cd rndr-bench
git switch --track origin/benchmarks/2026-11-08/medousa
# Substitute hermes or pi as needed.
git diff origin/benchmarks/2026-11-08/baseline...HEAD
```

Follow the selected branch's example README and validation report for commands and
limitations. Build outputs, credentials, machine environment dumps, and raw session
logs are deliberately not included. This archive does not merge changes into rndr.

## Future runs

Add a dated evidence directory and immutable result branches using
`benchmarks/YYYY-MM-DD/<agent>` (add a run suffix for multiple runs on one date).
Record the exact baseline and tip hashes, task, model/runtime versions, budgets,
assistance, verification evidence, and review conclusions. Do not overwrite old results.
