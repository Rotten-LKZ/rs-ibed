import en from './en';
import zh from './zh';

const translations: Record<string, Record<string, string>> = { en, zh };

let locale = $state('en');

export function getLocale() {
	return locale;
}

export function setLocale(l: string) {
	locale = l;
	if (typeof localStorage !== 'undefined') {
		localStorage.setItem('locale', l);
	}
}

export function toggleLocale() {
	setLocale(locale === 'en' ? 'zh' : 'en');
}

export function t(key: string): string {
	return translations[locale]?.[key] ?? key;
}

export function initLocale() {
	if (typeof localStorage !== 'undefined') {
		const saved = localStorage.getItem('locale');
		if (saved && translations[saved]) {
			locale = saved;
		}
	}
}
