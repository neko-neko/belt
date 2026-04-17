# Resume Mode

Driver-side behaviour when the belt protocol receives a resume invocation.

## Detection

If the Skill invoke `args` string contains the literal prefix
`resume run_id=` followed by a UUIDv7 run identifier, the protocol
driver is in **resume mode**. Example args string:

```
resume run_id=01JBC5R1MZQFVY9T8H0K3P7Q2N
```

Exact format; do not accept synonyms (`resume_run=`, `run=`, etc.).

## Steps

1. Do not run `belt-agent init`. The run already exists on disk.
2. Run `belt-agent status --run <id>` to read the current phase.
3. If `current_phase == "COMPLETED"`, report back to the caller and stop.
   (The caller `/belt:resume` is expected to catch this first via its
   precondition; this is a defensive second check.)
4. If `.belt/runs/<id>/handover.md` exists, read it and incorporate the
   `## Resume hint` section into the LLM context before resuming normal
   workflow.
5. Continue with the normal protocol loop: `belt-agent next` → `verify` /
   `regate` / `step` as usual.

## Cross-pipeline applicability

Resume mode is defined once here and applies to every pipeline driven by
this protocol. No per-pipeline SKILL.md change is needed.
