# Liquid & Canvas — round-two follow-up archive

These are follow-up working-tree snapshots, each based on its own first-round
result, **not three fresh runs from an identical starting tree**. The principal
reported that round two was botched. Do not treat this archive as a controlled
comparison, rank agents from it, or infer equivalent prompts, budgets or assistance.

## Provenance and limits

- Medousa's updated report explicitly records a mistakenly supplied Hermes review,
  followed by a corrected review of Medousa's own revision. Earlier remediation
  was retained. That contamination is preserved, not silently normalized away.
- Pi's new `REVISION_REPORT.md` explicitly calls its revision incomplete.
- Hermes has source/test changes but no updated top-level benchmark report; its
  copied report is inherited first-round evidence, not round-two validation.
- No standalone round-two prompt, review payloads, model/budget/timing metadata,
  transcripts, or external build logs were collected. The [original task](../2026-11-08-liquid-canvas/TASK.md)
  is background only and is not asserted to be the exact round-two instruction.
- All reports are agent-authored claims. The archivist did not rerun Cargo tests,
  review implementation correctness, inspect native pixels or certify GUI/IME.

## Preservation method

The coordinator created one explicitly attributed snapshot commit per agent using
an alternate Git index, parenting it to that agent's original result. Tracked and
non-ignored untracked working-tree changes were included without source edits.
Original checkout HEADs and working-tree status were verified unchanged; the
original changes remain uncommitted there. These snapshot commits are archival
assistance, not commits made by the benchmark agents.

All first-round refs and original history are preserved. Round-two results stay
separate; nothing was merged into rndr or combined into one implementation.
Bundle SHA-256 checks matched between Lenovo and Mac. Git bundle validation,
exact tree/parent/hash checks and per-round diff whitespace checks passed.

| Agent | First-round parent | Round-two snapshot |
|---|---|---|
| medousa | `a22dfd0` | [`8ca918c`](https://github.com/EntasisLabs/rndr-bench/tree/benchmarks/2026-11-09-round-2/medousa) |
| hermes | `d73ace9` | [`453eaf4`](https://github.com/EntasisLabs/rndr-bench/tree/benchmarks/2026-11-09-round-2/hermes) |
| pi | `bf3fe58` | [`5ab873c`](https://github.com/EntasisLabs/rndr-bench/tree/benchmarks/2026-11-09-round-2/pi) |

See [manifest.json](manifest.json) for exact hashes, file status and change stats.
To inspect only round-two changes, compare each snapshot with its listed parent:

```sh
git diff origin/benchmarks/2026-11-08/medousa..origin/benchmarks/2026-11-09-round-2/medousa
```

The folder date is the UTC archival date, not a measured run start/end time.
