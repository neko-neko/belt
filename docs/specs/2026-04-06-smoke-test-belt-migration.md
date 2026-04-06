# smoke-test belt 移植設計書

## 概要

既存の `/smoke-test` スキルを belt パイプライン + スキルの2層構造に移植する。belt（決定論的状態機械）がフェーズ遷移とゲートチェックを担い、スキル（非決定論的実行ガイド）が LLM の browser 操作・シナリオ生成・判定ロジックを担う。

## 成果物

| ファイル | 役割 | 言語 |
|---------|------|------|
| `pipelines/smoke-test.yml` | belt pipeline 定義（4フェーズ、args、gate、validate） | English |
| `skills/smoke-test/SKILL.md` | LLM 実行ガイド（フロー + ルール + フェーズ→リファレンスのマッピング） | English |
| `skills/smoke-test/references/server-detection.md` | サーバー自動検出テーブル + 起動手順 | English |
| `skills/smoke-test/references/scenario-generation.md` | 5基本観点 + 拡張ロジック + adversarial probe パターン | English |
| `skills/smoke-test/references/report-template.md` | smoke-test-report.md の完全テンプレート | English |
| `skills/smoke-test/references/vrt-detection.md` | VRT ツール検出 + diff ハンドリング | English |
| `skills/smoke-test/references/e2e-flaky-detection.md` | E2E スイート検出 + 2-pass フレーキー判定マトリクス | English |

## スコープ

### 含むもの

- 4ステップ → 4フェーズのフラットパイプライン
- 既存の9引数のマッピング
- gate（成果物存在の機械的検証）
- validate（adversarial probe、シナリオ品質の LLM 判断）
- スタンドアロン実行 + feature-dev からの `uses:` 参照（単一 pipeline.yml）
- リファレンスファイル分離による保守性確保

### 含まないもの

- 監査手法（done-criteria、phase-auditor dispatch）— ユーザースキル側の責務
- browser-use 必須の制約 — 緩和済み。E2E テストスイート（Playwright, Cypress）も許容
- belt-core の変更

## Pipeline 定義

### `pipelines/smoke-test.yml`

```yaml
name: smoke-test
version: 1
args:
  diff_base:    { type: string, default: "HEAD~1" }
  design:       { type: string, default: "" }
  server:       { type: string, default: "" }
  port:         { type: number, default: 0 }
  skip_vrt:     { type: bool, default: false }
  skip_e2e:     { type: bool, default: false }
  adhoc_only:   { type: bool, default: false }
  full_e2e:     { type: bool, default: false }
  perspectives: { type: string, default: "" }

phases:
  - id: env-setup
    description: "Start dev server and verify it is accessible."
    config:
      skill: "/smoke-test"
    gate:
      - cmd: "curl -sf http://localhost:${args.port:-3000}/ > /dev/null"

  - id: adhoc-test
    description: "Generate and execute ad-hoc smoke test scenarios via browser."
    config:
      skill: "/smoke-test"
    artifacts:
      - "smoke-test-report.md"
    gate:
      - file_exists: "smoke-test-report.md"
      - file_exists: "smoke-*.png"
    validate:
      - "At least one adversarial probe executed and documented in report"
      - "Test scenarios cover changes from diff (not just generic checks)"
    confirm: true

  - id: vrt-check
    description: "Run VRT diff check if VRT tooling is detected."
    when: "!args.skip_vrt"
    config:
      skill: "/smoke-test"

  - id: e2e-detection
    description: "Run E2E test suite with flaky detection (2-pass execution)."
    when: "!args.skip_e2e"
    config:
      skill: "/smoke-test"
```

### 設計判断

- **env-setup の gate**: `curl` でサーバー応答を機械的に検証。ポートは args またはスキルの自動検出に依存
- **adhoc-test**: gate で成果物（レポート + スクリーンショット）の存在を検証。validate で adversarial probe とシナリオ品質を LLM 判断。`confirm: true` で明示的遷移
- **vrt-check / e2e-detection**: gate なし。VRT ツールや E2E スイートが存在しない場合もあるため、検出・実行・判定は全てスキル側の責務。belt は `when:` でスキップ制御のみ
- **adhoc_only**: `--arg skip_vrt=true --arg skip_e2e=true` で代替可能。専用の `when:` は不要
- **feature-dev からの参照**: `uses: ./pipelines/smoke-test.yml` で sub-pipeline として組み込める

## Skill 構成

### ファイル構造

```
skills/smoke-test/
├── SKILL.md
└── references/
    ├── server-detection.md
    ├── scenario-generation.md
    ├── report-template.md
    ├── vrt-detection.md
    └── e2e-flaky-detection.md
```

### SKILL.md の構造

```
# Smoke Test

## Overview
  - Browser-based UI verification for code changes
  - References Belt Protocol (skills/belt-agent/SKILL.md)
  - Output: smoke-test-report.md + smoke-*.png
  - Status: PASS / FAIL / PAUSE

## Phase Map

| Phase | What to do | Reference |
|-------|-----------|-----------|
| env-setup | Start dev server | references/server-detection.md |
| adhoc-test | Generate & execute scenarios | references/scenario-generation.md, references/report-template.md |
| vrt-check | Run VRT if tooling detected | references/vrt-detection.md |
| e2e-detection | Run E2E with flaky detection | references/e2e-flaky-detection.md |

## Phase: env-setup
  - Read server-detection.md, start server, verify with curl
  - --server/--port override
  - Timeout 30s → PAUSE

## Phase: adhoc-test
  - Read scenario-generation.md, generate scenarios from diff + design + perspectives
  - Execute via browser (reconnaissance-then-action)
  - Screenshot per scenario: smoke-<name>.png
  - Write report per report-template.md
  - Retry failed scenarios max 2 times

## Phase: vrt-check
  - Read vrt-detection.md
  - Not detected → skip phase
  - Diff found → present to user for approval

## Phase: e2e-detection
  - Read e2e-flaky-detection.md
  - Not detected → skip phase
  - --full-e2e vs changed-files-only
  - Flaky = PASS (report only), implementation failure = FAIL

## Red Flags
  - Never: silent PASS on failure, VRT baseline update without approval
  - Always: screenshots, server cleanup
```

### SKILL.md の設計判断

- Phase Map テーブルでフェーズ→リファレンスの対応を明示
- 各フェーズセクションは3-5行の手順概要のみ。詳細はリファレンスに委譲
- リファレンスは独立して読み書きできる自己完結ドキュメント
- SKILL.md 本体のコンテキスト消費を最小化し、LLM は必要なリファレンスのみ読み込む

### リファレンスファイルの内容

#### `references/server-detection.md`

サーバー起動コマンドの自動検出テーブル:

| 検出対象 | 条件 | コマンド | デフォルトポート |
|---------|------|---------|---------------|
| package.json | `scripts.dev` 存在 | `npm run dev` | 3000 or 5173 |
| package.json | `scripts.start` 存在 | `npm start` | 3000 |
| Makefile | `dev` ターゲット存在 | `make dev` | 8080 |
| manage.py | ファイル存在 | `python manage.py runserver` | 8000 |
| docker-compose.yml | ファイル存在 | `docker compose up` | ports マッピング |

`--server` / `--port` 指定時はテーブルをスキップ。検出不可時は PAUSE。

#### `references/scenario-generation.md`

5基本観点:
1. ナビゲーション確認
2. ユーザーインタラクション
3. エラー不在確認（コンソール・ネットワーク）
4. レスポンシブ確認（desktop: 1280x720, mobile: 375x667）
5. 影響波及テスト

観点拡張の3パス:
- `--design` 指定時: 設計書からテスト観点・Must-Verify Checklist・Impact Analysis を抽出
- `--perspectives` 指定時: review エージェント（security, performance, coverage）を並列 dispatch
- 両方指定時: 設計書側を優先、エージェント側の重複を除外して統合

Adversarial probe パターン（最低1件必須）:
- 空入力 / 異常入力
- 同一操作の連続実行（idempotency）
- 存在しない対象への操作
- refresh 後の状態保持確認

#### `references/report-template.md`

`smoke-test-report.md` の完全テンプレート。セクション構成:
- ヘッダ（Date, Diff Base, Server, Status）
- Step 2: Ad-hoc Smoke Test（シナリオテーブル + Evidence Log）
- Step 3: VRT Diff Check
- Step 4: E2E + Flaky Detection（テスト結果テーブル + フレーキーテーブル + 実装起因テーブル）

Overall Status 判定ロジック:

| 条件 | ステータス |
|------|----------|
| 全ステップ PASS（フレーキー許容） | PASS |
| adhoc-test で2回リトライ後も失敗 | FAIL |
| adversarial probe 未実行 | FAIL |
| E2E で実装起因の失敗（2回とも FAIL） | FAIL |
| サーバー起動不可 | PAUSE |

#### `references/vrt-detection.md`

VRT ツール検出テーブル:

| 検出対象 | 判定条件 | 実行コマンド |
|---------|---------|------------|
| Playwright snapshots | `toMatchSnapshot` 使用 | `npx playwright test --grep snapshot` |
| reg-suit | `.reg/` or `regconfig.json` | `npx reg-suit run` |
| Storycap + reg-suit | `storycap` in devDeps | `npx storycap && npx reg-suit run` |
| Loki | `loki` in devDeps or `.lokirc` | `npx loki test` |

未検出 → スキップ。差分発生時 → ユーザーに提示して承認/拒否。

#### `references/e2e-flaky-detection.md`

E2E スイート検出テーブル:

| 検出対象 | 判定条件 | 実行コマンド |
|---------|---------|------------|
| Playwright | `playwright.config.*` | `npx playwright test` |
| Cypress | `cypress.config.*` | `npx cypress run` |
| その他 | `scripts.test:e2e` | `npm run test:e2e` |

2-pass フレーキー判定マトリクス:

| 1回目 | 2回目 | 判定 | アクション |
|-------|-------|------|----------|
| PASS | PASS | 安定 | 通過 |
| FAIL | FAIL | 実装起因 | FAIL。修正提案生成 |
| PASS | FAIL | フレーキー | 報告のみ（ブロック不可） |
| FAIL | PASS | フレーキー | 報告のみ（ブロック不可） |

## 設計判断の根拠

### なぜ browser-use 必須を緩和したか

既存スキルは browser-use CLI を必須とし、E2E テストスイートでの代替を禁止していた。belt は汎用ツールであり、UI を持たないプロジェクトでも smoke-test pipeline を使えるべき。browser-use を推奨しつつ、Playwright / Cypress 等の E2E スイートも gate として許容する。

### なぜ adversarial probe を validate にしたか

adversarial probe の実行有無は「レポートに記録があるか」で判定可能だが、その品質（本当に adversarial か、形式的なテストではないか）は LLM の判断が必要。pipeline の validate に入れることで、belt の HARD-GATE（validate 検証義務）が自動的に適用される。スキル側で別途 HARD-GATE を重ねる必要はない。

### なぜ vrt-check / e2e-detection に gate を置かないか

VRT ツールや E2E スイートが存在しないプロジェクトでは、これらのフェーズは「ツール未検出 → スキップ」が正常動作。gate を置くと未検出時に FAIL になってしまう。検出・実行・判定は全てスキル側の非決定論的処理に任せ、belt は `when:` でユーザー指定のスキップのみ制御する。

### なぜリファレンスを分離したか

既存の SKILL.md（378行）は検出テーブル・レポートテンプレート・判定マトリクス等のリファレンスデータがフローと混在しており、保守が困難。リファレンスを独立ファイルに分離することで:
- 各ファイルが1つの関心事に集中
- フレームワーク追加時に該当ファイルのみ更新
- LLM は必要なリファレンスのみ読み込み、コンテキスト消費を最小化

## 関連

- [BELT-20](https://linear.app/neko-neko/issue/BELT-20) — belt 再設計 MVP
- `skills/belt-agent/SKILL.md` — Belt Protocol 汎用スキル（本スキルが前提とする）
- `docs/specs/2026-04-06-belt-agent-skill.md` — Belt Protocol 汎用スキル設計書
- `examples/feature-dev/pipeline.yml` — feature-dev の belt 化サンプル（`uses: ./pipelines/smoke-test.yml` の参照元候補）
