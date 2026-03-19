export default function Footer() {
	return (
		<footer className="border-t border-border">
			<div className="mx-auto flex h-12 max-w-7xl items-center justify-center px-4 text-sm text-muted-foreground">
				&copy; {new Date().getFullYear()} tx3
			</div>
		</footer>
	);
}
