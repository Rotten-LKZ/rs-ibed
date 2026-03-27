let theme = $state<'light' | 'dark'>('light');

export function getTheme() {
	return theme;
}

export function setTheme(t: 'light' | 'dark') {
	theme = t;
	if (typeof localStorage !== 'undefined') {
		localStorage.setItem('theme', t);
	}
	applyTheme(t);
}

export function toggleTheme() {
	setTheme(theme === 'light' ? 'dark' : 'light');
}

function applyTheme(t: 'light' | 'dark') {
	if (typeof document !== 'undefined') {
		document.documentElement.classList.toggle('dark', t === 'dark');
	}
}

export function initTheme() {
	if (typeof localStorage !== 'undefined') {
		const saved = localStorage.getItem('theme');
		if (saved === 'light' || saved === 'dark') {
			theme = saved;
		} else if (
			typeof window !== 'undefined' &&
			window.matchMedia('(prefers-color-scheme: dark)').matches
		) {
			theme = 'dark';
		}
		applyTheme(theme);
	}
}
