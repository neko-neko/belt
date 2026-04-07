# linear-add / linear-cleanup Skill Migration to belt repo

**Status**: Approved
**Date**: 2026-04-07

## Summary

Migrate `linear-add` and `linear-cleanup` SKILL.md from dotfiles (`~/go/src/github.com/neko-neko/dotfiles/claude/skills/`) to the belt repo (`examples/skills/`), and create symlinks from `~/.claude/skills/` to maintain Claude Code skill discovery.

## Background

The `linear-refresh` pipeline in `examples/skills/linear-refresh/` depends on two domain skills (`linear-add`, `linear-cleanup`) that are currently managed in a separate dotfiles repository. This creates a split source of truth — the pipeline definition and sub-agent references live in belt, but the analysis guidelines they invoke live elsewhere.

Consolidating these skills into the belt repo makes the example self-contained and establishes belt as the single source of truth for all linear-refresh related artifacts.

## Scope

### In Scope

- `linear-add/SKILL.md` — new ticket candidate detection criteria
- `linear-cleanup/SKILL.md` — structural cleanup analysis guidelines

### Out of Scope

- `slackcli` — general-purpose tool, not linear-refresh specific
- `linear-cli` — managed as a plugin, not in dotfiles
- dotfiles `linear-refresh/SKILL.md` — old monolithic orchestrator, replaced by belt's lean orchestrator

## Design

### File Layout (belt repo)

```
examples/skills/
├── linear-refresh/          # existing (no changes)
│   ├── pipeline.yml
│   ├── SKILL.md
│   ├── belt.toml
│   ├── linear-add.yml
│   ├── linear-cleanup.yml
│   └── references/
├── linear-add/              # new
│   └── SKILL.md
└── linear-cleanup/          # new
    └── SKILL.md
```

### Symlinks

```
~/.claude/skills/linear-add     → <belt-repo>/examples/skills/linear-add
~/.claude/skills/linear-cleanup → <belt-repo>/examples/skills/linear-cleanup
```

### Dotfiles Removal

```
~/go/src/github.com/neko-neko/dotfiles/claude/skills/linear-add/      # delete
~/go/src/github.com/neko-neko/dotfiles/claude/skills/linear-cleanup/  # delete
```

### What Does NOT Change

| Target | Reason |
|--------|--------|
| SKILL.md content | Already handles both standalone and linear-refresh modes (CollectedContext presence check) |
| Reference files in linear-refresh | `/linear-cleanup`, `/linear-add` invokes resolve via symlinks |
| linear-refresh pipeline.yml | Sub-pipeline `config.skill` uses skill invoke, unaffected |
| linear-refresh SKILL.md | Lean orchestrator dispatches sub-agents, no direct skill loading |

### Skill Resolution Flow (after migration)

```
sub-agent: "Invoke /linear-cleanup"
  → Claude Code skill discovery
    → ~/.claude/skills/linear-cleanup/  (symlink)
      → <belt-repo>/examples/skills/linear-cleanup/SKILL.md
```

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| belt repo path change breaks symlinks | Skill invoke fails | Symlinks use absolute paths; re-create on relocation |
| dotfiles old linear-refresh references these skills | Reference failure | Uses `/skill-name` invoke pattern, not file paths — confirmed unaffected |
