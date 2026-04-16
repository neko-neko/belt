---
name: spec-review-done-criteria
audit: lite
phase: spec-review
---

# Phase 3 (spec-review) Done Criteria

- **SREV-01**: `docs/features/<topic>/test-strategy.md` の必須セクション
  (`Test Design Techniques` / `Quality Characteristics` / `Priority Matrix`)
  が spec-review 後も保持されている (TEST-02 と同等の構造)。
- **SREV-02**: spec-review findings の triage が完了している
  (grill-me group と selection group の両方が処理済み、未処理 finding が残っていない)。
- **SREV-03**: user 承認済み findings のみが `test-strategy.md` / `scenarios.yml` に反映されている
  (grill-me: `accept` または `accept_current`、selection: user が番号指定したもののみ)。
- **SREV-04**: 未承認 findings (grill-me `reject` および selection で選択されなかったもの) の
  差分が成果物ファイルに含まれていない。
- **SREV-05**: `args.e2e` が true の場合、`docs/features/<topic>/scenarios.yml` も
  spec-review のレビュー対象に含まれている (findings 内で scenarios が参照されている)。
- **SREV-06**: `test-strategy.md` または `scenarios.yml` を書き換えた場合、
  対応するコミットが作成されている (unstaged 変更が残っていない)。
