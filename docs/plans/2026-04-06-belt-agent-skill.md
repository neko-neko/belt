# belt-agent Generic Skill Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** belt-agent CLI を駆動する LLM 向けの汎用プロトコルスキル (`skills/belt-agent/SKILL.md`) を作成する。

**Architecture:** 単一の Markdown ファイルで、Belt Protocol ループ、コマンドレスポンス解釈、HARD-GATE、well-known config keys を定義する。全文英語。パイプライン固有スキルがこのファイルを内部参照する基盤レイヤー。

**Tech Stack:** Markdown (skill format)

**Spec:** [docs/specs/2026-04-06-belt-agent-skill.md](../specs/2026-04-06-belt-agent-skill.md)

---

## File Structure

```
skills/
└── belt-agent/
    └── SKILL.md    # Belt Protocol skill (English, single file)
```

---

### Task 1: Create `skills/belt-agent/SKILL.md`

**Files:**
- Create: `skills/belt-agent/SKILL.md`

- [ ] **Step 1: Create directory and write SKILL.md**

Create `skills/belt-agent/SKILL.md` with the following complete content:

```markdown
# Belt Protocol

Generic protocol for driving the belt-agent CLI. This skill defines how LLM agents
interact with belt's deterministic state machine — the command loop, response
interpretation, and safety constraints.

Pipeline-specific skills reference this protocol for consistent belt-agent usage.
This skill is not invoked directly by users.

## Protocol Loop

```
belt-agent init <pipeline.yml> [--arg key=value ...]
  |
loop {
  phase = belt-agent next [--run <id>]

  if phase.completed:
    break                        # pipeline complete

  execute(phase)                 # LLM executes the phase (see config)

  if phase.gate is not empty:
    result = belt-agent verify [--run <id>]

    while result.verdict == "FAIL":
      fix(result)                # LLM fixes failing gates
      result = belt-agent verify

  if phase.confirm or phase.validate is not empty:
    belt-agent step --confirm [--run <id>]
  else:
    belt-agent step [--run <id>]
}
```

### Rules

- `next` returns all phase information: description, config, artifacts, gate, validate,
  confirm, regate, max_retries, attempt, and args.
- Phases with gates require a `verify` PASS before `step`.
- Phases without gates (confirm-only, etc.) skip `verify` and go directly to `step`.
- `step` success is determined by the `advanced` field in the JSON response.
- `status` can be called at any time to inspect the full run state.
- `--run <id>` is optional on all commands; omit it to use the latest run.

## Command Response Handling

### init

Starts a new run. Returns the first active phase (after `when:` evaluation).

```json
{
  "run_id": "019d6195-...",
  "pipeline": "feature-dev",
  "phase": {
    "id": "design/explore",
    "description": "...",
    "config": {},
    "artifacts": [".belt/exploration/*.json"],
    "output_dir": ".belt/runs/<run_id>/design_explore"
  },
  "gate": [{ "file_exists": ".belt/exploration/*.json" }],
  "validate": [],
  "confirm": false,
  "max_retries": 0,
  "attempt": 0,
  "args": {}
}
```

### next

Returns current phase information, or signals pipeline completion.

| Response | Action |
|----------|--------|
| `completed: true` | Exit loop. Pipeline complete. |
| `phase` present | Read phase info, begin execution. |

### verify

Runs gate checks and returns a verdict.

```json
{
  "run_id": "...",
  "phase": "design/explore",
  "verdict": "PASS",
  "checks": [
    {
      "check_type": "file_exists",
      "passed": true,
      "detail": "matched 3 files",
      "duration_ms": null
    }
  ],
  "attempt": 1,
  "max_retries": 0
}
```

| Response | Action |
|----------|--------|
| `verdict: "PASS"` | Gate passed. Proceed to `step`. |
| `verdict: "FAIL"` | Read `checks`, fix failing gates, re-run `verify`. |

### step

Advances to the next phase.

| Response | Action |
|----------|--------|
| `advanced: true`, `to` present | Transition succeeded. Call `next`. |
| `advanced: true`, `completed: true` | Pipeline complete. |
| `advanced: false`, `reason: "confirmation_required"` | `--confirm` needed. Verify validate criteria first, then retry with `--confirm`. |

### status

Returns the full run state. Can be called at any time.

```json
{
  "run_id": "...",
  "pipeline": "feature-dev",
  "pipeline_file": "examples/feature-dev/pipeline.yml",
  "current_phase": "design/synthesize",
  "completed_phases": ["design/explore"],
  "skipped_phases": [],
  "phase_attempts": { "design/explore": 1 },
  "args": {},
  "created_at": "...",
  "updated_at": "..."
}
```

### Regate

When `next` returns a `regate` field containing phase IDs, `verify` re-checks gates
for those phases in addition to the current phase.

- If a regate target's gate fails, fix **that phase's work** (not the current phase).
- Repeat verify -> fix until all regate targets and the current phase pass.

## HARD-GATE

<HARD-GATE>
When validate criteria exist for a phase, you MUST NOT run
`belt-agent step --confirm` without verifying each criterion.

validate is a list of criteria that belt returns for LLM judgment.
belt cannot know whether these criteria were actually evaluated.
The --confirm flag is the LLM's declaration that criteria have been verified.
Passing it without verification is a protocol violation.
</HARD-GATE>

## Well-known Config Keys

`config` is an opaque map that belt passes through without interpretation.
Only the following key has a defined meaning in this protocol:

| Key | Type | Meaning |
|-----|------|---------|
| `config.skill` | `string` | Skill to invoke for this phase. |

### Rules

- Unknown config keys MAY be ignored (forward compatibility).
- Pipeline-specific skills MAY add custom keys freely (belt does not interpret them).
- Dispatch implementation (which agents to launch, how to execute) is the
  pipeline-specific skill's responsibility.
```

- [ ] **Step 2: Verify file was created correctly**

Run: `cat skills/belt-agent/SKILL.md | head -5`
Expected:
```
# Belt Protocol

Generic protocol for driving the belt-agent CLI. This skill defines how LLM agents
interact with belt's deterministic state machine — the command loop, response
interpretation, and safety constraints.
```

---

### Task 2: Validate Skill Against belt-agent CLI

**Files:**
- Read: `skills/belt-agent/SKILL.md`
- Read: `crates/belt-agent/src/main.rs`

スキルに記述された JSON 構造が実際の belt-agent CLI 出力と一致していることを検証する。

- [ ] **Step 1: Verify init output structure**

Run:
```bash
./target/debug/belt-agent init examples/feature-dev/pipeline.yml 2>/dev/null | python3 -c "
import sys, json
d = json.load(sys.stdin)
required = ['run_id', 'pipeline', 'phase', 'gate', 'validate', 'confirm', 'max_retries', 'attempt', 'args']
missing = [k for k in required if k not in d]
phase_required = ['id', 'description', 'config', 'artifacts', 'output_dir']
phase_missing = [k for k in phase_required if k not in d.get('phase', {})]
print(f'top-level missing: {missing}')
print(f'phase missing: {phase_missing}')
print('PASS' if not missing and not phase_missing else 'FAIL')
"
```

Expected: `PASS` (all fields present)

- [ ] **Step 2: Verify verify output structure**

Run:
```bash
RUN_ID=$(./target/debug/belt-agent init examples/feature-dev/pipeline.yml 2>/dev/null | python3 -c "import sys,json; print(json.load(sys.stdin)['run_id'])")
./target/debug/belt-agent verify --run "$RUN_ID" 2>/dev/null | python3 -c "
import sys, json
d = json.load(sys.stdin)
required = ['run_id', 'phase', 'verdict', 'checks', 'attempt', 'max_retries']
missing = [k for k in required if k not in d]
if d.get('checks'):
    check_required = ['check_type', 'passed', 'detail']
    check_missing = [k for k in check_required if k not in d['checks'][0]]
else:
    check_missing = []
print(f'top-level missing: {missing}')
print(f'check missing: {check_missing}')
print('PASS' if not missing and not check_missing else 'FAIL')
"
```

Expected: `PASS`

- [ ] **Step 3: Verify step output structure (normal advance)**

Run:
```bash
RUN_ID=$(./target/debug/belt-agent init examples/feature-dev/pipeline.yml 2>/dev/null | python3 -c "import sys,json; print(json.load(sys.stdin)['run_id'])")
./target/debug/belt-agent step --run "$RUN_ID" 2>/dev/null | python3 -c "
import sys, json
d = json.load(sys.stdin)
has_advanced = 'advanced' in d and d['advanced'] == True
has_from = 'from' in d
has_to = 'to' in d
print(f'advanced={d.get(\"advanced\")}, from={d.get(\"from\")}, to={d.get(\"to\")}')
print('PASS' if has_advanced and has_from and has_to else 'FAIL')
"
```

Expected: `PASS` with `advanced=True, from=design/explore, to=design/synthesize`

- [ ] **Step 4: Verify step output structure (confirmation_required)**

Run:
```bash
RUN_ID=$(./target/debug/belt-agent init examples/feature-dev/pipeline.yml 2>/dev/null | python3 -c "import sys,json; print(json.load(sys.stdin)['run_id'])")
# Advance through phases until we hit a confirm phase (design/write-design has confirm: true)
./target/debug/belt-agent step --run "$RUN_ID" 2>/dev/null  # design/explore -> design/synthesize
./target/debug/belt-agent step --run "$RUN_ID" 2>/dev/null  # design/synthesize -> design/write-design
./target/debug/belt-agent step --run "$RUN_ID" 2>/dev/null | python3 -c "
import sys, json
d = json.load(sys.stdin)
is_blocked = d.get('advanced') == False and d.get('reason') == 'confirmation_required'
print(f'advanced={d.get(\"advanced\")}, reason={d.get(\"reason\")}')
print('PASS' if is_blocked else 'FAIL')
"
```

Expected: `PASS` with `advanced=False, reason=confirmation_required`

- [ ] **Step 5: Verify status output structure**

Run:
```bash
RUN_ID=$(./target/debug/belt-agent init examples/feature-dev/pipeline.yml 2>/dev/null | python3 -c "import sys,json; print(json.load(sys.stdin)['run_id'])")
./target/debug/belt-agent status --run "$RUN_ID" 2>&1 | python3 -c "
import sys, json
d = json.load(sys.stdin)
required = ['run_id', 'pipeline', 'pipeline_file', 'current_phase', 'completed_phases', 'skipped_phases', 'phase_attempts', 'args', 'created_at', 'updated_at']
missing = [k for k in required if k not in d]
print(f'missing: {missing}')
print('PASS' if not missing else 'FAIL')
"
```

Expected: `PASS`

---

### Task 3: Commit

**Files:**
- Stage: `skills/belt-agent/SKILL.md`

- [ ] **Step 1: Commit the skill file**

Run:
```bash
git add skills/belt-agent/SKILL.md
git commit -m "feat(belt-agent): add generic Belt Protocol skill

Defines the belt-agent CLI driving protocol for LLM agents:
- Protocol loop (init -> next -> verify -> step)
- Command response interpretation with JSON examples
- HARD-GATE for validate verification obligation
- Well-known config keys (config.skill)"
```

---

### Task 4: Cleanup test runs

- [ ] **Step 1: Remove .belt/runs/ created during validation**

Run:
```bash
rm -rf .belt/runs/
```
