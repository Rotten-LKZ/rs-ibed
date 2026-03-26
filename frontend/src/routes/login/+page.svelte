<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { loginWithToken, loginWithCredentials } from '$lib/api';
	import Button from '$lib/components/Button.svelte';
	import { t, toggleLocale, getLocale } from '$lib/i18n/index.svelte';
	import { toggleTheme, getTheme } from '$lib/stores/theme.svelte';

	let { data } = $props();

	let input = $state('');
	let error = $state('');
	let loading = $state(false);
	let tokenHandled = $state(false);

	async function finishLogin(work: () => Promise<unknown>) {
		loading = true;
		error = '';
		try {
			await work();
			await goto(resolve('/'));
		} catch {
			error = t('login.error');
			loading = false;
		}
	}

	async function handleTokenLogin(token: string) {
		await finishLogin(() => loginWithToken(token));
	}

	async function handleSubmit(e: SubmitEvent) {
		e.preventDefault();
		if (!input.trim()) return;
		await finishLogin(() => loginWithCredentials(input));
	}

	$effect(() => {
		if (data.token && !tokenHandled) {
			tokenHandled = true;
			handleTokenLogin(data.token);
		}
	});
</script>

<div class="min-h-screen bg-gray-50 px-4 py-6 dark:bg-gray-900 sm:px-6 lg:px-8">
	<div class="mx-auto flex min-h-[calc(100vh-3rem)] max-w-5xl items-center justify-center">
		<div class="grid w-full gap-6 rounded-2xl bg-white p-5 shadow-sm ring-1 ring-gray-200 dark:bg-gray-800 dark:ring-gray-700 md:grid-cols-[1.1fr_0.9fr] md:p-8">
			<div class="space-y-4">
				<div class="flex flex-wrap items-center justify-between gap-3">
					<h1 class="text-2xl font-bold text-gray-900 dark:text-white">{t('login.title')}</h1>
					<div class="flex gap-2">
						<button
							onclick={toggleLocale}
							class="rounded-md px-2 py-1 text-xs text-gray-600 hover:bg-gray-100 dark:text-gray-300 dark:hover:bg-gray-700"
						>
							{getLocale() === 'en' ? '中文' : 'EN'}
						</button>
						<button
							onclick={toggleTheme}
							class="rounded-md px-2 py-1 text-xs text-gray-600 hover:bg-gray-100 dark:text-gray-300 dark:hover:bg-gray-700"
						>
							{getTheme() === 'light' ? '🌙' : '☀️'}
						</button>
					</div>
				</div>
				<p class="text-sm leading-6 text-gray-600 dark:text-gray-300">{t('login.description')}</p>
				<div class="rounded-xl bg-gray-100 p-4 text-sm text-gray-700 dark:bg-gray-900/50 dark:text-gray-200">
					<p class="font-medium">{t('login.tipTitle')}</p>
					<p class="mt-2 leading-6">{t('login.tipBody')}</p>
				</div>
			</div>

			<div class="space-y-4">
				{#if data.token && loading}
					<div class="rounded-xl border border-blue-200 bg-blue-50 p-4 text-sm text-blue-700 dark:border-blue-900 dark:bg-blue-950/40 dark:text-blue-200">
						{t('login.tokenLoading')}
					</div>
				{/if}

				<form onsubmit={handleSubmit} class="space-y-4">
					<label class="block space-y-2">
						<span class="text-sm font-medium text-gray-800 dark:text-gray-100">{t('login.inputLabel')}</span>
						<input
							type="text"
							bind:value={input}
							placeholder={t('login.placeholder')}
							class="w-full rounded-xl border border-gray-300 px-3 py-3 text-sm dark:border-gray-600 dark:bg-gray-700 dark:text-white dark:placeholder-gray-400"
							autocomplete="off"
						/>
					</label>
					{#if error}
						<p class="rounded-xl border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-700 dark:border-red-900 dark:bg-red-950/40 dark:text-red-300">
							{error}
						</p>
					{/if}
					<Button type="submit" disabled={loading || !input.trim()} class="w-full">
						{loading ? t('login.loading') : t('login.submit')}
					</Button>
				</form>
			</div>
		</div>
	</div>
</div>
