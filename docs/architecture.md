# Architecture

The tx3 Dashboard is a pure TypeScript application that observes the output of an external `tx3-lift` tracker. The tracker subscribes to a Cardano `utxorpc` stream, matches incoming transactions against a TII, and writes one row per match into a local SQLite file. The dashboard reads that same file via Kysely + better-sqlite3 from inside Nitro server functions and renders matches in the browser.

## System context (C4 L1)

```mermaid
C4Context
title System Context — tx3 Dashboard

Person(operator, "Builder / Operator", "Runs the dashboard for their own dApp")
System(dashboard, "tx3 Dashboard", "Monitors on-chain activity for a Tx3-described dApp")
System_Ext(utxorpc, "utxorpc Provider", "Cardano chain stream (v1beta WatchTx)")
System_Ext(registry, "Tx3 Registry", "Source of TII files (queried at vendor time, not runtime)")

Rel(operator, dashboard, "Views matched transactions", "HTTPS")
Rel(dashboard, utxorpc, "Subscribes to tx stream", "gRPC / TLS")
Rel(operator, registry, "Pulls TII once at vendor time", "GraphQL")
```

## Containers (C4 L2)

```mermaid
C4Container
title Container Diagram — tx3 Dashboard

Person(operator, "Builder / Operator")
System_Ext(utxorpc, "utxorpc Provider", "v1beta WatchTx")

Container_Boundary(c1, "tx3 Dashboard") {
    Container(tracker, "Tracker", "Rust binary — tx3-lift", "Subscribes, matches against TII, lifts, persists each match")
    ContainerDb(db, "tracker.db", "SQLite + WAL", "matches and cursor tables; tracker writes, dashboard reads")
    Container(ssr, "Dashboard SSR", "TanStack Start (Node / Nitro)", "Server-rendered React; queries the DB via Kysely")
    Container(browser, "Browser UI", "React + Tailwind + shadcn", "Renders matches list and detail")
}

Rel(operator, browser, "Uses", "HTTPS")
Rel(browser, ssr, "Loads pages", "HTTPS")
Rel(utxorpc, tracker, "Streams Apply / Undo / Idle", "gRPC")
Rel(tracker, db, "INSERT match, advance cursor")
Rel(ssr, db, "SELECT matches", "Kysely + better-sqlite3")

UpdateLayoutConfig($c4ShapeInRow="3", $c4BoundaryInRow="1")
```

The tracker and the dashboard are **decoupled at the SQLite file**. WAL mode (enabled by the tracker on first connect) lets the dashboard's reader run concurrently with the tracker's writer without blocking.

## Data flow per page load

```mermaid
sequenceDiagram
    participant B as Browser
    participant N as Nitro server fn
    participant K as Kysely
    participant S as SQLite (tracker.db)

    B->>N: GET / (or /txs/<hash>)
    N->>K: listMatches({ limit }) / getMatch(txHash)
    K->>S: SELECT ... FROM matches ...
    S-->>K: rows (tx_hash blob, lifted JSON, ...)
    K-->>N: typed rows
    N-->>B: SSR HTML with matches
```

The same Kysely instance is reused across requests in a process; better-sqlite3 opens the file in read-only mode and the dashboard never writes.

## Component responsibilities

- **Tracker** (Rust binary from [`tx3-lang/tx3-lift`](https://github.com/tx3-lang/tx3-lift), run as a sidecar): subscribes to the configured `utxorpc` stream, applies the `Fingerprint → Match → Lift` pipeline against the TII, and inserts one row into `matches` for each match. Owns the schema and the cursor.
- **Dashboard SSR** (this repo, `frontend/`): a TanStack Start app that opens `tracker.db` read-only inside Nitro server functions. `lib/db.ts` builds a Kysely instance, `lib/queries.ts` exposes `listMatches` and `getMatch`, `lib/lifted.ts` parses the small subset of the `lifted` JSON the MVP needs (parties + addresses).
- **SQLite (`tracker.db`)**: the integration boundary. WAL mode allows the writer (tracker) and the reader (dashboard) to run concurrently. The dashboard does not modify the schema, does not run migrations, and does not write rows.

## Why this shape

- **Single-purpose components.** The tracker is already battle-tested in `tx3-lift`. Embedding it as a library would re-litigate that. Running it as a sidecar lets it evolve in its own repo without ABI coupling.
- **Honest authority.** The tracker owns the schema; the dashboard observes. There is no risk of two implementations of the same schema drifting.
- **TanStack SSR is enough.** Nitro server functions can open SQLite directly. There is no use case in the MVP that needs a separate API tier, so we don't add one.
- **Trivial deployment.** Two processes — one is `cargo run`, the other is `pnpm build && node`. Operators can use tmux, systemd, pm2, or whatever else they prefer.

## Forward-looking: Postgres

The schema and access patterns are Postgres-portable. A future migration is split between the upstream tracker (1–3 days of upstream PR work to add a Postgres storage backend or replication path) and this repo (~half a day to swap `kysely`'s `SqliteDialect` for `PostgresDialect`, swap `better-sqlite3` for `pg`, and adjust the JSON path syntax in any `sql<T>` template literals — a single helper module).

See [§4.3 of the M3 design spec](superpowers/specs/2026-05-07-m3-dashboard-design.md#43-postgres-migration-plan-forward-looking-not-implemented) for the migration plan.
