import { createFileRoute, Link, notFound } from '@tanstack/react-router';
import { createServerFn } from '@tanstack/react-start';
import PartyChip from '../../components/PartyChip';
import TxNamePill from '../../components/TxNamePill';
import { createDb } from '../../lib/db';
import { getMatch, type MatchRow } from '../../lib/queries';

const fetchMatch = createServerFn({ method: 'GET' })
	.inputValidator((hash: string) => hash)
	.handler(async ({ data: hash }): Promise<MatchRow | null> => {
		const db = createDb();
		try {
			return await getMatch(db, hash);
		} finally {
			await db.destroy();
		}
	});

export const Route = createFileRoute('/txs/$hash')({
	component: MatchDetailView,
	loader: async ({ params }) => {
		const match = await fetchMatch({ data: params.hash });
		if (match === null) {
			throw notFound();
		}
		return match;
	},
});

function MatchDetailView() {
	const match = Route.useLoaderData();

	return (
		<div className="space-y-8">
			<Header match={match} />
			<PartiesSection parties={match.parties} />
			<RawLiftedDetails rawLifted={match.rawLifted} />
		</div>
	);
}

function Header({ match }: { match: MatchRow }) {
	const matchedAt = match.matchedAt instanceof Date ? match.matchedAt : new Date(match.matchedAt);
	const when = matchedAt.toISOString().replace('T', ' ').slice(0, 19);

	return (
		<div className="space-y-3">
			<div className="flex items-center gap-3">
				<TxNamePill name={match.txName} />
				<span className="text-sm text-muted-foreground">
					{match.protocolName} · {match.profileName} · slot {match.blockSlot.toLocaleString()}
				</span>
			</div>
			<p className="font-mono text-sm break-all">{match.hash}</p>
			<div className="flex items-center gap-4 text-xs text-muted-foreground">
				<span className="font-mono">{when}</span>
				<Link to="/" className="text-primary hover:underline">
					← back to list
				</Link>
			</div>
		</div>
	);
}

function PartiesSection({ parties }: { parties: MatchRow['parties'] }) {
	const entries = Object.entries(parties);

	return (
		<section className="space-y-3">
			<h2 className="text-lg font-semibold tracking-tight">Parties ({entries.length})</h2>
			{entries.length === 0 ? (
				<p className="text-sm text-muted-foreground">No parties annotated for this match.</p>
			) : (
				<div className="flex flex-wrap gap-2">
					{entries.map(([name, party]) => (
						<PartyChip key={name} name={name} address={party.address} role={party.role} />
					))}
				</div>
			)}
		</section>
	);
}

function RawLiftedDetails({ rawLifted }: { rawLifted: string }) {
	let pretty: string;
	try {
		pretty = JSON.stringify(JSON.parse(rawLifted), null, 2);
	} catch {
		pretty = rawLifted;
	}

	return (
		<details className="rounded-lg border border-border bg-muted/20 p-4">
			<summary className="cursor-pointer text-sm font-medium">Raw lifted JSON (debug)</summary>
			<pre className="mt-3 overflow-x-auto font-mono text-xs">{pretty}</pre>
		</details>
	);
}
