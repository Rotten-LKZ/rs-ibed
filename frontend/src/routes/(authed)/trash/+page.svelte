<script lang="ts">
	import { fetchImages, restoreManagedImage } from '$lib/api';
	import { t } from '$lib/i18n/index.svelte';
	import { onMount } from 'svelte';
	import type { ImageModel } from '$lib/sdk';

	let items = $state<ImageModel[]>([]);
	let loading = $state(true);
	let error = $state('');
	let busyId = $state<number | null>(null);

	async function loadTrash() {
		loading = true;
		error = '';
		try {
			// API doesn't have a deleted filter, so fetch all and filter client-side
			const result = await fetchImages({ perPage: 100 });
			items = result.items.filter((i) => i.is_deleted);
		} catch {
			error = t('home.loadError');
		} finally {
			loading = false;
		}
	}

	onMount(loadTrash);

	async function handleRestore(item: ImageModel) {
		busyId = item.id;
		try {
			await restoreManagedImage(item.id);
			await loadTrash();
		} catch {
			error = t('home.actionError');
		} finally {
			busyId = null;
		}
	}

	async function handleRestoreAll() {
		for (const item of items) {
			busyId = item.id;
			try {
				await restoreManagedImage(item.id);
			} catch {
				error = t('home.actionError');
				break;
			}
		}
		busyId = null;
		await loadTrash();
	}

	function formatSize(bytes: number): string {
		if (bytes < 1024) return bytes + ' B';
		if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB';
		return (bytes / (1024 * 1024)).toFixed(1) + ' MB';
	}
</script>

<div class="space-y-4">
	<div class="flex items-center justify-between">
		<h1 class="text-2xl font-bold text-gray-900 dark:text-white">{t('trash.title')}</h1>
		{#if items.length > 0}
			<button
				onclick={handleRestoreAll}
				disabled={busyId !== null}
				class="rounded-lg bg-green-600 px-4 py-2 text-sm font-medium text-white hover:bg-green-700 disabled:opacity-40"
			>{t('trash.restoreAll')}</button>
		{/if}
	</div>

	<p class="text-sm text-gray-500 dark:text-gray-400">{t('trash.info')}</p>

	{#if error}
		<p class="rounded-lg border border-red-200 bg-red-50 dark:border-red-900 dark:bg-red-950/40 px-4 py-3 text-sm text-red-700 dark:text-red-300">{error}</p>
	{/if}

	{#if loading}
		<div class="rounded-xl border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 p-6 text-sm text-gray-500">{t('home.loading')}</div>
	{:else if items.length === 0}
		<div class="rounded-xl border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 p-6 text-sm text-gray-500">{t('trash.empty')}</div>
	{:else}
		<div class="grid gap-4 grid-cols-1 sm:grid-cols-2 xl:grid-cols-3">
			{#each items as item (item.id)}
				<article class="overflow-hidden rounded-xl border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800">
					<div class="aspect-video bg-gray-100 dark:bg-gray-900 flex items-center justify-center text-sm text-gray-400">
						{t('home.deletedBadge')}
					</div>
					<div class="space-y-2 p-3">
						<div class="min-w-0">
							<h3 class="truncate text-sm font-medium text-gray-900 dark:text-white">{item.display_name}</h3>
							<p class="text-xs text-gray-500 dark:text-gray-400">#{item.id} · {item.width}×{item.height} · {formatSize(item.size)}</p>
						</div>
						<button
							onclick={() => handleRestore(item)}
							disabled={busyId === item.id}
							class="rounded-lg bg-green-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-green-700 disabled:opacity-40"
						>{t('home.restore')}</button>
					</div>
				</article>
			{/each}
		</div>
	{/if}
</div>
