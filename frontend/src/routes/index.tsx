import { createFileRoute } from '@tanstack/react-router';

export const Route = createFileRoute('/')({
	component: DashboardOverview,
});

function DashboardOverview() {
	return (
		<div className="space-y-6">
			<h1 className="text-2xl font-bold tracking-tight">Overview</h1>
			<p className="text-muted-foreground">Dashboard overview page.</p>
		</div>
	);
}
