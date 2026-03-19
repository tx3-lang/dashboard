import { createFileRoute } from '@tanstack/react-router';

export const Route = createFileRoute('/txs/$hash')({
	component: TransactionDetail,
});

function TransactionDetail() {
	const { hash } = Route.useParams();

	return (
		<div className="space-y-6">
			<h1 className="text-2xl font-bold tracking-tight">Transaction Detail</h1>
			<p className="font-mono text-sm text-muted-foreground break-all">{hash}</p>
		</div>
	);
}
