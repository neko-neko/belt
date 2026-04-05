# flowrail

> Agent workflow engine. YAML で宣言された決定論的 state machine を、冪等かつ拡張可能に駆動する CLI ツール。

**Status**: Phase 1 in progress (Pipeline Lint/Fmt MVP)

## 概要

`flowrail` は LLM エージェント向けの軽量ワークフローエンジン。

- **Tiny by Constraint**: flowrail core が知る概念は 5 つのみ (State Machine / Artifact Lifecycle / Rule Set Resolver / 4 Core Primitive Checks / Hook Executor)
- **Linux 哲学**: "Do One Thing and Do It Well"
- **Rule Set Architecture**: ワークフロー知識は全て YAML 側の rule set に外出し
- **Separation by Audience**: agent CLI (`flowrail`) と human CLI (`flowrail-tui`) を Cargo workspace の別 crate に分離し、前者は security-first / minimal dependencies に徹する

## Crate 構成 (Cargo Workspace)

```
flowrail/
├── crates/
│   ├── flowrail-core/    # 📦 library
│   ├── flowrail/         # 🤖 agent CLI binary (`flowrail <resource> <verb>`)
│   └── flowrail-tui/     # 👤 human TUI binary (Phase 3)
├── docs/
│   ├── specs/            # 設計書
│   └── plans/            # 実装計画
├── catalog/              # Standard Rule Sets
├── examples/             # サンプル pipeline
└── schema/               # JSON Schemas
```

## ビルド

```bash
cargo build --workspace
```

agent CLI のみビルド (TUI 依存を一切取り込まない):
```bash
cargo build -p flowrail
```

## ドキュメント

- [Design Spec](docs/specs/2026-04-05-flowrail-cli-rule-set-architecture-design.md) — 設計書
- [Phase 1 Implementation Plan](docs/plans/2026-04-05-flowrail-phase1-pipeline-lint-fmt.md) — 実装計画
- [CLAUDE.md](CLAUDE.md) — プロジェクト規約 (Claude Code セッション向け)

## Related

- **Upstream inspiration**: [dotfiles](https://github.com/neko-neko/dotfiles) の `claude/skills/workflow-engine/`
- **Linear tracking**: [CLA-5](https://linear.app/neko-neko/issue/CLA-5)

## License

MIT OR Apache-2.0
