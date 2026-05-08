import { createFileRoute, Link } from '@tanstack/react-router';
import { createServerFn } from '@tanstack/react-start';
import PartyChip from '../components/PartyChip';
import TxNamePill from '../components/TxNamePill';
import { createDb } from '../lib/db';
import { truncateHex } from '../lib/lifted';
import { listMatches, type MatchRow } from '../lib/queries';

const fetchMatches = createServerFn({ method: 'GET' }).handler(async (): Promise<MatchRow[]> => {
	const db = createDb();
	try {
		return await listMatches(db, 50);
	} finally {
		await db.destroy();
	}
});

export const Route = createFileRoute('/')({
	component: MatchesView,
	loader: () => fetchMatches(),
});

function MatchesView() {
	const matches = Route.useLoaderData();

	return (
		<div className="space-y-6">
			<div className="flex items-baseline justify-between">
				<h1 className="text-2xl font-bold tracking-tight">Matches</h1>
				<p className="text-sm text-muted-foreground">{matches.length} most recent</p>
			</div>
			{matches.length === 0 ? <EmptyState /> : <MatchesTable matches={matches} />}
		</div>
	);
}

function EmptyState() {
	return (
		<div className="rounded-lg border border-dashed border-border bg-background px-6 py-12 text-center">
			<p className="text-sm text-muted-foreground">
				No matches yet — confirm the tracker is running. See <code className="font-mono">docs/running.md</code>.
			</p>
		</div>
	);
}

function MatchesTable({ matches }: { matches: MatchRow[] }) {
	return (
		<div className="overflow-x-auto rounded-lg border border-border">
			<table className="w-full text-sm">
				<thead className="border-b border-border bg-muted/30 text-left text-xs uppercase tracking-wide text-muted-foreground">
					<tr>
						<th className="px-4 py-2 font-medium">Tx</th>
						<th className="px-4 py-2 font-medium">Hash</th>
						<th className="px-4 py-2 font-medium">Slot</th>
						<th className="px-4 py-2 font-medium">Parties</th>
						<th className="px-4 py-2 font-medium">When</th>
					</tr>
				</thead>
				<tbody className="divide-y divide-border">
					{matches.map(m => (
						<MatchRowItem key={m.id} match={m} />
					))}
				</tbody>
			</table>
		</div>
	);
}

function MatchRowItem({ match }: { match: MatchRow }) {
	const matchedAt = match.matchedAt instanceof Date ? match.matchedAt : new Date(match.matchedAt);
	const when = matchedAt.toISOString().replace('T', ' ').slice(0, 19);

	return (
		<tr className="hover:bg-muted/20">
			<td className="px-4 py-3 align-middle">
				<TxNamePill name={match.txName} />
			</td>
			<td className="px-4 py-3 align-middle">
				<Link to="/txs/$hash" params={{ hash: match.hash }} className="font-mono text-sm text-primary hover:underline">
					{truncateHex(match.hash)}
				</Link>
			</td>
			<td className="px-4 py-3 align-middle">
				<span className="font-mono text-sm text-muted-foreground">{match.blockSlot.toLocaleString()}</span>
			</td>
			<td className="px-4 py-3 align-middle">
				<div className="flex flex-wrap gap-2">
					{Object.entries(match.parties).map(([name, party]) => (
						<PartyChip key={name} name={name} address={party.address} role={party.role} />
					))}
				</div>
			</td>
			<td className="px-4 py-3 align-middle">
				<span className="font-mono text-xs text-muted-foreground">{when}</span>
			</td>
		</tr>
	);
}
