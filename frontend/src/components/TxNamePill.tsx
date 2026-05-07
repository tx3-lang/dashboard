interface TxNamePillProps {
	name: string;
}

export default function TxNamePill({ name }: TxNamePillProps) {
	return (
		<span className="inline-flex items-center rounded-full bg-primary/10 px-2 py-0.5 text-xs font-semibold text-primary">
			{name}
		</span>
	);
}
