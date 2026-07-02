---
date: 2026-07-02
status: Draft
depends-on:
  - plugins/belt/skills/feature-dev/pipeline.yml
  - plugins/belt/skills/bug-fix/pipeline.yml
  - plugins/belt/skills/handover/checkpoint.yml
  - crates/belt-core/src/expander.rs
  - crates/belt-core/src/parser.rs
  - plugins/belt-agent/skills/protocol/SKILL.md
---

# belt pipeline 分割設計 — design / diagnose / build / verify

## 背景

2026-07-02 の belt 全域分析 (9-agent workflow + orchestrator 検証) で、feature-dev pipeline の実使用データを定量化した:

- belt pipeline 利用は 2026-04-22〜05-07 に集中し、以後 feature-dev の起動はゼロ (5/30 の bug-fix 1 本のみ)
- 完走 run の実測: 11.7〜27.0h / plan.md 1,065〜1,641 行 / review findings 112〜128K
- 全完走 run が execute 直前に /clear→/belt:resume を強制され、session 分断が構造化されていた
- backend/CLI repo では monkey-test / dogfood が構造的に SKIP-all (実測 2〜8 分のセレモニー)
- 5/7 以降、rakmy_server は 25 commits を belt 外で出荷しつつ design.md / plan.md の文書習慣だけを残した

結論: **成果物規律 (design/plan 文書) は定着し、10-phase の runtime 強制だけが捨てられた**。改善は削除でも修理でもなく、サイズ選択肢の提供 = pipeline 分割である。

## Decisions

| # | 決定 | 根拠 |
|---|------|------|
| D1 | 合成機構は `invoke.pipeline` (inline 展開) | 唯一の現存合成機構。expander が `{parent}/{sub}` にフラット展開するため合成後も**単一 run** (status / resume / narrative は従来どおり 1 本)。checkpoint.yml で実績あり |
| D2 | bug-fix も同時分割 | build 段 (execute → code-review → integrate) が feature-dev と同一構成のため、共有 `build` に 1 本化すると criteria 物理複製 (shared_criteria_parity) が解消される |
| D3 | 命名は design / diagnose / build / verify | 短い動詞で対にする Linux 哲学準拠。feature-dev / bug-fix は合成エントリポイント名として維持 |
| D4 | 付随修正は codex timeout + 自動 skip のみ同梱 | plan 粒度ガード / grill-me 上限は follow-up。e2e ゲートは verify の sub 化で自然解決 |
| D5 | expander を再帰展開に拡張 (belt-core 唯一のコード変更) | build 内の verify(sub) を合成親から参照すると 2 段ネストになる。pain-driven: 合成の実需要が顕在化した |

## 新スキル構成

```
plugins/belt/skills/
├── design/    (新) design → test-scenarios → spec-review → plan     [feature 系上流]
├── diagnose/  (新) rca → fix-plan → fix-plan-review                 [bug 系上流]
├── build/     (新) execute → code-review → verify(sub, when:e2e) → integrate  [共有]
├── verify/    (新) monkey-test → dogfood                            [共有・e2e 専用]
├── feature-dev/ (改) design(sub) + checkpoint(sub) + build(sub) の 3-phase 合成
├── bug-fix/     (改) diagnose(sub) + checkpoint(sub) + build(sub) の 3-phase 合成
└── handover/checkpoint.yml (無変更)
```

- 各新 pipeline は単体で `belt-agent init` 可能 (args: e2e / codex を自前宣言)
- 利用パターン: 小粒 = `belt:build` 単体 (手書き plan 可、confirm 3〜4 回) / 中粒 = `belt:design` → 別セッションで `belt:build` (pipeline 境界が /clear の自然な切れ目、checkpoint 不要) / 大型 = 合成 feature-dev (従来同等の保証)
- 合成側の args (e2e / codex) は `with: { e2e: "args.e2e", codex: "args.codex" }` で sub へ伝播
- regate topology は各 pipeline 内で閉じる (spec-review → test-scenarios は design 内、code-review → execute は build 内)

## belt-core 変更: expander 再帰展開

現行 `expand_pipeline` は top-level phases の `Invoker::Pipeline` のみ展開し、sub-pipeline 内の `Invoker::Pipeline` は未解決のまま残す。これを再帰展開に拡張する:

- namespace は `{parent}/{sub}/{subsub}` 連結 (例: `build/verify/monkey-test`)
- 循環検出: 展開スタックに canonical path の visited set を持ち、再訪で `InvalidPipeline` エラー
- 深さ上限 4 (超過はエラー)
- `with` 置換は各レベルの自スコープのみに適用 (I1 の parent-scope 規則 6493cf2 を維持)
- 継承規則 (最終 sub-phase への gate/regate/validate append、config merge、when 伝播) は再帰の各レベルで現行規則をそのまま適用
- lint も再帰先の存在・妥当性を検証する

## dual-format: 単体実行ファイルの sub 参照

design / diagnose / build / verify の pipeline.yml は Pipeline 形式 (`args:`) で書き、単体 init と `invoke.pipeline` 参照を同一ファイルで賄う。`parse_sub_pipeline` は `SubPipeline` 型 (deny_unknown_fields なし) で読むため `args:` は無視される見込み。

- **Spike (plan 先頭タスク)**: serde-saphyr が unknown field を無視することを検証する
- Fallback: 拒否される場合は `SubPipeline` に `#[serde(default)] args` を追加、または parse 統合
- checkpoint.yml は SubPipeline 形式のまま (単体実行しない)

## artifact 受け渡し

build は上流産物 (design_doc / plan_doc / rca_doc / fix_plan_doc) を **consumes 宣言しない**:

- 単体 lint で上流 produces が存在せず通らない
- 上流が design / diagnose の 2 系統あり固定できない

合成実行時は inline 展開の単一 run 内で notes も `belt://current` で自然に共有される。単体実行時は build SKILL.md が入口で「docs/features/*/plan.md (または手持ち plan) の所在確認」を指示する。

## criteria / references の再配置

criteria は担当 pipeline へ `git mv` (履歴保持):

| 移設先 | criteria |
|--------|----------|
| design/ | design.md, test-scenarios.md, spec-review.md, plan.md |
| diagnose/ | rca.md, fix-plan.md, fix-plan-review.md |
| build/ | execute.md, code-review.md, integrate.md (**1 本化**) |
| verify/ | monkey-test.md, dogfood.md |

references も同様: brainstorming/writing-plans supplement + path-convention → design、worktrunk supplement + evidence-catalog → build (**1 本化**)、monkey/dogfood supplement → verify。

execute.md / code-review.md / evidence-catalog.md の物理複製が解消されるため **shared_criteria_parity.rs は削除**する。

## 序数除去 + ドキュメント修正

- 全 criteria の `# Phase N (id) Done Criteria` 見出し → `# id Done Criteria`
- references / monkey-test SKILL.md の「Phase N」言及 → phase id 参照 (共有スキルは成果物名で依存を表現)
- **protocol SKILL.md の invoke.pipeline 節を実装に合わせて修正**: 「nested belt-agent run を初期化し black-box として扱う」→「expander が inline 展開し、orchestrator は展開済み leaf phase を `next` で受け取る (nested init は発生しない)」。本 spec 作成過程で発見した doc drift。あわせて Commands 表に locate を追加
- AGENTS.md の陳腐化した `uses:` 記述 (YAML 例含む) を invoke.pipeline に更新、Crate 構成 / skill 一覧を新構成に更新

## codex timeout + 自動 skip

code-review / spec-review SKILL.md の codex 節に追記:

> 並列 batch の他 reviewer が全員完了した時点で codex が未応答なら skip し、merged findings に `{observation: "codex", severity: "low", description: "codex adversarial pass skipped (no response)"}` を 1 件記録する。

LLM に時計はないため、経過時間でなく**他 reviewer 完了を基準**にする。実測で codex 無応答 → 手動 skip が常態化していた摩擦 (「codex 終わらないね」) の解消。

## テスト / lock 更新

- expander 再帰の unit tests (2 段展開 / 循環 / 深さ上限 / with スコープ) + belt-core.yml へ scenario 追加
- feature_dev_refresh.rs / bug_fix_refresh.rs を新構成 (3 sub 参照の展開形) に改訂
- 新 pipeline 4 本の shape lock は `pipeline_split_refresh.rs` 1 本に統合
- shared_criteria_parity.rs 削除、lock-ledger.md 更新
- plugin version 0.2.0 → 0.3.0 (plugin.json / marketplace.json)

## Out of Scope (follow-up)

- plan 粒度ガード (criteria/plan.md への task 数上限 criterion)
- grill-me ラウンド上限
- `--inherits-from` の protocol 教育
- bug-fix 特有の deep-debug 最適化
- 旧 run (.belt/runs) の migration (旧 pipeline_file 絶対パス参照は expand 時に旧ファイルが無ければ落ちるが、実利用 run は全て COMPLETED / 放棄済みで実害なし)

## Open Questions

1. dual-format spike の結果 (fallback 選択は plan で確定)
2. verify 単体の user-invocable 可否 → **true** とする (UI repo で検証だけ回す用途)
3. sub-pipeline 内部の regate ターゲット (build 内 code-review の `regate: [execute]`) が展開時に `{parent}/execute` へ namespace リネームされるか。現行 expander の未踏挙動 (checkpoint.yml は regate なし)。リネームされない場合は expander 再帰化と同時に実装する (plan の spike で確認)
