import { describe, expect, it } from 'vitest';
import { bytesToHex, parseLifted, truncateHex } from '../lifted';

describe('bytesToHex', () => {
	it('encodes a non-empty byte array as lowercase hex without 0x prefix', () => {
		expect(bytesToHex([0xab, 0x01, 0xff])).toBe('ab01ff');
	});

	it('returns an empty string for an empty array', () => {
		expect(bytesToHex([])).toBe('');
	});

	it('throws when a value is outside the byte range', () => {
		expect(() => bytesToHex([300])).toThrow(/byte/i);
	});
});

describe('truncateHex', () => {
	it('truncates a long hex string with a horizontal ellipsis', () => {
		expect(truncateHex('0123456789abcdef0123456789abcdef', 6)).toBe('012345…abcdef');
	});

	it('returns the input unchanged when shorter than or equal to edge*2', () => {
		expect(truncateHex('abcd', 6)).toBe('abcd');
	});
});

describe('parseLifted', () => {
	it('re-encodes party byte addresses as hex strings', () => {
		const json = JSON.stringify({
			tx_name: 'buy_ticket',
			parties: {
				buyer: { address: [0x61, 0x12, 0x34], role: 'Input' },
				treasury: { address: [0x61, 0xab], role: 'Output' },
			},
		});

		const result = parseLifted(json);

		expect(result.txName).toBe('buy_ticket');
		expect(result.parties).toEqual({
			buyer: { address: '611234', role: 'Input' },
			treasury: { address: '61ab', role: 'Output' },
		});
	});

	it('returns an empty parties object when parties is missing', () => {
		const result = parseLifted(JSON.stringify({ tx_name: 'noop' }));
		expect(result.parties).toEqual({});
	});

	it('throws on invalid JSON', () => {
		expect(() => parseLifted('not json')).toThrow();
	});
});
