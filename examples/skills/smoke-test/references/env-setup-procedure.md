# env-setup Procedure

Start the dev server and verify it is accessible.

## Procedure

1. If `args.server` and `args.port` are set, use them directly (skip auto-detection).
2. Otherwise, read [server-detection.md](server-detection.md) and auto-detect
   the server command and port.
3. Start the server in the background.
4. Wait for the server to respond (timeout: 30 seconds).
5. If timeout → report PAUSE status.
