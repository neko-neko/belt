# BELT-31: Gate cmd execution timeout

**Linear**: [BELT-31](https://linear.app/neko-neko/issue/BELT-31)
**Parent**: [BELT-20](https://linear.app/neko-neko/issue/BELT-20)
**Date**: 2026-04-07

## Summary

Add a `timeout` field to `GateCheck::Cmd` that kills the child process after a deadline. Default 1800 seconds (30 minutes). `timeout: 0` opts out (no timeout). Implementation uses `try_wait()` polling — no async runtime, no unsafe, no new crates.

## Background

`gate.rs` の `execute_cmd()` は `Command::new("sh").output()` を使用しており、タイムアウトがない。ハングしたコマンド（無限ループするテスト、応答しない外部サービス等）が belt-agent を無期限にブロックし、LLM の API コストが暴走する。GitHub Actions がジョブにデフォルトタイムアウトを設けるのと同じ理由で、belt も安全側に倒す。

## Design

### YAML

```yaml
gate:
  - cmd: "cargo test"              # timeout: 1800 (default, 30 min)
  - cmd: "make lint"
    timeout: 60                    # 60 seconds
  - cmd: "long-running-build"
    timeout: 0                     # no timeout (opt-out)
```

### model.rs — GateCheck::Cmd

```rust
Cmd {
    cmd: String,
    #[serde(default = "default_gate_timeout")]
    timeout: u64,
},
```

```rust
fn default_gate_timeout() -> u64 {
    1800
}
```

- `timeout` の単位は秒
- `0` は無制限（タイムアウトなし）
- デフォルト 1800 秒（30 分）
- untagged enum 互換: `cmd` が必須 discriminant。`timeout` は optional で追加しても既存 YAML のデシリアライズ順序を壊さない

### gate.rs — execute_cmd

現在の `Command::output()` を `Command::spawn()` + ポーリングに置換。

```
execute_cmd(cmd, work_dir) → execute_cmd(cmd, work_dir, timeout)
```

ただし `execute_gate()` の pub API は変更不要。`GateCheck::Cmd { cmd, timeout }` から timeout を取得してそのまま渡す。

#### timeout == 0 の場合

現行の `Command::output()` をそのまま使用。挙動変更なし。

#### timeout > 0 の場合

1. `Command::spawn()` で子プロセス起動（`stdout: Stdio::piped()`, `stderr: Stdio::piped()`）
2. `child.stdout.take()` と `child.stderr.take()` を別スレッドで読み取り（パイプバッファ deadlock 防止）
3. `child.try_wait()` を 100ms 間隔でポーリング
4. deadline (`Instant::now() + Duration::from_secs(timeout)`) 超過 → `child.kill()` + `child.wait()`（ゾンビ回収）
5. タイムアウト時は `GateResult { passed: false, timed_out: true, detail: "timed out after {N}s" }`
6. 正常終了時は現行と同じ結果を返す

#### パイプ読み取りスレッド

```rust
let stdout_reader = child.stdout.take().unwrap();
let stderr_reader = child.stderr.take().unwrap();

let stdout_handle = std::thread::spawn(move || {
    let mut buf = Vec::new();
    std::io::Read::read_to_end(&mut stdout_reader, &mut buf).ok();
    buf
});
let stderr_handle = std::thread::spawn(move || {
    let mut buf = Vec::new();
    std::io::Read::read_to_end(&mut stderr_reader, &mut buf).ok();
    buf
});
```

子プロセス終了後（正常 or kill）にスレッドを `join()` して出力を回収。

### gate.rs — GateResult

```rust
#[derive(Debug, Clone, Serialize)]
pub struct GateResult {
    pub check_type: String,
    pub passed: bool,
    pub detail: Option<String>,
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub timed_out: bool,
}
```

- `timed_out: bool` を追加（デフォルト `false`）
- タイムアウト時のみ `true`
- 既存の JSON 出力に `"timed_out": false` が追加される（後方互換）
- LLM がタイムアウトと通常の FAIL を区別可能

### 影響範囲

| ファイル | 変更 |
|---|---|
| `crates/belt-core/src/model.rs` | `GateCheck::Cmd` に `timeout: u64` + `default_gate_timeout()` |
| `crates/belt-core/src/gate.rs` | `execute_cmd` にタイムアウトロジック、`GateResult` に `timed_out` |
| `crates/belt-core/tests/gate_test.rs` | 新規テスト |
| `crates/belt-core/tests/model_test.rs` | デシリアライズテスト |
| `crates/belt-agent/tests/cli_test.rs` | CLI integration テスト |

### 変更しないもの

| 項目 | 理由 |
|------|------|
| `execute_gate()` / `execute_gates()` の pub API シグネチャ | timeout は GateCheck 内から取得 |
| `git_clean` | スコープ外（cmd のみ） |
| belt.toml | グローバルデフォルトは YAGNI |
| async runtime | Non-Goals |
| engine.rs / view.rs / lint.rs / main.rs | 影響なし |

## Test Plan

### A. Model: デシリアライズ (model_test.rs, 4 tests)

| # | Test | YAML Input | Expected |
|---|------|-----------|----------|
| 1 | `cmd_default_timeout` | `cmd: "cargo test"` | `Cmd { cmd: "cargo test", timeout: 1800 }` |
| 2 | `cmd_explicit_timeout` | `{ cmd: "make lint", timeout: 60 }` | `Cmd { cmd: "make lint", timeout: 60 }` |
| 3 | `cmd_timeout_zero` | `{ cmd: "sleep 999", timeout: 0 }` | `Cmd { cmd: "sleep 999", timeout: 0 }` |
| 4 | `cmd_timeout_does_not_affect_other_variants` | `file_exists: "*.md"` の後に `{ cmd: "test", timeout: 10 }` | 両方正しくデシリアライズ。FileExists に timeout フィールドが混入しない |

### B. Gate: 正常完了 (gate_test.rs, 4 tests)

| # | Test | Setup | Expected |
|---|------|-------|----------|
| 5 | `cmd_with_timeout_passes` | `cmd: "true"`, timeout: 5 | `passed: true, timed_out: false, duration_ms: Some(< 5000)` |
| 6 | `cmd_with_timeout_fails_normally` | `cmd: "false"`, timeout: 5 | `passed: false, timed_out: false, detail: "exit 1: ..."` |
| 7 | `cmd_with_timeout_zero_passes` | `cmd: "true"`, timeout: 0 | `passed: true, timed_out: false`（無制限パス） |
| 8 | `cmd_with_default_timeout_passes` | `cmd: "true"`, timeout: 1800 (default) | `passed: true, timed_out: false` |

### C. Gate: タイムアウト (gate_test.rs, 4 tests)

| # | Test | Setup | Expected |
|---|------|-------|----------|
| 9 | `cmd_timeout_kills_hanging_process` | `cmd: "sleep 60"`, timeout: 1 | `passed: false, timed_out: true, detail: "timed out after 1s"` |
| 10 | `cmd_timeout_duration_reflects_timeout` | `cmd: "sleep 60"`, timeout: 2 | `duration_ms >= 2000 && duration_ms < 3000` |
| 11 | `cmd_timeout_stderr_not_captured_on_kill` | `cmd: "sleep 60"`, timeout: 1 | `detail` にタイムアウトメッセージが含まれ、stderr は無関係 |
| 12 | `cmd_fast_finish_before_timeout` | `cmd: "echo fast"`, timeout: 1 | `passed: true, timed_out: false, duration_ms < 1000` |

### D. Gate: エラー系 (gate_test.rs, 3 tests)

| # | Test | Setup | Expected |
|---|------|-------|----------|
| 13 | `cmd_spawn_failure_with_timeout` | `cmd` に存在しないコマンド (e.g., `/nonexistent`), timeout: 5 | `passed: false, timed_out: false, detail: "failed to spawn: ..."` |
| 14 | `cmd_stderr_output_on_failure_with_timeout` | `cmd: "echo err >&2 && false"`, timeout: 5 | `passed: false, timed_out: false, detail` に "err" を含む |
| 15 | `cmd_signal_exit_with_timeout` | `cmd: "kill -9 $$"`, timeout: 5 | `passed: false, timed_out: false, detail: "exit signal: ..."` |

### E. Gate: timed_out フィールド検証 (gate_test.rs, 2 tests)

| # | Test | Setup | Expected |
|---|------|-------|----------|
| 16 | `gate_result_timed_out_default_false` | file_exists gate 実行 | `timed_out: false` |
| 17 | `gate_result_timed_out_serializes` | timed_out: true の GateResult を JSON 化 | `"timed_out": true` が出力に含まれる |

### F. Gate: execute_gates 統合 (gate_test.rs, 2 tests)

| # | Test | Setup | Expected |
|---|------|-------|----------|
| 18 | `execute_gates_one_timeout_fails_all` | 2 checks: `true` (timeout 5) + `sleep 60` (timeout 1) | `all_passed: false`, 2 番目のみ `timed_out: true` |
| 19 | `execute_gates_all_pass_with_timeout` | 2 checks: `true` (timeout 5) + `echo ok` (timeout 5) | `all_passed: true`, 両方 `timed_out: false` |

### G. CLI 統合 (cli_test.rs, 2 tests)

| # | Test | Setup | Expected JSON |
|---|------|-------|---------------|
| 20 | `verify_outputs_timed_out_field` | pipeline with `cmd: "true"`, timeout: 5 → init → verify | `checks[0].timed_out == false` が JSON に含まれる |
| 21 | `verify_timeout_returns_fail` | pipeline with `cmd: "sleep 60"`, timeout: 1 → init → verify | `verdict: "FAIL"`, `checks[0].timed_out == true` |

### テスト実装上の注意

- タイムアウトテスト（#9, #10, #11, #18, #21）は `sleep` コマンドを使用。実行に 1-2 秒かかるが、CI でも安定
- `#[ignore]` は付けない。timeout: 1 のテストなら 2 秒以内に完了
- `#15` の `kill -9 $$` は sh 自体を kill するので signal exit を検証可能
- `cmd: "/nonexistent"` はスポーン失敗のテスト。`sh -c "/nonexistent"` は sh が起動するため、代わりに spawn 自体が失敗するケースを検討。実際には `sh -c` 経由なので sh の exit code 127 で検出する

## Known Limitations

- **ポーリング遅延**: 最大 100ms の遅延。gate check では実用上問題なし
- **プロセスグループ未対応**: `child.kill()` は直接の子プロセスのみ kill。子プロセスが孫プロセスを spawn した場合、孫は残る可能性がある。将来的に `setsid` + プロセスグループ kill が必要だが、`unsafe_code = "forbid"` 制約下では対応不可。実用上、`sh -c` の直接の子が kill されれば shell も終了するため問題になることは稀
- **git_clean 未対応**: スコープ外。git は通常高速だが、NFS 等で遅延する可能性はある。必要に応じて別チケットで対応
