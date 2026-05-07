import { Link } from '@tanstack/react-router';
import ThemeToggle from './ThemeToggle';

export default function Header() {
	return (
		<header className="sticky top-0 z-50 border-b border-border bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
			<div className="mx-auto flex h-14 max-w-7xl items-center gap-4 px-4">
				<Link to="/" className="text-lg font-semibold">
					tx3 Dashboard
				</Link>
				<span className="text-xs text-muted-foreground">Matches view</span>
				<div className="ml-auto">
					<ThemeToggle />
				</div>
			</div>
		</header>
	);
}
