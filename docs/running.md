# Running the dashboard

This guide walks you through running the tx3 Dashboard end-to-end against the mainnet DeFi demo (Indigo, VyFi, Bodega, Fluid Aquarium, and Strike on Cardano mainnet).

## Prerequisites

You'll need:

- **Rust stable** (for building and running the tracker from `tx3-lang/tx3-lift`) — only required for the from-source path.
- **Node 24** (the version Nitro and TanStack Start are tested against in CI) — only required for the from-source path.
- **pnpm 10** — install with `npm install -g pnpm@10` or `corepack enable` — only required for the from-source path.
- **A sibling clone of [`tx3-lang/tx3-lift`](https://github.com/tx3-lang/tx3-lift)** at `../tx3-lift` (relative to this repo) — only required for the from-source path. The dashboard does not embed the tracker; you run the tracker binary from that clone as a sidecar.
- **A Demeter `utxorpc` API key.** Sign up at [demeter.run](https://demeter.run) for a free `dmtr_…` key. The recommended way to supply the key is the `DMTR_API_KEY` environment variable — you do not need to edit `tracker.toml` (the committed file keeps `api_key` commented out for this reason).

## Running with Docker

The simplest way to run the full system is with Docker Compose. You need:

- **Docker with Compose v2** (`docker compose` as a subcommand, not `docker-compose`).
- **A Demeter `utxorpc` API key** (see Prerequisites above).

```bash
# 1. Copy the example env file and fill in your key.
cp .env.example .env
#    Open .env and set DMTR_API_KEY=dmtr_...

# 2. Start the stack.
docker compose up
#    Add -d to detach: docker compose up -d

# 3. Open the dashboard.
#    http://localhost:3000
```

### How it works

The `tracker` service reads `deploy/tracker.toml` and the TII files from `protocols/` (both bind-mounted read-only into the container). It writes `tracker.db` into a named Docker volume (`tracker-data`). The `dashboard` service mounts the same named volume and reads the database. The dashboard waits for the tracker's healthcheck — which checks that `/data/tracker.db` exists — before starting, so you never see a `SQLITE_CANTOPEN` crash from a race at startup.

### Docker troubleshooting

- **Images not found** — the `tracker` and `dashboard` images are pulled from `ghcr.io/tx3-lang/tracker` and `ghcr.io/tx3-lang/dashboard`. Both must be published and publicly accessible on GHCR. If `docker compose pull` fails, check that the packages are public in the GitHub org.
- **Named volume on a network filesystem** — SQLite WAL mode (`-wal` / `-shm` sidecar files) is not safe on NFS or other network-backed filesystems. The `tracker-data` named volume must reside on a local filesystem. Docker Desktop on macOS and Linux with the default local volume driver both satisfy this requirement.
- **Empty list at `/`** — the tracker needs to scan the tip of mainnet and find a matching transaction before the dashboard has anything to show. Mainnet matches for the configured protocols typically appear within a few minutes. Check `docker compose logs tracker` to confirm blocks are flowing in.
- **Stopping and data lifecycle** — `docker compose down` stops the containers but keeps the `tracker-data` volume (the database is preserved). `docker compose down -v` removes the volume and wipes all stored matches.

## Environment variables

| Variable | Used by | Required | Default | Purpose |
|----------|---------|----------|---------|---------|
| `DMTR_API_KEY` | Tracker | Yes | — | Demeter API key for the configured `utxorpc` endpoint. Supply via `.env` for Docker, or export it in your shell for bare-metal. The tracker reads this variable directly when `api_key` is absent from `tracker.toml`; no wrapper script is needed. |
| `TRACKER_DB_PATH` | Dashboard | No | `./tracker.db` | Path the dashboard opens read-only. Leave unset to read the file the tracker writes alongside `tracker.toml`. In the Docker stack this is set to `/data/tracker.db` inside the container. |
| `RUST_LOG` | Tracker | No | (warn) | Log level for the tracker binary. `info` is comfortable for first-run; `debug` for protocol-level debugging. |
| `PORT` | Dashboard | No | `3000` | Host port the dashboard is published on. In Docker the container always listens on 3000; only the host-side binding uses this variable. |

## Running from source

If you prefer to build from source (or are developing the tracker or dashboard itself), you'll need two terminals. From a fresh clone of this repo:

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

> **API key**: supply `DMTR_API_KEY` as an environment variable (shown above). Do not add it to `tracker.toml` — the committed file keeps `api_key` commented out so the key is never accidentally committed.

## Production build

For an operator-managed deployment without `vite dev`. From the repo root:

```bash
cd dashboard/frontend
pnpm install
pnpm build
node .output/server/index.mjs
```

Set `TRACKER_DB_PATH` if `tracker.db` lives outside the working directory. The Nitro bundle binds to `:3000` by default; override with `PORT`.

## Troubleshooting

### Empty list at `/`

The dashboard renders "No matches yet — confirm the tracker is running." If you're seeing it indefinitely:

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

- `tx3-lang/tx3-lift` tracker commit: `04d0b90` (`docs: add integration test report (#8)`)
- Node 24, pnpm 10, Rust stable.

## Deferred deployment polish

These are out of scope for M3 and tracked for follow-up iterations:

- **Service-manager units** (systemd / launchd / pm2 templates) committed alongside the repo for one-shot operator install.
- **Postgres backend** for the tracker (1–3 days upstream PR) plus the dashboard-side dialect swap (~half a day) — see [`architecture.md` § Forward-looking](architecture.md#forward-looking-postgres).
