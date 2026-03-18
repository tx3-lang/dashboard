import { createFileRoute } from '@tanstack/react-router';

export const Route = createFileRoute('/txs/')({
	component: TransactionList,
});

function TransactionList() {
	return (
		<div className="space-y-6">
			<h1 className="text-2xl font-bold tracking-tight">Transactions</h1>
			<p className="text-muted-foreground">List of recent transactions.</p>
		</div>
	);
}
