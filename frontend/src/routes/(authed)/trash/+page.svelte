<script lang="ts">
	import {
		emptyAllTrash,
		fetchImages,
		permanentDeleteManagedImage,
		restoreManagedImage
	} from '$lib/api';
	import { t } from '$lib/i18n/index.svelte';
	import { onMount } from 'svelte';
	import type { ImageModel } from '$lib/sdk';

	let items = $state<ImageModel[]>([]);
	let loading = $state(true);
	let error = $state('');
	let busyId = $state<number | null>(null);
	let busyGlobal = $state(false);

	async function loadTrash() {
		loading = true;
		error = '';
		try {
			const result = await fetchImages({ perPage: 100, deleted: true });
			items = result.items;
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
		busyGlobal = true;
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
		busyGlobal = false;
		await loadTrash();
	}

	async function handlePermanentDelete(item: ImageModel) {
		if (!confirm(t('trash.permanentDeleteConfirm'))) return;
		busyId = item.id;
		try {
			await permanentDeleteManagedImage(item.id);
			await loadTrash();
		} catch (e: unknown) {
			// 503 Service Unavailable = storage node unreachable
			const msg = e instanceof Error ? e.message : String(e);
			if (
				msg.includes('503') ||
				msg.toLowerCase().includes('storage') ||
				msg.toLowerCase().includes('unavailable')
			) {
				error = t('trash.permanentDeleteFailed');
			} else {
				error = t('home.actionError');
			}
		} finally {
			busyId = null;
		}
	}

	async function handleEmptyTrash() {
		if (!confirm(t('trash.emptyTrashConfirm'))) return;
		busyGlobal = true;
		try {
			const result = await emptyAllTrash();
			if (!result.ok && result.failed > 0) {
				// Partial failure - some succeeded, some failed
				error = t('trash.emptyTrashPartial')
					.replace('{succeeded}', String(result.succeeded))
					.replace('{failed}', String(result.failed));
			}
			await loadTrash();
		} catch (e: unknown) {
			// 503 Service Unavailable = storage node unreachable
			const msg = e instanceof Error ? e.message : String(e);
			if (
				msg.includes('503') ||
				msg.toLowerCase().includes('storage') ||
				msg.toLowerCase().includes('unavailable')
			) {
				error = t('trash.emptyTrashFailed');
			} else {
				error = t('home.actionError');
			}
		} finally {
			busyGlobal = false;
		}
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
			<div class="flex gap-2">
				<button
					onclick={handleRestoreAll}
					disabled={busyId !== null || busyGlobal}
					class="rounded-lg bg-green-600 px-4 py-2 text-sm font-medium text-white hover:bg-green-700 disabled:opacity-40"
					>{t('trash.restoreAll')}</button
				>
				<button
					onclick={handleEmptyTrash}
					disabled={busyId !== null || busyGlobal}
					class="rounded-lg bg-red-600 px-4 py-2 text-sm font-medium text-white hover:bg-red-700 disabled:opacity-40"
					>{t('trash.emptyTrash')}</button
				>
			</div>
		{/if}
	</div>

	<p class="text-sm text-gray-500 dark:text-gray-400">{t('trash.info')}</p>

	{#if error}
		<p
			class="rounded-lg border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-700 dark:border-red-900 dark:bg-red-950/40 dark:text-red-300"
		>
			{error}
		</p>
	{/if}

	{#if loading}
		<div
			class="rounded-xl border border-gray-200 bg-white p-6 text-sm text-gray-500 dark:border-gray-700 dark:bg-gray-800"
		>
			{t('home.loading')}
		</div>
	{:else if items.length === 0}
		<div
			class="rounded-xl border border-gray-200 bg-white p-6 text-sm text-gray-500 dark:border-gray-700 dark:bg-gray-800"
		>
			{t('trash.empty')}
		</div>
	{:else}
		<div class="grid grid-cols-1 gap-4 sm:grid-cols-2 xl:grid-cols-3">
			{#each items as item (item.id)}
				<article
					class="overflow-hidden rounded-xl border border-gray-200 bg-white dark:border-gray-700 dark:bg-gray-800"
				>
					<div
						class="flex aspect-video items-center justify-center bg-gray-100 text-sm text-gray-400 dark:bg-gray-900"
					>
						{t('home.deletedBadge')}
					</div>
					<div class="space-y-2 p-3">
						<div class="min-w-0">
							<h3 class="truncate text-sm font-medium text-gray-900 dark:text-white">
								{item.display_name}
							</h3>
							<p class="text-xs text-gray-500 dark:text-gray-400">
								#{item.id} · {item.width}×{item.height} · {formatSize(item.size)}
							</p>
						</div>
						<div class="flex gap-2">
							<button
								onclick={() => handleRestore(item)}
								disabled={busyId === item.id || busyGlobal}
								class="rounded-lg bg-green-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-green-700 disabled:opacity-40"
								>{t('home.restore')}</button
							>
							<button
								onclick={() => handlePermanentDelete(item)}
								disabled={busyId === item.id || busyGlobal}
								class="rounded-lg bg-red-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-red-700 disabled:opacity-40"
								>{t('trash.permanentDelete')}</button
							>
						</div>
					</div>
				</article>
			{/each}
		</div>
	{/if}
</div>
