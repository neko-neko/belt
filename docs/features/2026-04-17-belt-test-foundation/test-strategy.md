# belt-test-foundation (F1) — Test Strategy

F1 deliverable が正しく機能することを確認するための test strategy。ISTQB 4 技法 + ISO 25010 品質特性評価 + Must-Verify Checklist (MV-01 〜 MV-30) への mapping。`--e2e` off のため scenarios.yml (feature-internal) は生成しない。

## Test Design Techniques

### 1. Equivalence Partitioning

| ID | 適用対象 | Valid class | Invalid class |
|---|---|---|---|
| **EP-01** | `docs/testing/scenarios/*.yml` schema | 全必須 field (`id`, `category`, `severity`, `given`, `when`, `then`) 揃い、`severity` が enum (`critical|high|medium|low`) 内、`id` が kebab-case | required field 欠落、`severity` が enum 外 (`trivial` 等)、YAML syntax error、unknown top-level key |
| **EP-02** | `docs/testing/lock-ledger.md` entries の `locks-file:` | 指定パスのファイルが repo root 基準で実在 | 指定パス不在、相対パス基準混在 |
| **EP-03** | Rust doc-comment `/// scenario: <id>` | 正規表現 `^\s*///\s+scenario:\s+(\S+)\s*$` に match、ID が scenarios.yml 内に存在 | typo (`senario:` / `scenraio:`)、leading slash 欠 (`// scenario:`)、scenarios.yml に ID 未登録 (orphan) |
| **EP-04** | `view_test.rs` の `filetime::set_file_mtime` 設定値 | phase_start 前の時刻 (skip 対象) / phase_start 後の時刻 (pick 対象) | システム API 未サポート FS (本 MVP scope 外) |

### 2. Boundary-Value Analysis

| ID | 適用対象 | 境界値 |
|---|---|---|
| **BV-01** | scenarios.yml の `scenarios` array エントリ数 | 0 (空配列で schema valid), 1 (最小有意), 多数 (stub module で 0 許容 vs 本格 module で N) |
| **BV-02** | scenario `id` 文字列長 | kebab-case 最短 (3 文字、例: `a-b`)、実用範囲最長 (例: 48 文字前後) |
| **BV-03** | lock-ledger.md の entry 数 | 0 (初期状態不許容)、1 (最小)、6 (F1 時点で 5 既存 + 1 新規 = 6 要求) |
| **BV-04** | `filetime::set_file_mtime` の epoch 値 | `UNIX_EPOCH` (0)、`SystemTime::now()` 近辺、precision ns 差の 2 ファイル mtime 境界 |

### 3. Decision Tables

| ID | 対象 | 入力次元 | 判定 |
|---|---|---|---|
| **DT-01** | scenario ID ↔ Rust doc-comment binding | (YAML に ID 存在: y/n) × (Rust に `/// scenario: X` 存在: y/n) | 4 象限: (y,y) pass / (y,n) orphan-yml fail / (n,y) orphan-rust fail / (n,n) not-applicable |
| **DT-02** | lock-ledger entry ↔ locks-file ↔ 実 lock test | (ledger entry: y/n) × (locks-file 実在: y/n) × (当該 test `cargo test` pass: y/n) | 8 象限、full-y 以外は contract test fail or F2/F3 で human investigation |
| **DT-03** | `belt lint` CLI 契約 | (入力: valid/invalid/nonexistent/ambiguous) × (exit code: 0/1/2) × (stderr: "ok"/"duplicate"/他) | 既存 5 test の契約保持、doc-comment 付与後も assertion behavior 不変 |

### 4. State-Transition Testing

| ID | 遷移シナリオ |
|---|---|
| **ST-01** | scenarios.yml entry 追加 → Rust doc-comment 未追加 → `cargo test --test scenarios_contract` fail (orphan-yml 検出) → Rust doc-comment 追加 → contract test pass |
| **ST-02** | scenarios.yml entry 削除 → Rust doc-comment 残存 → `cargo test --test scenarios_contract` fail (orphan-rust 検出) → 人 review で Rust doc-comment 削除 or scenarios.yml 追加戻し → contract test pass |
| **ST-03** | view_test.rs: 置換前 (`thread::sleep(20ms)` で mtime 順序 seeding) → 置換中 (filetime explicit mtime 設定) → 置換後 (assertion behavior 保持、100 連続 pass) の 3 段階 |
| **ST-04** | pilot audit decision label 推移: unresolved → (scenarios.yml に対応 ID あり) kept / (redundant-with-X) deleted / (similar to Y) merged / (high-level extract) abstracted。reason label は audit-template.md の許容集合内 |

## Quality Characteristics (ISO 25010)

| 特性 | Relevance | 理由 |
|---|---|---|
| **Functional suitability** | in-scope | scenarios_contract.rs が SSOT ↔ Rust binding の drift を catch、F1 の存在意義そのもの |
| **Performance efficiency** | in-scope | scenarios_contract.rs は lint スケール (<1s)、CI gate としての速度要件 |
| **Compatibility** | out-of-scope | 外部プロトコル・API・他ツール連携なし |
| **Usability** | out-of-scope | 内部開発者向け、UI なし |
| **Reliability** | in-scope | view_test.rs flaky fix により mtime 決定論化、audit 中の noise 排除 |
| **Security** | out-of-scope | production code 変更ゼロ、新 dep ゼロ、秘密情報への影響なし |
| **Maintainability** | in-scope | F2/F3 が F1 作者不在で audit-template.md + lock-ledger.md のみで実行可能なこと |
| **Portability** | in-scope | Unix (macOS + Linux) 前提、Windows は MVP scope 外 (CLAUDE.md 継承) |

## Priority Matrix

| 特性 | 臨界度 | 根拠 |
|---|---|---|
| Functional suitability | **critical** | SSOT ↔ Rust binding が drift すると audit の north star が崩壊、F1 の主目的消失 |
| Maintainability | **critical** | F2/F3 handover 失敗 = F1 の存在意義消失、audit-template.md の self-documentation が鍵 |
| Reliability | **high** | view_test.rs flaky が audit 中の decision-making を汚染すると質が低下 |
| Performance efficiency | medium | < 1 秒は目安、多少超過しても F1 成立 (ただし CI gate に置く限り速度は継続的関心事) |
| Portability | low | Unix-only 明示済、Windows の対応は MVP scope 外で合意済 |
| Compatibility / Usability / Security | N/A | out-of-scope (上記 Quality Characteristics 参照) |

## Non-Functional Requirements

- **NFR-01 (Performance)**: `cargo test --test scenarios_contract -- --nocapture` wall-clock real time < 1.0 秒
- **NFR-02 (Reliability)**: `for i in {1..100}; do cargo test -p belt-core --test view_test --quiet || break; done` で 100 連続 pass (flaky 消失の empirical 確認)
- **NFR-03 (Maintainability)**: Phase 3 spec-review の external reviewer が audit-template.md + lock-ledger.md のみを参照して「F2 として pilot audit 1 ファイル (任意) を擬似実行可能」と判定
- **NFR-04 (Functional suitability)**: scenarios_contract.rs が以下 3 種の drift injection で全て fail する (robustness probe):
  - scenarios.yml に ID 追加 / Rust 未追加 → fail
  - Rust doc-comment 追加 / scenarios.yml 未追加 → fail
  - lock-ledger `locks-file:` を不在 path に変更 → fail
- **NFR-05 (Portability)**: Linux (Ubuntu 22.04 以降) + macOS (14+) の両方で `cargo test --workspace` 全 green (release.yml が target に指定する x86_64-linux-gnu / aarch64-linux-gnu / x86_64-darwin / aarch64-darwin 全 4 architecture)

## Must-Verify Mapping

design.md の MV-01 〜 MV-30 を、上記技法 / NFR / 手動検査に mapping。

| MV ID | Verification Method | Assertion / Evidence |
|---|---|---|
| MV-01 | 手動 (file 存在 + 目視) | `docs/testing/README.md` 存在 + 境界宣言を含む |
| MV-02 | EP-01 + BV-01 | scenarios_contract.rs が `belt.yml` を serde_saphyr で parse 成功 |
| MV-03 | EP-01 + 手動 | `belt-agent.yml` stub が schema valid (具体形は Phase 2 記述通り) |
| MV-04 | EP-01 + 手動 | `belt-core.yml` config module entries + 他 module stub |
| MV-05 | EP-02 + 手動 | lock-ledger.md frontmatter に `locks-file:` 各 entry |
| MV-06 | 手動 | audit-template.md に decision tree + reason label 列挙 + duplication 候補表 |
| MV-07 | NFR-01 + `cargo test` | scenarios_contract.rs が workspace test で pass、単独 <1s |
| MV-08 | EP-01 | 3 scenarios.yml を serde_saphyr で parse 成功 |
| MV-09 | DT-01 (y,n) 象限 | scenarios.yml の全 ID が Rust doc-comment 参照先に存在 |
| MV-10 | DT-01 (n,y) 象限 | Rust `/// scenario: X` の X が全て scenarios.yml に登録 |
| MV-11 | EP-02 + DT-02 | lock-ledger `locks-file:` 指定ファイル全実在 |
| MV-12 | EP-03 (typo 耐性) | typo `senario:` は正規表現 match しない + `scenario: X` (leading slash 欠) も match しない |
| MV-13 | EP-03 + 手動 | `cli_test.rs` の 5 test 全てに `/// scenario: belt-*` 付与 |
| MV-14 | EP-03 + 手動 | `config_test.rs` の 6 test 全てに `/// scenario: belt-core-config-*` 付与 |
| MV-15 | 手動 | audit-report.md に pilot 各 test の judgement + reason 記載 |
| MV-16 | 手動 | audit-report で使用した reason label が audit-template.md 列挙集合の subset |
| MV-17 | 手動 (git diff) | `feature_dev_refresh.rs` の diff ゼロ (lock-ledger.md entry 追加のみ) |
| MV-18 | 手動 | lock-ledger.md の `feature_dev_refresh.rs` entry に cross-coupling (shared_criteria_parity / shared_filter_parity / bug_fix_refresh / review_skills_refresh) 明記 |
| MV-19 | EP-04 + BV-04 | `view_test.rs` の 6 箇所 `thread::sleep` が `filetime::set_file_mtime` 置換済 |
| MV-20 | NFR-02 | 100 連続 `cargo test -p belt-core --test view_test` pass |
| MV-21 | 直接実行 | `cargo test --workspace` exit 0、全 test green |
| MV-22 | 直接実行 | `cargo clippy --workspace -- -D warnings` exit 0 |
| MV-23 | 直接実行 | `cargo fmt --all -- --check` exit 0 |
| MV-24 | DT-01 部分 | 5 既存 lock test が未改変 (`git diff` で確認) かつ `cargo test` で全 pass |
| MV-25 | 手動 | `.belt/runs/<run_id>/notes/phase-design.md` 存在、frontmatter (`phase: design` / `run_id: ...`) + 4 節 (`## Decisions` / `## Concerns` / `## Directives` / `## Observations`) |
| MV-26 | Phase 3/5/6 責務 | F1 design-gate 時点では未検証、参考列挙 (plan / execute / code-review phase で各 note 作成を確認) |
| MV-27 | 直接実行 | `git branch --list feature/2026-04-17-belt-test-foundation` exit 0 |
| MV-28 | 直接実行 (済) | `cargo test --workspace` = 387/387 pass (F1 進入前に確認済、本 phase-design commit 時点で再確認) |
| MV-29 | 手動 | `docs/testing/README.md` が CLAUDE.md / AGENTS.md の `docs/` 構造節 (存在すれば) と矛盾なし |
| MV-30 | 直接実行 | `grep -rl "cli_test.rs\|config_test.rs\|feature_dev_refresh.rs" docs/ | sort > after.txt && diff before.txt after.txt` 結果が改名等を含まない (rename 0 件) |

### Mapping 網羅性検証

- MV-01 〜 MV-30 の 30 件全てが少なくとも 1 つの verification method に紐付いている (上表)
- 全 ISTQB 技法 (EP / BV / DT / ST) と NFR (01-05) が少なくとも 1 MV に使われている
- 手動検査項目は 30 件中 12 件 (MV-01, MV-03, MV-04, MV-05, MV-06, MV-13, MV-14, MV-15, MV-16, MV-17, MV-18, MV-25, MV-29) — review burden は ~ 40%、機械検証 60%
