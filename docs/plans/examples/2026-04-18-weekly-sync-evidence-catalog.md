---
name: evidence-catalog
description: >-
  Concrete evidence catalog for weekly-sync pipeline (thought experiment).
  Conforms to plugins/belt-agent/references/evidence-schema.md.
---

# Evidence Catalog (weekly-sync thought experiment)

Schema: plugins/belt-agent/references/evidence-schema.md

## Evidence

### E-LINEAR-SCAN: Linear ticket scan result
- condition: always
- claimed: artifacts/scan/phase-{N}-linear.json
- verified: Re-issue linear issue list command and confirm count matches.
- required_capabilities: [linear-cli]
- if_unavailable: block
- collection: Save `linear issue list --team <team> --updated-after <from> --json` output.

### E-OUTPUT-ADAPTER-SCAN: Output adapter (GitHub Project) scan result
- condition: always
- claimed: artifacts/scan/phase-{N}-output.json
- verified: Re-issue gh project item list command and confirm count matches.
- required_capabilities: [gh-cli]
- if_unavailable: block
- collection: Save adapter's scan_existing + scan_new_external output.

### E-SYNC-PLAN: Diff analysis plan
- condition: always
- claimed: artifacts/analysis/phase-{N}-sync-plan.json
- verified: Re-run diff analysis on scan.json and confirm plan categories match.
- required_capabilities: [bash]
- if_unavailable: block
- collection: Output of analyze step (new, status-change, context-update categories).

### E-USER-APPROVAL: User approval transcript for sync plan
- condition: always
- claimed: artifacts/approval/phase-{N}-approval.md
- verified: N/A (interactive user input cannot be re-run)
- required_capabilities: []
- if_unavailable: block
- collection: Save user response ("ok" / modifications / "cancel") with timestamp.

### E-SYNC-SIDE-EFFECTS: Mutation log from sync phase
- condition: always
- claimed: artifacts/sync/phase-{N}-side-effects.json
- verified: Query Linear + output adapter and confirm mutations applied.
- required_capabilities: [linear-cli, gh-cli]
- if_unavailable: manual_fallback
- collection: Record every SSOT update, issue create, comment add, status change.

### E-CUSTOMER-DOC: Customer-facing weekly document
- condition: always
- claimed: artifacts/docs/phase-{N}-customer-doc.md
- verified: Confirm Linear document exists with title pattern `[定例 <to>]`.
- required_capabilities: [linear-cli]
- if_unavailable: block
- collection: Save generated customer document draft pre-approval and final post-approval.
