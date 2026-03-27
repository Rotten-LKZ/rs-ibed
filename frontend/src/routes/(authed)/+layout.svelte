<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { logoutUser } from '$lib/api';
	import Sidebar from '$lib/components/Sidebar.svelte';
	import { t, toggleLocale, getLocale } from '$lib/i18n/index.svelte';
	import { toggleTheme, getTheme } from '$lib/stores/theme.svelte';

	let { children, data } = $props();
	let sidebarOpen = $state(false);

	async function handleLogout() {
		await logoutUser();
		await goto(resolve('/login'));
	}
</script>

<div class="flex min-h-screen bg-gray-50 dark:bg-gray-950">
	<!-- Mobile overlay -->
	{#if sidebarOpen}
		<div
			class="fixed inset-0 z-20 bg-black/50 lg:hidden"
			aria-hidden="true"
			onclick={() => (sidebarOpen = false)}
		></div>
	{/if}

	<!-- Sidebar: hidden on mobile unless open, fixed on desktop -->
	<aside
		class="fixed inset-y-0 left-0 z-30 flex flex-col transition-transform duration-200
			{sidebarOpen ? 'translate-x-0' : '-translate-x-full'} lg:static lg:flex lg:translate-x-0"
	>
		<Sidebar onclose={() => (sidebarOpen = false)} />
	</aside>

	<!-- Main content -->
	<div class="flex min-w-0 flex-1 flex-col">
		<!-- Top bar -->
		<header
			class="sticky top-0 z-10 flex items-center gap-3 border-b border-gray-200 bg-white px-4 py-3 dark:border-gray-700 dark:bg-gray-900"
		>
			<!-- Hamburger for mobile -->
			<button
				class="rounded-lg p-2 text-gray-600 hover:bg-gray-100 lg:hidden dark:text-gray-400 dark:hover:bg-gray-800"
				aria-label="Open menu"
				onclick={() => (sidebarOpen = true)}
			>
				<svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
					<path
						stroke-linecap="round"
						stroke-linejoin="round"
						stroke-width="2"
						d="M4 6h16M4 12h16M4 18h16"
					/>
				</svg>
			</button>

			<span class="flex-1 text-sm font-medium text-gray-700 dark:text-gray-300">
				{t('home.welcome')}: <span class="font-semibold">{data?.me.user ?? ''}</span>
			</span>

			<button
				onclick={toggleTheme}
				class="rounded-lg p-2 text-sm text-gray-600 hover:bg-gray-100 dark:text-gray-400 dark:hover:bg-gray-800"
				title={t('theme.toggle')}>{getTheme() === 'dark' ? '☀' : '🌙'}</button
			>

			<button
				onclick={toggleLocale}
				class="rounded-lg p-2 text-sm text-gray-600 hover:bg-gray-100 dark:text-gray-400 dark:hover:bg-gray-800"
				title={t('lang.toggle')}>{getLocale() === 'zh' ? 'EN' : '中'}</button
			>

			<button
				onclick={handleLogout}
				class="rounded-lg px-3 py-1.5 text-sm font-medium text-red-600 hover:bg-red-50 dark:text-red-400 dark:hover:bg-red-900/20"
				>{t('home.logout')}</button
			>
		</header>

		<!-- Page content -->
		<main class="flex-1 p-4 md:p-6">
			{@render children()}
		</main>
	</div>
</div>
