# belt

> Agent workflow engine. YAML で宣言された決定論的 state machine を、冪等かつ拡張可能に駆動する CLI ツール。

**Status**: Phase 1 in progress (`belt-dev` Pipeline Lint/Fmt MVP)

## 概要

`belt` は LLM エージェント向けの軽量ワークフローエンジン。

- **Tiny by Constraint**: belt-core が知る概念は 5 つのみ (State Machine / Artifact Lifecycle / Rule Set Resolver / 4 Core Primitive Checks / Hook Executor)
- **Linux 哲学**: "Do One Thing and Do It Well"
- **Rule Set Architecture**: ワークフロー知識は全て YAML 側の rule set に外出し
- **Separation by Audience (3 audience)**: developer CLI (`belt-dev`), agent runtime CLI (`belt`), human TUI (`belt-tui`) を Cargo workspace の別 crate / 別 binary に分離。それぞれ依存関係レベルで独立し、supply chain / binary size / security を独立に最適化する

## Binary Separation

| Binary | Audience | 役割 | Phase |
|--------|---------|------|-------|
| `belt-dev` | **Developer** (rule set 作者) | `pipeline lint` / `pipeline fmt` など authoring-time ツール | **Phase 1 (MVP)** |
| `belt` | **Agent** (LLM, CI/CD, script) | `run` / `state` / `snapshot` 等 workflow 実行 runtime | Phase 2 |
| `belt-tui` | **Human** (operator) | ratatui-based real-time state 可視化 | Phase 3 |

lint/fmt は authoring-time のツールであり agent runtime には含まれない (原則 8)。

## Crate 構成 (Cargo Workspace)

```
belt/
├── crates/
│   ├── belt-core/    # 📦 library: state machine, resolver, primitives, hooks
│   ├── belt-dev/     # 🛠  developer CLI binary: pipeline lint/fmt (Phase 1)
│   ├── belt/         # 🤖 agent runtime CLI binary (Phase 2)
│   └── belt-tui/     # 👤 human TUI binary (Phase 3)
├── docs/
│   ├── specs/        # 設計書
│   └── plans/        # 実装計画
├── catalog/          # Standard Rule Sets
├── examples/         # サンプル pipeline
└── schema/           # JSON Schemas
```

## ビルド

```bash
cargo build --workspace
```

developer CLI のみビルド (Phase 1 MVP):
```bash
cargo build -p belt-dev
```

agent runtime CLI のみビルド (Phase 2 以降、Phase 1 では placeholder):
```bash
cargo build -p belt
```

## ドキュメント

- [Design Spec](docs/specs/2026-04-05-belt-cli-rule-set-architecture-design.md) — 設計書
- [Phase 1 Implementation Plan](docs/plans/2026-04-05-belt-phase1-pipeline-lint-fmt.md) — 実装計画
- [CLAUDE.md](CLAUDE.md) — プロジェクト規約 (Claude Code セッション向け)

## Related

- **Upstream inspiration**: [dotfiles](https://github.com/neko-neko/dotfiles) の `claude/skills/workflow-engine/`
- **Linear tracking**: [CLA-5](https://linear.app/neko-neko/issue/CLA-5)

## License

MIT OR Apache-2.0
