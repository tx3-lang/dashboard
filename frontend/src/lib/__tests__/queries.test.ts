import Database from 'better-sqlite3';
import { describe, expect, it } from 'vitest';
import { createDb } from '../db';
import { getMatch, listMatches } from '../queries';

const SCHEMA_SQL = `
	CREATE TABLE matches (
		id INTEGER PRIMARY KEY,
		tx_hash BLOB NOT NULL,
		block_slot INTEGER NOT NULL,
		block_hash BLOB NOT NULL,
		source_name TEXT NOT NULL,
		protocol_name TEXT NOT NULL,
		tx_name TEXT NOT NULL,
		profile_name TEXT NOT NULL,
		lifted TEXT NOT NULL,
		matched_at INTEGER NOT NULL,
		UNIQUE(tx_hash, source_name)
	);
	CREATE TABLE cursor (
		id INTEGER PRIMARY KEY CHECK (id = 1),
		slot INTEGER NOT NULL,
		block_hash BLOB NOT NULL
	);
	CREATE TABLE _schema_versions (
		name TEXT PRIMARY KEY,
		applied_at INTEGER NOT NULL
	);
`;

interface SeedRow {
	tx_hash: Buffer;
	block_slot: number;
	matched_at: number;
	lifted: string;
}

function seed(): { sqlite: Database.Database; rows: SeedRow[] } {
	const sqlite = new Database(':memory:');
	sqlite.exec(SCHEMA_SQL);

	const rows: SeedRow[] = [
		{
			tx_hash: Buffer.from([0x01, 0x01]),
			block_slot: 100,
			matched_at: 1700000000,
			lifted: JSON.stringify({
				tx_name: 'buy_ticket',
				parties: {
					buyer: { address: [0x61, 0x01], role: 'Input' },
					treasury: { address: [0x61, 0x02], role: 'Output' },
				},
			}),
		},
		{
			tx_hash: Buffer.from([0x02, 0x02]),
			block_slot: 110,
			matched_at: 1700000050,
			lifted: JSON.stringify({
				tx_name: 'buy_ticket',
				parties: {
					buyer: { address: [0x61, 0x02], role: 'Input' },
					treasury: { address: [0x61, 0x03], role: 'Output' },
				},
			}),
		},
	];

	const insert = sqlite.prepare(
		`INSERT INTO matches (tx_hash, block_slot, block_hash, source_name, protocol_name, tx_name, profile_name, lifted, matched_at)
		 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)`,
	);
	for (const row of rows) {
		insert.run(
			row.tx_hash,
			row.block_slot,
			Buffer.from([0x00]),
			'utxorpc',
			'ticketing-2026',
			'buy_ticket',
			'preview',
			row.lifted,
			row.matched_at,
		);
	}

	return { sqlite, rows };
}

describe('listMatches', () => {
	it('returns matches newest-first with parsed lifted parties', async () => {
		const { sqlite } = seed();
		const db = createDb({ existing: sqlite });

		const result = await listMatches(db, 10);

		expect(result).toHaveLength(2);
		expect(result[0].hash).toBe('0202');
		expect(result[0].txName).toBe('buy_ticket');
		expect(result[0].parties.buyer.address).toBe('6102');
		expect(result[0].matchedAt).toEqual(new Date(1700000050 * 1000));

		await db.destroy();
	});

	it('respects the limit parameter', async () => {
		const { sqlite } = seed();
		const db = createDb({ existing: sqlite });

		const result = await listMatches(db, 1);

		expect(result).toHaveLength(1);

		await db.destroy();
	});
});

describe('getMatch', () => {
	it('returns the match for a valid hex tx_hash', async () => {
		const { sqlite } = seed();
		const db = createDb({ existing: sqlite });

		const result = await getMatch(db, '0101');

		expect(result).not.toBeNull();
		expect(result?.parties.treasury.address).toBe('6102');

		await db.destroy();
	});

	it('returns null when no match is found', async () => {
		const { sqlite } = seed();
		const db = createDb({ existing: sqlite });

		const result = await getMatch(db, 'deadbeef');

		expect(result).toBeNull();

		await db.destroy();
	});

	it('rejects empty / non-hex / odd-length input', async () => {
		const { sqlite } = seed();
		const db = createDb({ existing: sqlite });

		await expect(getMatch(db, '')).rejects.toThrow(/tx_hash hex/i);
		await expect(getMatch(db, 'xyz')).rejects.toThrow(/tx_hash hex/i);
		await expect(getMatch(db, 'abc')).rejects.toThrow(/tx_hash hex/i);

		await db.destroy();
	});
});
