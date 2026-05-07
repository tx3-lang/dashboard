import { truncateHex } from '../lib/lifted';

interface PartyChipProps {
	name: string;
	address: string;
	role?: string;
}

export default function PartyChip({ name, address, role }: PartyChipProps) {
	return (
		<span className="inline-flex items-center gap-2 rounded-full border border-border bg-background px-3 py-1 text-sm">
			<span className="h-1.5 w-1.5 rounded-full bg-primary" aria-hidden />
			<span className="font-semibold">{name}</span>
			<span className="font-mono text-muted-foreground">{truncateHex(address)}</span>
			{role ? <span className="text-[10px] uppercase tracking-wide text-muted-foreground">{role}</span> : null}
		</span>
	);
}
