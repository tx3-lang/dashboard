import Database from 'better-sqlite3';
import { describe, expect, it } from 'vitest';
import { createDb } from '../db';

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
`;

describe('createDb', () => {
	it('wraps an existing better-sqlite3 instance and runs a typed query', async () => {
		const sqlite = new Database(':memory:');
		sqlite.exec(SCHEMA_SQL);

		const db = createDb({ existing: sqlite });

		const rows = await db.selectFrom('matches').select('id').execute();

		expect(rows).toEqual([]);

		await db.destroy();
	});
});
