import type { Kysely } from 'kysely';
import type { DashboardDatabase } from './db';
import { bytesToHex, type LiftedParty, parseLifted } from './lifted';

export interface MatchRow {
	readonly id: number;
	readonly hash: string;
	readonly txName: string;
	readonly protocolName: string;
	readonly profileName: string;
	readonly blockSlot: number;
	readonly matchedAt: Date;
	readonly parties: Record<string, LiftedParty>;
	readonly rawLifted: string;
}

const HEX_RE = /^[0-9a-fA-F]+$/;
const DEFAULT_LIMIT = 50;
const MIN_LIMIT = 1;
const MAX_LIMIT = 200;

/**
 * Normalize a `tx_hash` BLOB column (Buffer | Uint8Array | number[]) into a
 * lowercase hex string. The driver will hand us a Buffer in practice, but
 * Kysely's typing has us go through Buffer.
 */
function blobToHex(blob: Buffer | Uint8Array): string {
	if (Buffer.isBuffer(blob)) {
		return blob.toString('hex');
	}
	return bytesToHex(Array.from(blob));
}

interface RawMatch {
	id: number;
	tx_hash: Buffer;
	block_slot: number;
	protocol_name: string;
	profile_name: string;
	lifted: string;
	matched_at: number;
}

function toMatchRow(raw: RawMatch): MatchRow {
	const lifted = parseLifted(raw.lifted);
	return {
		id: raw.id,
		hash: blobToHex(raw.tx_hash),
		txName: lifted.txName,
		protocolName: raw.protocol_name,
		profileName: raw.profile_name,
		blockSlot: raw.block_slot,
		matchedAt: new Date(raw.matched_at * 1000),
		parties: lifted.parties,
		rawLifted: lifted.raw,
	};
}

/**
 * AP-1 — list recent matches, newest-first.
 *
 * `limit` defaults to 50 and is clamped to `[1, 200]`.
 */
export async function listMatches(db: Kysely<DashboardDatabase>, limit: number = DEFAULT_LIMIT): Promise<MatchRow[]> {
	const clamped = Math.max(MIN_LIMIT, Math.min(MAX_LIMIT, limit));
	const rows = await db
		.selectFrom('matches')
		.select(['id', 'tx_hash', 'block_slot', 'protocol_name', 'profile_name', 'lifted', 'matched_at'])
		.orderBy('id', 'desc')
		.limit(clamped)
		.execute();
	return rows.map(row => toMatchRow(row as RawMatch));
}

/**
 * AP-2 — fetch a single match by its `tx_hash` (hex string).
 *
 * Throws if `txHashHex` is not a valid even-length hex string.
 */
export async function getMatch(db: Kysely<DashboardDatabase>, txHashHex: string): Promise<MatchRow | null> {
	if (!HEX_RE.test(txHashHex) || txHashHex.length % 2 !== 0) {
		throw new Error(`invalid tx_hash hex: ${txHashHex}`);
	}
	const buf = Buffer.from(txHashHex, 'hex');
	const row = await db
		.selectFrom('matches')
		.select(['id', 'tx_hash', 'block_slot', 'protocol_name', 'profile_name', 'lifted', 'matched_at'])
		.where('tx_hash', '=', buf)
		.limit(1)
		.executeTakeFirst();
	return row ? toMatchRow(row as RawMatch) : null;
}
