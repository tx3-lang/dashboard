# Running the dashboard

This guide walks you through running the tx3 Dashboard end-to-end against the committed `buidler-fest/ticketing-2026` demo on Cardano preview.

## Prerequisites

You'll need:

- **Rust stable** (for building and running the tracker from `tx3-lang/tx3-lift`).
- **Node 24** (the version Nitro and TanStack Start are tested against in CI).
- **pnpm 10** — install with `npm install -g pnpm@10` or `corepack enable`.
- **A sibling clone of [`tx3-lang/tx3-lift`](https://github.com/tx3-lang/tx3-lift)** at `../tx3-lift` (relative to this repo). The dashboard does not embed the tracker; you run the tracker binary from that clone as a sidecar.
- **A `utxorpc` endpoint and API key.** The committed `tracker.toml` points at Demeter's Cardano preview endpoint; sign up at [demeter.run](https://demeter.run) for a free `dmtr_…` key.

## Environment variables

| Variable | Used by | Required | Default | Purpose |
|----------|---------|----------|---------|---------|
| `DMTR_API_KEY` | Tracker | Yes | — | Demeter API key for the configured `utxorpc` endpoint. The tracker fails to start without it. |
| `TRACKER_DB_PATH` | Dashboard | No | `./tracker.db` | Path the dashboard opens read-only. Leave unset to read the file the tracker writes alongside `tracker.toml`. |
| `RUST_LOG` | Tracker | No | (warn) | Log level for the tracker binary. `info` is comfortable for first-run; `debug` for protocol-level debugging. |

## First run

You'll need two terminals. From a fresh clone of this repo:

```bash
# Once: clone the tx3-lift sibling
git clone https://github.com/tx3-lang/tx3-lift ../tx3-lift
```

```bash
# Terminal A — tracker
cd ../tx3-lift
DMTR_API_KEY=dmtr_... RUST_LOG=info \
    cargo run -p tracker -- ../dashboard/tracker.toml
```

```bash
# Terminal B — dashboard
cd dashboard/frontend
pnpm install
pnpm dev
```

Visit <http://localhost:3000>. The tracker writes `dashboard/tracker.db` (plus its `-wal` / `-shm` companion files); the dashboard reads from the same file. New matches appear after a page reload.

## Production build

For an operator-managed deployment without `vite dev`:

```bash
cd frontend
pnpm install
pnpm build
node .output/server/index.mjs
```

Set `TRACKER_DB_PATH` if `tracker.db` lives outside the working directory. The Nitro bundle binds to `:3000` by default; override with `PORT`.

## Troubleshooting

### Empty list at `/`

The dashboard renders an empty-state message that names the tracker's current cursor slot. If you're seeing it indefinitely:

- Confirm the tracker terminal shows `Apply` events flowing in (or, on preview, that the protocol's policy filter actually matches recent on-chain activity).
- Confirm `tracker.db` exists in the dashboard working directory and is non-empty: `sqlite3 tracker.db 'SELECT COUNT(*) FROM matches;'`.
- Reload the page — the MVP does not auto-refresh.

### `SQLITE_CANTOPEN` on dashboard startup

The dashboard opens the DB with `fileMustExist: true`. If you see this error:

- The tracker hasn't created the file yet. Start the tracker first, wait for it to log its first cursor advance, then start `pnpm dev`.
- `TRACKER_DB_PATH` points at a non-existent path. Unset it and let the default (`./tracker.db` relative to the dashboard CWD) take over, or pass an absolute path.
- The file exists but the dashboard process doesn't have read permission. Check ownership and mode.

### Native build failure for `better-sqlite3`

`better-sqlite3` ships with a native binding. On `pnpm install` you may see "Ignored build scripts: better-sqlite3" because the package was added to `pnpm.onlyBuiltDependencies` after first install. Fix it with:

```bash
pnpm rebuild better-sqlite3
```

If you're on a system without a working `node-gyp` toolchain, install the platform's build tools (`xcode-select --install` on macOS; `build-essential` + `python3` on Debian/Ubuntu).

### Stale match after a chain rollback

If the upstream chain rolls back past a slot the tracker had recorded, the tracker emits an `Undo` event and removes the affected rows. Reload the dashboard page — the now-deleted match will return a 404 from `getMatch`. The list view at `/` automatically reflects the rollback on the next request.

## Tested with

- `tx3-lang/tx3-lift` tracker commit: `<TBD-fill-in-task-12>`
- Node 24, pnpm 10, Rust stable.

## Deferred deployment polish

These are out of scope for M3 and tracked for follow-up iterations:

- **Docker compose** stack that runs both the tracker and the dashboard with a shared volume for `tracker.db`.
- **Service-manager units** (systemd / launchd / pm2 templates) committed alongside the repo for one-shot operator install.
- **Postgres backend** for the tracker (1–3 days upstream PR) plus the dashboard-side dialect swap (~half a day) — see [`architecture.md` § Forward-looking](architecture.md#forward-looking-postgres).
