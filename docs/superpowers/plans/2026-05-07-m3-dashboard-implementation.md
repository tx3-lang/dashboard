# M3 Dashboard Implementation Plan (v2)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to execute this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. **Style:** declarative — each task specifies the public contract, the tests that must pass, and acceptance criteria; the implementer chooses the implementation. Code is shown literally only where it IS the artifact (SQL schema, TOML config, shell commands).

**Goal:** Replace the M1 backend scaffold with a TanStack Start frontend that reads SQLite produced by the externally-run `tx3-lift` tracker. Ship two views (matches list + match detail) plus the documentation needed for an operator to run the system end-to-end.

**Architecture:** Two-process topology — the tracker runs as a sidecar (built from `tx3-lang/tx3-lift`) and writes `tracker.db` (SQLite, WAL). The dashboard is a TanStack Start app whose Nitro server functions read that SQLite with Kysely + better-sqlite3.

**Tech stack:** TypeScript, TanStack Start (Nitro SSR), TanStack Router, Tailwind 4, shadcn, Kysely, better-sqlite3, vitest, Biome, pnpm.

---

## Spec reference

Implements `docs/superpowers/specs/2026-05-07-m3-dashboard-design.md`. Two access patterns (AP-1 list, AP-2 detail), no Rust on the dashboard side, MVP scope.

## Pre-conditions

- Node 24 + pnpm 10 (matches frontend CI).
- `tx3-lang/tx3-lift` cloned at `../tx3-lift/` relative to this repo (used to run the tracker; not a build dependency of the dashboard).
- Rust stable (only needed to run the tracker).

---

## File structure

### Created

```
protocols/buidler-fest/ticketing-2026.tii   committed fixture
tracker.toml                                 tracker sidecar config
docs/architecture.md
docs/access-patterns.md
docs/running.md
frontend/src/lib/db.ts
frontend/src/lib/lifted.ts
frontend/src/lib/queries.ts
frontend/src/lib/__tests__/db.test.ts
frontend/src/lib/__tests__/lifted.test.ts
frontend/src/lib/__tests__/queries.test.ts
frontend/src/components/PartyChip.tsx
frontend/src/components/TxNamePill.tsx
```

### Modified

- `README.md` — refreshed with C4 Context diagram + quick start.
- `.gitignore` — `tracker.db*` artifacts.
- `frontend/package.json` — adds Kysely + better-sqlite3.
- `frontend/src/components/Header.tsx` — slimmed.
- `frontend/src/routes/index.tsx` — rewritten as matches list.
- `frontend/src/routes/txs/$hash.tsx` — rewritten as match detail.
- `frontend/README.md` — slimmed.

### Deleted

- `backend/` (entire directory).
- `.github/workflows/backend-ci.yml`.
- `frontend/src/routes/txs/index.tsx`.

---

## Tasks

### Task 1: Remove the M1 backend scaffold

**Goal:** delete the obsolete Rust backend and its CI; keep tracker artifacts out of git.

**Files**
- Delete: `backend/`, `.github/workflows/backend-ci.yml`
- Modify: `.gitignore` — append `tracker.db`, `tracker.db-wal`, `tracker.db-shm`

**Acceptance**
- `git ls-files backend/` and `git ls-files .github/workflows/backend-ci.yml` are empty.
- `(cd frontend && pnpm typecheck)` succeeds.
- A grep for `backend` in the surviving files turns up only documentation references that you've cleaned up (or none).

**Commit:** `chore: remove M1 backend scaffold and its CI; v2 reads tracker SQLite directly`

---

### Task 2: Vendor the demo TII and ship `tracker.toml`

**Goal:** commit the buidler-fest TII fixture and the tracker config so an operator runs end-to-end with one extra clone.

**Files**
- Create: `protocols/buidler-fest/ticketing-2026.tii`
- Create: `tracker.toml` (repo root)

**TII fetch** (one-time vendor step):

```bash
mkdir -p protocols/buidler-fest
curl -s -X POST https://api.tx3.land/graphql \
     -H 'content-type: application/json' \
     -d '{"query":"{ protocol(scope:\"buidler-fest\", name:\"ticketing-2026\") { tii } }"}' \
  | jq -r '.data.protocol.tii' \
  > protocols/buidler-fest/ticketing-2026.tii
```

If the registry returns the `tii` field as an inline JSON object (not a string), drop the `-r` and use `jq '.data.protocol.tii'`. Confirm:

```bash
jq '.protocol.name, (.transactions | keys), (.profiles | keys)' protocols/buidler-fest/ticketing-2026.tii
```
must print `"ticketing-2026"`, `["buy_ticket"]`, and a profile list including `"preview"`.

**`tracker.toml` content** (literal):

```toml
[upstream]
endpoint  = "https://preview.utxorpc-v0.demeter.run"
intersect = "tip"
# api_key set via DMTR_API_KEY env

[upstream.filter]
mints_policy_id = "1d9c0b541adc300c19ddc6b9fb63c0bfe32b1508305ba65b8762dc7b"

[storage]
database_path = "./tracker.db"

[[sources]]
name      = "ticketing-2026-preview"
tii_path  = "./protocols/buidler-fest/ticketing-2026.tii"
profile   = "preview"
```

**Acceptance**
- `cd ../tx3-lift && cargo run -p tracker -- ../dashboard/tracker.toml` reaches the log line `subscribing to WatchTx endpoint=…` and creates `tracker.db` inside the dashboard repo. (Stop after one minute; full smoke is Task 12.)

**Commit:** `feat: vendor ticketing-2026 TII and tracker.toml for the M3 demo`

---

### Task 3: Frontend dependencies

**Goal:** add Kysely + better-sqlite3 with the native build allowance pnpm requires.

**Files**: `frontend/package.json`, `frontend/pnpm-lock.yaml`.

**Steps**

```bash
cd frontend
pnpm add kysely better-sqlite3
pnpm add -D @types/better-sqlite3
```

In `frontend/package.json`, append `"better-sqlite3"` to the existing `pnpm.onlyBuiltDependencies` array.

**Acceptance**
- `pnpm install` finishes without `package … is not allowed to run scripts` warnings.
- `pnpm typecheck` succeeds.

**Commit:** `feat(frontend): add kysely + better-sqlite3 for SSR data access`

---

### Task 4: Database connection module

**Goal:** typed read-only Kysely instance over `tracker.db`. Abstracts the file-vs-existing-instance choice for tests.

**Files**: `frontend/src/lib/db.ts`, `frontend/src/lib/__tests__/db.test.ts`.

**Public contract**

```ts
interface MatchesRow {
  id: number;
  tx_hash: Buffer;
  block_slot: number;
  block_hash: Buffer;
  source_name: string;
  protocol_name: string;
  tx_name: string;
  profile_name: string;
  lifted: string;
  matched_at: number;
}
interface CursorRow { id: number; slot: number; block_hash: Buffer }
interface SchemaVersionRow { name: string; applied_at: number }
interface DashboardDatabase {
  matches: MatchesRow;
  cursor: CursorRow;
  _schema_versions: SchemaVersionRow;
}

createDb(opts?: { path?: string; existing?: BetterSqlite3.Database }): Kysely<DashboardDatabase>
```

**Behavior**
- `existing` provided → wrap it; do not open or pragma. Used by tests with `:memory:`.
- `path` provided → open `readonly: true`, `fileMustExist: true`. Set `journal_mode = WAL`.
- Neither → resolve path from `process.env.TRACKER_DB_PATH ?? './tracker.db'`, then open as above.

**Tests** (`db.test.ts`)
- Build a `:memory:` connection seeded with the matches/cursor schema; pass it via `existing`; run a typed `selectFrom('matches').select('id').execute()`; expect `[]`.

**Acceptance**: test passes; `pnpm typecheck` clean.

**Commit:** `feat(frontend): Kysely-backed read-only connection to tracker.db`

---

### Task 5: Lifted JSON types and helpers

**Goal:** type the subset of `lifted` JSON the MVP renders and re-encode the byte-array fields the lifter writes (e.g. `address: [0x61, ...]`) into hex strings.

**Files**: `frontend/src/lib/lifted.ts`, `frontend/src/lib/__tests__/lifted.test.ts`.

**Public contract**

```ts
interface LiftedParty { address: string; role: string }
interface Lifted { txName: string; parties: Record<string, LiftedParty>; raw: string }

bytesToHex(bytes: number[]): string
truncateHex(hex: string, edge?: number): string   // default edge = 6
parseLifted(json: string): Lifted
```

**Behavior**
- `bytesToHex`: lowercase hex without `0x`. `[]` → `''`. Throws on non-integers or values outside `[0, 255]`. Error message contains the word "byte".
- `truncateHex(hex, edge)`: returns `hex` unchanged when `hex.length <= edge * 2`; otherwise `${hex.slice(0, edge)}…${hex.slice(-edge)}`.
- `parseLifted`:
  - `JSON.parse` first (re-throws on invalid JSON).
  - Reads `tx_name` (defaults to `''`).
  - Iterates `parties` if present, skipping entries without an array `address`. Re-encodes `address` via `bytesToHex`. `role` is read as string (default `''`).
  - Returns `{ txName, parties, raw: <input json> }`.

**Tests** (`lifted.test.ts`)
- `bytesToHex([0xab, 0x01, 0xff])` → `'ab01ff'`.
- `bytesToHex([])` → `''`.
- `bytesToHex([300])` throws (`/byte/i`).
- `truncateHex('0123456789abcdef0123456789abcdef', 6)` → `'012345…abcdef'`.
- `truncateHex('abcd', 6)` → `'abcd'`.
- `parseLifted` with `tx_name: 'buy_ticket'` and parties `{ buyer: { address: [0x61, 0x12, 0x34], role: 'Input' }, treasury: { address: [0x61, 0xab], role: 'Output' } }` → `{ txName: 'buy_ticket', parties: { buyer: { address: '611234', role: 'Input' }, treasury: { address: '61ab', role: 'Output' } } }`.
- `parseLifted` without parties → `parties === {}`.
- `parseLifted('not json')` throws.

**Acceptance**: 8 tests pass.

**Commit:** `feat(frontend): lifted JSON parser + bytes-to-hex helpers`

---

### Task 6: Queries — listMatches and getMatch

**Goal:** implement AP-1 and AP-2 as typed Kysely queries returning a consumer-friendly shape.

**Files**: `frontend/src/lib/queries.ts`, `frontend/src/lib/__tests__/queries.test.ts`.

**Public contract**

```ts
interface MatchRow {
  id: number;
  hash: string;            // hex
  txName: string;
  protocolName: string;
  profileName: string;
  blockSlot: number;
  matchedAt: Date;
  parties: Record<string, LiftedParty>;
  rawLifted: string;
}

listMatches(db: Kysely<DashboardDatabase>, limit?: number): Promise<MatchRow[]>
getMatch(db: Kysely<DashboardDatabase>, txHashHex: string): Promise<MatchRow | null>
```

**Behavior**
- `listMatches`: `SELECT … FROM matches ORDER BY id DESC LIMIT ?`. Default `limit = 50`, clamped to `[1, 200]`.
- `getMatch`: `SELECT … FROM matches WHERE tx_hash = ? LIMIT 1`. Hex input must match `/^[0-9a-fA-F]+$/` and have even length — throw otherwise. Decode to `Buffer` for the bind.
- Both use `parseLifted` (Task 5) to populate `parties` and `rawLifted`.
- Both convert `matched_at` (unix seconds) to `Date`.

**Tests** (`queries.test.ts`)
- Seed an in-memory SQLite with the schema from Task 4. Insert two rows: tx_hash `[0x01, 0x01]` (slot 100, matched_at 1700000000, lifted with parties `buyer→[0x61,0x01]`, `treasury→[0x61,0x02]`) and tx_hash `[0x02, 0x02]` (slot 110, matched_at 1700000050, parties `buyer→[0x61,0x02]`, `treasury→[0x61,0x03]`).
- `listMatches(db, 10)` → 2 rows newest-first; first row's `hash === '0202'`, `txName === 'buy_ticket'`, `parties.buyer.address === '6102'`, `matchedAt` equals `new Date(1700000050 * 1000)`.
- `listMatches(db, 1)` → 1 row.
- `getMatch(db, '0101')` → match where `parties.treasury.address === '6102'`.
- `getMatch(db, 'deadbeef')` → `null`.

**Acceptance**: 4 tests pass.

**Commit:** `feat(frontend): listMatches + getMatch queries`

---

### Task 7: Reusable display components

**Goal:** two small chip components used by both pages.

**Files**: `frontend/src/components/PartyChip.tsx`, `frontend/src/components/TxNamePill.tsx`.

**Public contract**

```tsx
TxNamePill({ name: string })
PartyChip({ name: string; address: string; role?: string })
```

**Visual contract**
- `TxNamePill`: rounded pill, `bg-primary/10 text-primary`, small text size (xs), font-semibold.
- `PartyChip`: rounded pill with a 1.5×1.5 primary-color dot leading the row, party name (font-semibold), then address rendered through `truncateHex` in `font-mono text-muted-foreground`, then optional role suffix in tiny uppercase muted text.

Use the existing palette tokens (`primary`, `muted-foreground`, `border`, `background`) — do not introduce new colors.

**Acceptance**: `pnpm typecheck` clean. Visual eyeball happens via Tasks 8–9.

**Commit:** `feat(frontend): PartyChip + TxNamePill display components`

---

### Task 8: Matches list view at `/`

**Goal:** replace the M1 placeholder with the AP-1 list.

**Files**: `frontend/src/routes/index.tsx` (replace).

**Behavior**
- Server function (`createServerFn({ method: 'GET' }).handler(...)`) opens a fresh `createDb()` per call, runs `listMatches(db, 50)`, calls `db.destroy()` in a `finally`. Returns `MatchRow[]` (must be JSON-serializable — `Date` is fine via TanStack Start's serialization).
- `Route.loader` invokes the server fn; component reads via `Route.useLoaderData()`.
- Render header: `<h1>Matches</h1>` + caption "N most recent" (right-aligned, `text-sm text-muted-foreground`).
- When `matches.length === 0`: empty-state card with the message *"No matches yet — confirm the tracker is running. See `docs/running.md`."* in a dashed-border box.
- Otherwise: a table with columns Tx (TxNamePill) · Hash (clickable `Link to /txs/$hash` with truncated hex) · Slot (`toLocaleString()`, monospace muted) · Parties (row of `PartyChip`s) · When (`matchedAt.toISOString().replace('T', ' ').slice(0, 19)`).

**Acceptance**
- `pnpm dev` boots; `/` renders.
- Empty `tracker.db` → empty state.
- Populated `tracker.db` → rows display; clicking a hash navigates to `/txs/$hash`.

**Commit:** `feat(frontend): matches list view at /`

---

### Task 9: Match detail view at `/txs/$hash`

**Goal:** replace the M1 placeholder with the AP-2 detail; remove the obsolete `/txs` index.

**Files**
- Modify: `frontend/src/routes/txs/$hash.tsx`
- Delete: `frontend/src/routes/txs/index.tsx`

**Behavior**
- Server function declared with `.validator((hash: string) => hash)` and `.handler(async ({ data: hash }) => {...})`. Body opens `createDb()`, runs `getMatch(db, hash)`, destroys in `finally`. Returns `MatchRow | null`.
- `Route.loader` calls the server fn passing `params.hash`; if result is `null`, throw TanStack Router's `notFound()`.
- Component renders:
  - Header: TxNamePill + caption `${protocol_name} · ${profile_name} · slot ${block_slot.toLocaleString()}`, then the full hash in `font-mono text-sm break-all`, then `matched_at` (ISO, sliced to seconds) and a "back to list" `Link to "/"`.
  - Parties section: heading `Parties (${count})`, then either a paragraph "No parties annotated for this match." (when empty) or a wrapped row of `<PartyChip>`s for each entry of `parties`, passing `role`.
  - Collapsible `<details>` titled "Raw lifted JSON (debug)" containing `JSON.stringify(JSON.parse(rawLifted), null, 2)` in a `<pre>`.

**Acceptance**
- Following a row from `/` lands on the detail with parties filled.
- Hitting an unknown hash returns 404 (TanStack Router handles `notFound()`).
- The "Raw lifted JSON" accordion expands to valid pretty JSON.

**Commit:** `feat(frontend): match detail view; remove /txs index`

---

### Task 10: Slim the Header

**Goal:** drop the now-irrelevant Transactions nav link.

**Files**: `frontend/src/components/Header.tsx`.

**Final shape**: brand `Link to "/"`, a small "Matches view" caption (`text-xs text-muted-foreground`), `<ThemeToggle/>` pushed right via `ml-auto`. No `<nav>` element.

**Acceptance**: `pnpm typecheck` clean; manual confirm — only brand + caption + theme toggle visible.

**Commit:** `feat(frontend): slim header to brand + theme toggle`

---

### Task 11: Documentation

**Goal:** deliver M3-D — refreshed root README plus three docs files. Use Mermaid C4 syntax (`C4Context`, `C4Container`) for the architectural diagrams.

**Files**
- Modify: `README.md`
- Create: `docs/architecture.md`, `docs/access-patterns.md`, `docs/running.md`
- Modify: `frontend/README.md`

**Per-file content contract**

`README.md` (root):
- One-paragraph intro (what the dashboard is, who it's for).
- `C4Context` diagram (copy from spec §3.1).
- "Quick start" with the 4 commands across two terminals.
- "What the demo shows" paragraph (single-tx-name `buy_ticket` protocol on preview).
- Links to the three `docs/*.md`.
- Project structure tree.

`docs/architecture.md`:
- C4Context + C4Container diagrams (copy from spec §3.1).
- A `sequenceDiagram` for "data flow per page load": Browser → Nitro server fn → Kysely → SQLite → back.
- Component responsibilities section (3 bullets — tracker, dashboard, SQLite).
- "Why this shape" section (4 bullets, copied/condensed from spec §3.2).
- "Forward-looking: Postgres" section linking to spec §4.3.

`docs/access-patterns.md`:
- AP-1 / AP-2 summary table (Pattern, Function, Used by, SQL).
- Per-pattern subsection with the input/output type signatures (copy from plan Tasks 6).
- "Deferred patterns" section listing the four MVP-deferred capabilities (overview metrics, accounts explorer, script log, anomalies).

`docs/running.md`:
- Prerequisites bullet list (Rust stable, Node 24, pnpm 10, sibling tx3-lift clone, utxorpc endpoint).
- Env vars table: `DMTR_API_KEY`, `TRACKER_DB_PATH`, `RUST_LOG`.
- First-run section with the 4 commands across two terminals.
- Production build snippet (`pnpm build` + `node .output/server/index.mjs`).
- Troubleshooting subsections: empty list, `SQLITE_CANTOPEN`, native build failure, stale match after rollback.
- "Tested with" section — leave the tracker commit hash as a placeholder for Task 12 to fill.
- "Deferred deployment polish" bullets (Docker compose, service-manager units, Postgres).

`frontend/README.md`: 3-line slim — links to root README and `docs/running.md` plus the `pnpm dev / build / test / typecheck / check` script list.

**Acceptance**
- All four files render correctly on GitHub. C4 diagrams display (mermaid C4 is beta — if rendering breaks, fall back to standard `flowchart` syntax with a node note in the commit message).
- `grep -r "TODO\|TBD\|XXX" docs/ README.md frontend/README.md` returns only the deliberate "Tested with" placeholder in `docs/running.md`.

**Commit:** `docs: refresh M3 documentation (architecture, access patterns, running)`

---

### Task 12: Smoke checklist (manual)

**Goal:** end-to-end verification before declaring M3 complete.

**Steps**

1. **All checks green:** `(cd frontend && pnpm test --run && pnpm typecheck && pnpm check)`.

2. **Run the tracker for ≥ 5 minutes** against preview:
   ```bash
   cd ../tx3-lift
   DMTR_API_KEY=dmtr_… RUST_LOG=tracker=info \
     cargo run --release -p tracker -- ../dashboard/tracker.toml
   ```
   Expect at least one `matched tx=…` log line. Confirm rows in the DB:
   ```bash
   sqlite3 tracker.db "SELECT count(*), max(block_slot) FROM matches"
   ```

3. **Open the dashboard against the populated DB**: `(cd frontend && pnpm dev)`, then visit the printed URL. Confirm:
   - List shows the row(s) seen in the tracker log.
   - Clicking a row lands on `/txs/$hash` with parties populated.
   - Raw JSON accordion expands to valid JSON.
   - With an empty DB (delete `tracker.db` and reload), the empty state renders.

4. **Pin the tested tracker commit**:
   ```bash
   git -C ../tx3-lift rev-parse --short HEAD
   ```
   Update the placeholder in `docs/running.md` and commit (`docs: pin tested tx3-lift commit`).

5. **Capture screenshots** via `openwolf designqc`. Choose one matches-list and one match-detail JPEG, copy into `docs/screenshots/`, embed in `README.md` under "What the demo shows", and commit (`docs: add MVP screenshots to README`).

**Acceptance**: every step above produces the expected outcome. M3 acceptance criteria A1, B1, C1, D1 all met.

---

## Self-review

**Spec coverage**
- §3 Architecture (two-process topology, C4 Context + Container) → Tasks 1–10 collectively assemble the dashboard side; Task 2 sets up the tracker side.
- §4 Data model (observe tracker schema, parse lifted) → Tasks 4, 5.
- §5 Access patterns AP-1 / AP-2 → Task 6 implements both queries; Tasks 8, 9 wire them into routes.
- §6 Frontend (Kysely, layout, components) → Tasks 3, 4, 7, 8, 9, 10.
- §7 Configuration & deployment → Task 2 (tracker.toml + TII), Task 4 (`TRACKER_DB_PATH`), Task 11 (`docs/running.md`).
- §8 Documentation → Task 11.
- Smoke / acceptance → Task 12.

**Style compliance**
- Each task specifies goal, files, contract (where code is involved), tests with concrete assertions, and acceptance — implementer chooses the implementation.
- Literal content is reserved for SQL, TOML, JSON shapes, and shell commands (i.e. configuration and verification, not implementation).
- No "implement later" / "TBD" / "similar to Task N without code" placeholders.

**Type consistency**
- `MatchesRow`, `CursorRow`, `DashboardDatabase` defined in Task 4, consumed by Task 6 (`db.selectFrom('matches')`).
- `MatchRow` defined in Task 6, consumed by Tasks 8, 9 (route loaders).
- `LiftedParty`, `Lifted`, `bytesToHex`, `truncateHex`, `parseLifted` defined in Task 5; `parseLifted` consumed in Task 6; `truncateHex` consumed in Task 7.
- `TRACKER_DB_PATH` env var name consistent across Task 4 default-resolver, Task 11 docs, root README.

---

## Execution handoff

Plan complete. Two execution options:

1. **Subagent-driven (recommended)** — fresh subagent per task with two-stage review (spec → code quality).
2. **Inline execution** — execute in this session via `executing-plans` with checkpoints.

Which approach?
