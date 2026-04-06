# Server Detection

Auto-detect and start a dev server for smoke testing.

## Override

If `args.server` and `args.port` are provided, skip detection and use them directly:

```
<args.server>   # e.g., "npm run dev"
# Then verify: curl -sf http://localhost:<args.port>/ > /dev/null
```

## Auto-Detection Table

Check in order. Use the first match.

| Check | Condition | Command | Default Port |
|-------|-----------|---------|-------------|
| package.json | `scripts.dev` exists | `npm run dev` | 3000 or 5173 (Vite) |
| package.json | `scripts.start` exists (no `dev`) | `npm start` | 3000 |
| Makefile | `dev` target exists | `make dev` | 8080 |
| Makefile | `serve` target exists (no `dev`) | `make serve` | 8080 |
| manage.py | file exists | `python manage.py runserver` | 8000 |
| docker-compose.yml | file exists | `docker compose up` | from `ports` mapping |

### How to check

- **package.json**: Read the file, check `scripts` object for key existence.
- **Makefile**: `grep -q '^dev:' Makefile` or `grep -q '^serve:' Makefile`
- **Vite detection**: If `vite` appears in `devDependencies`, default port is 5173 instead of 3000.

## Startup

1. Run detected command in background (`run_in_background: true`).
2. Poll `curl -sf http://localhost:<port>/` every 2 seconds.
3. Timeout after 30 seconds → PAUSE with message:
   "Server did not respond within 30 seconds. Check the startup command and port."

## No Detection

If no match found in the detection table:
1. Ask user: "No dev server detected. What command starts the server, and on what port?"
2. If no response → PAUSE.
