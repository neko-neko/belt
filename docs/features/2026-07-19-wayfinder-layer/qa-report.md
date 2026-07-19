# QA report — wayfinder-layer

- **Feature**: wayfinder-layer (`/belt:wayfinder` experimental planning skill)
- **Branch**: `feature/2026-07-19-wayfinder-layer` @ `5846e5431cbd9362b9f413829352ba26b7ee0831`
- **Scenario source**: `docs/features/2026-07-19-wayfinder-layer/scenarios.yml` (8 scenarios, all `kind: cli`)
- **Evidence dir**: `.belt/runs/019f7907-8f44-7163-ab46-657e71629c1a/qa/` (one `<id>.txt` transcript per scenario; not committed)
- **Method**: each scenario's `when:` command executed verbatim from the repo root; command, full output, and exit code captured; judged against its `then:` clause. QA verifier edits no source files.

## Scenario results

| # | id | kind | Result | Evidence file | Note |
|---|----|------|--------|---------------|------|
| 1 | lint-passes | cli | PASS | `qa/lint-passes.txt` | `belt lint` exited 0, printed `ok: …/pipeline.yml` |
| 2 | pipeline-structure | cli | PASS | `qa/pipeline-structure.txt` | map/resolve/handoff ids present; resolve phase declares `max_retries: 0` |
| 3 | command-correctness | cli | PASS | `qa/command-correctness.txt` | blocked-by / linear api / handoff-ref.json / `--workspace neko-neko` / `--run` present; inverted `blocks <blocker>` absent |
| 4 | skill-config-rounds | cli | PASS | `qa/skill-config-rounds.txt` | `[wayfinder]` header + `rounds` key documented in SKILL.md |
| 5 | readme-declarations | cli | PASS | `qa/readme-declarations.txt` | all 7 AND-conjunct tokens present (wayfinder, linear-cli, 2.0.0, brainstorming, domain-modeling, prototype, mattpocock attribution) |
| 6 | manifests-list-wayfinder | cli | PASS | `qa/manifests-list-wayfinder.txt` | both plugin.json and marketplace.json reference `/belt:wayfinder` |
| 7 | git-hygiene | cli | PASS | `qa/git-hygiene.txt` | grep over diff produced no output, exited 1 (no `crates/` or pre-existing skill/agent file touched) |
| 8 | requirements-skill-untouched | cli | PASS | `qa/requirements-skill-untouched.txt` | path-scoped diff empty; requirements skill byte-unmodified |

**Tally: 8 PASS / 0 FAIL (8 of 8).**

## Exploratory pass (bounded, read-only / non-mutating)

Advisory checks beyond the 8 scenarios. None mutated Linear or any repo file. Findings are advisory notes, not scenario results.

- **Both manifests are valid JSON.** `python3 -c 'import json; json.load(...)'` on `plugins/belt/.claude-plugin/plugin.json` and `.claude-plugin/marketplace.json` both printed `VALID JSON`. This strengthens scenario 6 (a `grep` match alone would not prove well-formed JSON).
- **`map_id` sed contract is consistent between pipeline.yml and SKILL.md.** Both gates (map phase line 16, resolve phase line 36) extract the map id with `sed -n 's/^- map_id:[[:space:]]*//p' "$mr" | head -1`, anchored to `- map_id:` at column 0. SKILL.md §4 (line 90) documents the requirement as "It MUST contain the literal line `- map_id: <ID>` — the pipeline gate parses that exact form." The 4-space indentation in the SKILL.md "Layout:" block (lines 94–104) is markdown code-block rendering, not file content; the prose pins the literal, unindented line. Contract matches — no discrepancy.
- **belt binary sanity.** `belt` on PATH resolves to this repo's release build (`target/release/belt`); scenario 1's `ok:` output confirms it parses and lints the pipeline. (`belt --version` is not a supported flag — belt is subcommand-only — but this is expected and unrelated to any scenario.)
- **Change scope.** `git diff --name-only main...HEAD` lists 13 files; the shippable subset (wayfinder skill triad, README.md, plugin.json, marketplace.json) plus planning docs under `docs/`. No `crates/` path and no pre-existing skill/agent file appears — consistent with scenarios 7 and 8.

### Scope boundary — live Linear pilot deliberately excluded (NOT a FAIL)

Acceptance for this feature is split by design. AC-1 (lint) and AC-7 (git hygiene), plus command-correctness / structure / config / README / manifest declarations, are the deterministic static checks encoded in `scenarios.yml` and verified above. **AC-2, AC-3, AC-4, AC-5, AC-6, and AC-8 are LIVE behaviors that write to the real `neko-neko` Linear workspace** (create the map issue, decision sub-issues and blocking relations, resolve tickets, run AFK research, hand off to `/belt:requirements`, reconstruct the frontier across sessions).

These live behaviors live in `pilot-runbook.md` (scenarios P1–P6) as a **user-authorized pilot**, intentionally kept out of `scenarios.yml` because belt QA has no per-scenario authorization gate and the QA verifier replays every scenario present. Per the QA charter, this verifier executed **no** Linear-writing command and did **not** attempt the pilot. The absence of live coverage here is **by design and is not recorded as a failure**. The merge bar for this feature is explicitly "authored, pilot-unverified": passing `scenarios.yml` is sufficient to pass autonomous QA, and the live pilot is a recorded required follow-up owned by the user.

## Required follow-up (non-blocking for this QA gate)

- Execute the live pilot in `pilot-runbook.md` (P1–P6) against the `neko-neko` Linear workspace to validate AC-2..AC-6 and AC-8 behaviorally. This is a user-driven, authorized run and is out of autonomous-QA scope.

## Verdict

**PASS — 8/8 scenarios pass. Autonomous QA gate met.** The wayfinder-layer feature satisfies every deterministic scenario in `scenarios.yml`; the live Linear pilot (AC-2..AC-6, AC-8) is a deliberately-split, user-authorized required follow-up and is not counted against this gate.
