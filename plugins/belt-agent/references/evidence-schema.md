---
name: evidence-schema
description: >-
  Schema for evidence collection and verification. Domain-neutral
  type-only core. Concrete evidence items are defined in each skill's
  own evidence-catalog.md.
---

# Evidence Schema

Type and protocol definition for evidence collection and verification.
Concrete evidence-ids are defined in each skill's
`./references/evidence-catalog.md`.

## 2-Layer Model

- **Claimed (Layer 1)**: "what happened" record that the Executor collects and stores
- **Verified (Layer 2)**: "really holds" check independently run by the Audit Agent

Layer 1 must always be produced during phase execution. Layer 2 runs only when
the environment satisfies the declared `required_capabilities`; when it does
not, the auditor proceeds with Layer 1 evidence alone, annotated to flag the
coverage gap.

## Applicability Condition Notation

Evidence applicability is decided by predicates based on observable facts
(file existence, keyword occurrence, etc.):

- `condition: always` — always applicable
- `condition: require_all: [<predicate>, ...]` — applicable only when all predicates hold
- `condition: require_any: [<predicate>, ...]` — applicable when any predicate holds

Predicates are things like glob patterns, grep patterns, or keyword
occurrences in the spec body. They must be decidable and independently
reproducible.

## if_unavailable Policy (3 kinds)

Behavior when the evidence's `required_capabilities` are not satisfied:

| Policy | Behavior |
|---|---|
| `skip_with_warning` | Exclude the evidence; no impact on the verdict (warning only) |
| `manual_fallback` | PAUSE and let the user collect it; resume after user input |
| `block` | Fail as a blocker if collection is impossible; the phase does not pass |

## Evidence Declaration Structure

Each evidence-id in a skill-local `evidence-catalog.md` has the following fields:

| Field | Required | Description |
|---|---|---|
| `id` | Yes | Unique identifier (e.g., `E-TEST`, `E-LINT`) |
| `description` | Yes | One-line description |
| `claimed` | Yes | Layer 1 storage path (may include templates) |
| `verified` | Yes | Layer 2 verification procedure (independently runnable) |
| `required_capabilities` | Yes | Capabilities needed for Layer 2 (e.g., `[bash]`, `[browser-automation]`) |
| `condition` | Yes | Applicability decision (notation above) |
| `if_unavailable` | Yes | Policy choice |
| `collection` | Yes | Claimed-evidence collection procedure (how the Executor produces the artifact) |
| `variants` | No | Variants within the same id (e.g., `[desktop, mobile]`) |

## Phase Reference (Inversion of Control)

Each phase's `criteria/<phase>.md` **picks** evidence via
`uses_evidence: [E-XXX]`. The reverse direction, where evidence specifies
phases (`applies_to: [...]`), is not adopted. There is no activity type enum.

```markdown
### <ID>: <criterion title>
- severity, verify_type, verification, pass_condition, fail_diagnosis_hint
- uses_evidence: [E-TEST, E-LINT]   (optional, references skill-local evidence-catalog)
- depends_on_artifacts: [path]       (optional, direct path reference)
- forward_check
```

`uses_evidence` is an optional field. It coexists with the existing
`depends_on_artifacts` field.

## Locations

- Schema (this file): `plugins/belt-agent/references/evidence-schema.md`
- Concrete catalogs: `plugins/belt/skills/<skill>/references/evidence-catalog.md`
- Phase-side pick: `uses_evidence:` field in `plugins/belt/skills/<skill>/criteria/<phase>.md`

## See also

- [`_schema.md`](./_schema.md) — done-criteria schema (the sibling type-only core for criteria files; criteria consume evidence via `uses_evidence`).
