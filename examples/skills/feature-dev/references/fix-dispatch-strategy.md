# Fix Dispatch Strategy

When an audit phase returns FAIL, the orchestrator applies fixes using the
executor appropriate for the failed work phase.

## Dispatch Table

| Work Phase | Fix Executor | Strategy |
|------------|-------------|----------|
| design | Orchestrator | Re-read investigation record, rescan codebase if info is missing |
| spec-review | Orchestrator | Edit design doc directly based on audit findings |
| plan | Orchestrator | Edit plan doc directly based on audit findings |
| plan-review | Orchestrator | Edit plan doc directly based on audit findings |
| execute | `feature-implementer` subagent | Decompose fix instructions into TDD tasks, launch with full task context |
| doc-audit | Orchestrator or `feature-implementer` | depends-on fixes → Edit directly; content fixes → invoke /doc-check; new docs → feature-implementer |
| smoke-test | `feature-implementer` subagent | Bug fixes to implementation code |
| code-review | `feature-implementer` subagent | Apply review finding fixes |
| test-review | `feature-implementer` subagent | Apply test code fixes |

## Fix Context Template

When dispatching a subagent for fixes, inject:

```
## Fix Context

### Failed Criteria
{criterion ID, severity, and detail from the audit verdict}

### Fix Instructions
{the auditor's recommended fix — what to change, where, and why}

### Current State
{relevant git diff or file content showing the current state}

### Verification
After applying the fix, verify by:
{the criterion's verification steps from done-criteria}
```

## Rules

- The orchestrator MUST NOT fix on behalf of a subagent executor.
  If the dispatch table says `feature-implementer`, launch one.
- Fixes that produce code changes will trigger regate on the next `belt-agent step`
  (belt handles this automatically via the pipeline's regate configuration).
- If a fix is blocked (cannot be applied), report `blocked` status and PAUSE.
