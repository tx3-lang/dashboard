# Access patterns

The MVP defines two read-only access patterns against the tracker's `matches` table. Both run inside TanStack Start's `createServerFn` (Nitro side, never in the browser) and return typed rows via Kysely plus a manually-typed view of the parsed `lifted` JSON.

## Summary

| ID | Pattern | Function | Used by | SQL |
|----|---------|----------|---------|-----|
| AP-1 | Recent matches list | `listMatches(db, limit)` | `/` (matches list) | `SELECT id, tx_hash, tx_name, block_slot, lifted, matched_at FROM matches ORDER BY id DESC LIMIT ?` |
| AP-2 | Single match detail | `getMatch(db, txHashHex)` | `/txs/$hash` (detail) | `SELECT id, tx_hash, tx_name, block_slot, lifted, matched_at FROM matches WHERE tx_hash = ? LIMIT 1` |

Both functions live in [`frontend/src/lib/queries.ts`](../frontend/src/lib/queries.ts). The shared `MatchRow` shape returned to the route loader is:

```ts
export interface MatchRow {
    readonly id: number;
    readonly hash: string;             // hex-encoded tx_hash
    readonly txName: string;
    readonly protocolName: string;
    readonly profileName: string;
    readonly blockSlot: number;
    readonly matchedAt: Date;
    readonly parties: Record<string, LiftedParty>;
    readonly rawLifted: string;
}
```

`LiftedParty` (from `lib/lifted.ts`) carries the named address bytes re-encoded as a hex string and the role (`Input` / `Output`) the lifter recorded. bech32 rendering is intentionally deferred for the MVP.

## AP-1 — `listMatches`

Returns the most recent matches across all sources, newest first. Used by the matches list at `/`.

```ts
async function listMatches(
    db: Kysely<DashboardDatabase>,
    limit: number = 50,
): Promise<MatchRow[]>
```

`limit` defaults to 50 and is clamped to `[1, 200]` to keep page sizes bounded. The query selects `id, tx_hash, block_slot, protocol_name, profile_name, tx_name, lifted, matched_at`, orders by `id DESC` (the tracker's monotonic insertion order), and applies `LIMIT ?`. Each row is mapped to a `MatchRow` with `tx_hash` re-encoded as hex and `lifted` parsed into typed parties.

## AP-2 — `getMatch`

Returns a single match by its hex-encoded `tx_hash`, or `null` if not found. Used by the detail page at `/txs/$hash`.

```ts
async function getMatch(
    db: Kysely<DashboardDatabase>,
    txHashHex: string,
): Promise<MatchRow | null>
```

The function validates that `txHashHex` is an even-length hex string, decodes it to a `Buffer`, and runs `WHERE tx_hash = ? LIMIT 1`. The `(tx_hash, source_name)` unique index on the tracker side means at most one row per source per hash; for the single-source MVP, one row per hash.

## Deferred patterns

These were scoped out of the MVP and are listed here for future iterations:

- **Activity Overview / time series** — aggregate counts of matches per `tx_name` over time, suitable for a small dashboard chart.
- **Accounts (Address explorer)** — given an address, list every match that involved it as a party.
- **Script Execution Log** — surface decoded script inputs and witnesses from the `lifted` JSON for protocol developers in debug mode.
- **Anomalies / alerts** — surface gaps in the cursor, unexpectedly long matched-at intervals, or matches that fail the lifter post-hoc.

Each of these is feasible against the same `matches` table without schema changes; they were deferred to keep the M3 surface narrow.
