# belt-test-foundation (F1) — Test Strategy

F1 deliverable が正しく機能することを確認するための test strategy。ISTQB 4 技法 + ISO 25010 品質特性評価 + Must-Verify Checklist (MV-01 〜 MV-30) への mapping。`--e2e` off のため scenarios.yml (feature-internal) は生成しない。

## Test Design Techniques

### 1. Equivalence Partitioning

| ID | 適用対象 | Valid class | Invalid class |
|---|---|---|---|
| **EP-01** | `docs/testing/cli-behavior/*.yml` schema | 全必須 field (`id`, `category`, `severity`, `given`, `when`, `then`) 揃い、`severity` が enum (`critical|high|medium|low`) 内、`id` が kebab-case | required field 欠落、`severity` が enum 外 (`trivial` 等)、YAML syntax error、unknown top-level key |
| **EP-02** | `docs/testing/lock-ledger.md` entries の `locks-file:` | 指定パスのファイルが repo root 基準で実在 | 指定パス不在、相対パス基準混在 |
| **EP-03** | Rust doc-comment `/// scenario: <id>` | 正規表現 `^\s*///\s+scenario:\s+(\S+)\s*$` に match、**block comment `/* ... */` 事前 strip 後**、ID が scenarios.yml 内に存在 | (i) typo (`senario:` / `scenraio:`) / (ii) leading slash 欠 (`// scenario:`) / (iii) block comment 内 `/* /// scenario: X */` → strip で除去 / (iv) block doc-comment `/** scenario: X */` → strip で除去 / (v) inner doc-comment `//! scenario: X` → ignore / (vi) 文字列 literal `"/// scenario: X"` → strip で除去 / (vii) `#[cfg(test)]` / `#[cfg(feature = "x")]` gate 下 → grep 対象に含める (実行可否判定は手動 review) / (viii) scenarios.yml に ID 未登録 (orphan-rust) |
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
- **NFR-02 (Reliability)**: `for i in {1..100}; do cargo test -p belt-core --test view_test --quiet || exit 1; done; echo OK` で 100 連続 pass (fail 時 `|| exit 1` が即時 propagate、`|| break` の silent-pass bug を回避)
- **NFR-03 (Maintainability)**: Phase 3 spec-review の external reviewer が audit-template.md + lock-ledger.md のみを参照して 1 pilot file (推奨: `crates/belt/tests/cli_test.rs`) の audit を独立再現し、各 test の判定 label が F1 判定と 80% 以上一致。audit-template.md の v1 9 label 集合 (design.md P6 参照) は fixed enumeration、`audit_template_version` が scenarios_contract.rs で版整合 check される
- **NFR-04 (Functional suitability)**: scenarios_contract.rs が以下 7 種の drift injection で全て fail する (robustness probe):
  - scenarios.yml に ID 追加 / Rust 未追加 → fail (orphan-yml)
  - Rust doc-comment 追加 / scenarios.yml 未追加 → fail (orphan-rust)
  - lock-ledger `locks-file:` を不在 path に変更 → fail
  - `/* /// scenario: X */` (block comment 内) を埋めて orphan-yml が fail するか → strip 後 `scenario: X` が見えなくなれば正
  - `/** scenario: X */` (block doc-comment) を埋めて orphan-yml が fail するか → strip 後除去
  - `let s = "/// scenario: X";` (文字列 literal) を埋めて false positive しないか → strip 後除去
  - `//! scenario: X` (inner doc-comment) を埋めて `/// scenario:` とは区別されるか
- **NFR-05 (Portability)**: `dist-workspace.toml` の 4 Rust target triple — `x86_64-unknown-linux-gnu` (glibc >= 2.35) / `aarch64-unknown-linux-gnu` (glibc >= 2.35) / `x86_64-apple-darwin` (macOS 14+) / `aarch64-apple-darwin` (macOS 14+) — で build 可能。Empirical 検証は `release.yml` の cross build pass (既存 CI) + ローカル `cargo test --workspace` pass (開発機 = macOS 14+ arm64) を合算 signal として受容。**全 4 target での `cargo test` 実行は future work** (F1 では scope out、design.md Non-Goals に既述)

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
| MV-12 | EP-03 (context false case 網羅) | NFR-04 の 7 種 drift injection (orphan-yml / orphan-rust / locks-file 不在 / block comment / block doc-comment / 文字列 literal / inner doc-comment) 全てで fail 。 typo `senario:` / `scenario: X` (leading slash 欠) も false positive しない |
| MV-13 | EP-03 + 手動 | `cli_test.rs` の 5 test 全てに `/// scenario: belt-*` 付与 |
| MV-14 | EP-03 + 手動 | `config_test.rs` の 6 test 全てに `/// scenario: belt-core-config-*` 付与 |
| MV-15 | 手動 | audit-report.md に pilot 各 test の judgement + reason 記載 |
| MV-16 | 手動 | audit-report で使用した reason label が audit-template.md 列挙集合の subset |
| MV-17 | 手動 (git diff) | `feature_dev_refresh.rs` の diff ゼロ (lock-ledger.md entry 追加のみ) |
| MV-18 | 手動 (3 要件) | lock-ledger.md の `feature_dev_refresh.rs` entry に (A) 11 個 `#[test]` 関数名列挙 (B) それらが lock する pipeline.yml 側面 (args / narrative phases / max_retries / scenarios.when / regate 等) dimension name 列挙 (C) cross-coupling (shared_criteria_parity / shared_filter_parity / bug_fix_refresh / review_skills_refresh) 明記 |
| MV-19 | EP-04 + BV-04 + assertion-identity 追加 | `view_test.rs` の 6 箇所 `thread::sleep` が `filetime::set_file_mtime` 置換済 + strict ordering 箇所は **mtime delta 2 秒以上** (HFS+ 1s granularity 対応) |
| MV-20 | NFR-02 (`\|\| exit 1` 版) | 100 連続 `cargo test -p belt-core --test view_test` pass、fail 時即 exit 1 |
| MV-21 | 直接実行 | `cargo test --workspace` exit 0、全 test green |
| MV-22 | 直接実行 | `cargo clippy --workspace -- -D warnings` exit 0 |
| MV-23 | 直接実行 | `cargo fmt --all -- --check` exit 0 |
| MV-24 | DT-01 部分 | 5 既存 lock test が未改変 (`git diff` で確認) かつ `cargo test` で全 pass |
| MV-25 | 手動 | `.belt/runs/<run_id>/notes/phase-design.md` 存在、frontmatter (`phase: design` / `run_id: ...`) + 4 節 (`## Decisions` / `## Concerns` / `## Directives` / `## Observations`) |
| MV-26 | Phase 3/5/6 責務 | F1 design-gate 時点では未検証、参考列挙 (plan / execute / code-review phase で各 note 作成を確認) |
| MV-27 | 直接実行 | `git branch --list feature/2026-04-17-belt-test-foundation` exit 0 |
| MV-28 | 直接実行 (済) | `cargo test --workspace` = 387/387 pass (F1 進入前に確認済、本 phase-design commit 時点で再確認) |
| MV-29 | 手動 | `docs/testing/README.md` が CLAUDE.md / AGENTS.md の `docs/` 構造節 (存在すれば) と矛盾なし |
| MV-30 | 直接実行 + capture step | **capture**: Phase 5 execute 最初のタスクで `grep -rl "cli_test.rs\|config_test.rs\|feature_dev_refresh.rs" docs/ | sort > .belt/runs/{run_id}/artifacts/mv30-before.txt`。**verify**: Phase 6 code-review で `grep -rl ... > after.txt && diff .belt/runs/{run_id}/artifacts/mv30-before.txt after.txt` が empty (rename 0 件) |
| MV-31 (CCS-05) | EP-04 + 手動 (assertion tokenization diff) | `view_test.rs` の `assert_eq!` / `assert!` / `assert_ne!` 行が置換前後で diff ゼロ (filetime call 追加 / sleep 削除のみが差分、assertion ロジック identity 保持) |
| MV-32 (CCS-06) | 手動 + scenarios_contract.rs | audit-report.md frontmatter に `audited_at` / `audited_commit` / `audit_template_version` 記載、scenarios_contract.rs で frontmatter 存在 + `audit_template_version` が audit-template.md 宣言と一致 |
| MV-33 (CCS-06) | 手動 | audit-template.md に F2 着手時の re-audit trigger 手順 (`git log --since="<audited_at>" -- <pilot_file>` が非空なら pilot audit 再実施) 記載 |

### Mapping 網羅性検証

- MV-01 〜 MV-33 の **33 件**全てが少なくとも 1 つの verification method に紐付いている (上表)
- 全 ISTQB 技法 (EP / BV / DT / ST) と NFR (01-05) が少なくとも 1 MV に使われている
- 手動検査項目は 33 件中 14 件 (MV-01, MV-03, MV-04, MV-05, MV-06, MV-13, MV-14, MV-15, MV-16, MV-17, MV-18, MV-25, MV-29, MV-31, MV-33 部分) — review burden は ~ 42%、機械検証 58%
