---
name: criteria-template
description: >-
  Template and quality rules for done-criteria files. Reference when creating a
  new done-criteria file.
---

# Done Criteria Template

## File Format

```markdown
---
phase: {N}
name: {phase_name}
max_retries: 3
audit: required
---

## Criteria

### {ID}: {criterion title}
- **severity**: blocker | quality
- **verify_type**: automated | inspection
- **verification**:
  {Concrete steps for the Audit Agent to execute. For inspection type, numbered steps are required.}
- **pass_condition**: {No subjective terms. Use numeric thresholds or pattern matches that can be judged deterministically.}
- **fail_diagnosis_hint**: {On FAIL, what to investigate to resolve.}
- **uses_evidence**: [{evidence-ids from the skill-local evidence-catalog.md, e.g., E-TEST, E-LINT. Optional.}]
- **depends_on_artifacts**: [{Paths to artifacts required for verification. Optional.}]
- **forward_check**: {Whether this is sufficient as input to the next phase. Optional.}
```

## Template Rules

These rules are enforced by the template structure itself:

1. **No subjective terms in `pass_condition`**: Words like "appropriate", "sufficient", "concrete", or "correct" are disallowed. Use numeric thresholds (e.g., "2 or more") or pattern matches (e.g., "contains a file path").
2. **Inspection type requires numbered steps**: Enumerate the decision procedure in `verification` as "1. ... 2. ... 3. ...".
3. **`fail_diagnosis_hint` is required**: On FAIL, always state what to investigate to resolve the failure.
4. **Severity semantics**:
   - `blocker`: If unmet, the phase must FAIL. Eligible for the fix-and-re-audit retry loop.
   - `quality`: If unmet while every blocker PASSes, the phase passes with a warning only. Not eligible for the retry loop.
5. **`uses_evidence` references must resolve**: If `uses_evidence` is present, each listed `E-XXX` MUST be declared in the owning skill's `./references/evidence-catalog.md`. (Currently convention-level; lint enforcement is Future Work per spec.)

## Human Review: 3-Point Scan

When creating or modifying a done-criteria file, check these three points:

1. **Can you judge PASS/FAIL yourself by reading `pass_condition`?** — If not, it is ambiguous.
2. **Are there too many blocker criteria?** — Making everything a blocker causes retry hell.
3. **Is any truly necessary check for this phase missing?** — Coverage.

Expected time: 2-3 minutes per file.

## ID Convention

- Phase N criteria: `DN-01`, `DN-02`, ...
- Evidence-derived criteria (synthesized dynamically): `DN-E1`, `DN-E2`, ...
