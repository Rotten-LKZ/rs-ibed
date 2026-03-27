<script lang="ts">
	import { resolve } from '$app/paths';
	import {
		fetchImages,
		deleteManagedImage,
		renameManagedImage,
		restoreManagedImage
	} from '$lib/api';
	import { t } from '$lib/i18n/index.svelte';
	import { onMount } from 'svelte';
	import type { ImageListItem } from '$lib/sdk';

	let items = $state<ImageListItem[]>([]);
	let total = $state(0);
	let currentPage = $state(1);
	let perPage = $state(20);
	let totalPages = $derived(Math.max(1, Math.ceil(total / perPage)));
	let loading = $state(true);
	let error = $state('');
	let search = $state('');
	let dateFrom = $state('');
	let dateTo = $state('');
	let busyId = $state<number | null>(null);
	let copiedId = $state<number | null>(null);

	async function loadImages() {
		loading = true;
		error = '';
		try {
			const result = await fetchImages({
				page: currentPage,
				perPage,
				name: search || undefined,
				dateFrom: dateFrom || undefined,
				dateTo: dateTo || undefined
			});
			items = result.items;
			total = result.total;
		} catch {
			error = t('home.loadError');
		} finally {
			loading = false;
		}
	}

	onMount(loadImages);

	function handleSearch(e: Event) {
		e.preventDefault();
		currentPage = 1;
		loadImages();
	}

	function resetFilters() {
		search = '';
		dateFrom = '';
		dateTo = '';
		currentPage = 1;
		loadImages();
	}

	async function handleRename(item: ImageListItem) {
		const name = prompt(t('home.renamePrompt'), item.display_name);
		if (!name || name === item.display_name) return;
		busyId = item.id;
		try {
			await renameManagedImage(item.id, name);
			await loadImages();
		} catch {
			error = t('home.actionError');
		} finally {
			busyId = null;
		}
	}

	async function handleDelete(item: ImageListItem) {
		busyId = item.id;
		try {
			await deleteManagedImage(item.id);
			await loadImages();
		} catch {
			error = t('home.actionError');
		} finally {
			busyId = null;
		}
	}

	async function handleRestore(item: ImageListItem) {
		busyId = item.id;
		try {
			await restoreManagedImage(item.id);
			await loadImages();
		} catch {
			error = t('home.actionError');
		} finally {
			busyId = null;
		}
	}

	function copyLink(item: ImageListItem) {
		const url = `${window.location.origin}${item.view_url}`;
		navigator.clipboard.writeText(url);
		copiedId = item.id;
		setTimeout(() => {
			copiedId = null;
		}, 1500);
	}

	function goPage(p: number) {
		currentPage = p;
		loadImages();
	}

	function formatSize(bytes: number): string {
		if (bytes < 1024) return bytes + ' B';
		if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB';
		return (bytes / (1024 * 1024)).toFixed(1) + ' MB';
	}

	function formatDate(iso: string): string {
		return new Date(iso).toLocaleDateString();
	}
</script>

<div class="space-y-4">
	<h1 class="text-2xl font-bold text-gray-900 dark:text-white">{t('images.title')}</h1>

	<!-- Filters -->
	<form
		onsubmit={handleSearch}
		class="rounded-xl border border-gray-200 bg-white p-4 dark:border-gray-700 dark:bg-gray-800"
	>
		<div class="flex flex-col gap-3 sm:flex-row sm:flex-wrap sm:items-end">
			<input
				type="text"
				bind:value={search}
				placeholder={t('home.searchPlaceholder')}
				class="min-w-0 flex-1 rounded-lg border border-gray-300 bg-white px-3 py-2 text-sm text-gray-900 dark:border-gray-600 dark:bg-gray-700 dark:text-white"
			/>
			<div class="flex gap-2">
				<input
					type="date"
					bind:value={dateFrom}
					class="rounded-lg border border-gray-300 bg-white px-3 py-2 text-sm text-gray-900 dark:border-gray-600 dark:bg-gray-700 dark:text-white"
				/>
				<input
					type="date"
					bind:value={dateTo}
					class="rounded-lg border border-gray-300 bg-white px-3 py-2 text-sm text-gray-900 dark:border-gray-600 dark:bg-gray-700 dark:text-white"
				/>
			</div>
			<div class="flex gap-2">
				<button
					type="submit"
					class="rounded-lg bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700"
					>{t('home.search')}</button
				>
				<button
					type="button"
					onclick={resetFilters}
					class="rounded-lg border border-gray-300 px-4 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50 dark:border-gray-600 dark:text-gray-300 dark:hover:bg-gray-700"
					>{t('images.reset')}</button
				>
			</div>
		</div>
	</form>

	{#if error}
		<p
			class="rounded-lg border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-700 dark:border-red-900 dark:bg-red-950/40 dark:text-red-300"
		>
			{error}
		</p>
	{/if}

	<!-- Total count -->
	<p class="text-sm text-gray-500 dark:text-gray-400">{t('home.total')}: {total}</p>

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
			{t('home.empty')}
		</div>
	{:else}
		<!-- Card grid -->
		<div class="grid grid-cols-1 gap-4 sm:grid-cols-2 xl:grid-cols-3">
			{#each items as item (item.id)}
				<article
					class="overflow-hidden rounded-xl border border-gray-200 bg-white dark:border-gray-700 dark:bg-gray-800"
				>
					<a
						href={resolve(`/images/${item.id}`)}
						class="block aspect-video bg-gray-100 dark:bg-gray-900"
					>
						{#if !item.is_deleted}
							<img src={item.view_url} alt={item.display_name} class="h-full w-full object-cover" />
						{:else}
							<div class="flex h-full items-center justify-center text-sm text-gray-400">
								{t('home.deletedBadge')}
							</div>
						{/if}
					</a>
					<div class="space-y-2 p-3">
						<div class="flex items-start justify-between gap-2">
							<div class="min-w-0">
								<h3 class="truncate text-sm font-medium text-gray-900 dark:text-white">
									{item.display_name}
								</h3>
								<p class="text-xs text-gray-500 dark:text-gray-400">
									#{item.id} · {item.width}×{item.height} · {formatSize(item.size)}
								</p>
							</div>
							{#if item.is_deleted}
								<span
									class="shrink-0 rounded-full bg-amber-100 px-2 py-0.5 text-xs font-medium text-amber-700 dark:bg-amber-950/50 dark:text-amber-200"
									>{t('home.deletedBadge')}</span
								>
							{/if}
						</div>
						<p class="text-xs text-gray-400">
							{t('images.created')}: {formatDate(item.created_at)}
						</p>
						<div class="flex flex-wrap gap-1.5">
							<button
								onclick={() => copyLink(item)}
								disabled={item.is_deleted}
								class="rounded px-2 py-1 text-xs font-medium text-blue-600 hover:bg-blue-50 disabled:opacity-40 dark:text-blue-400 dark:hover:bg-blue-900/20"
							>
								{copiedId === item.id ? t('images.copied') : t('images.copyLink')}
							</button>
							<button
								onclick={() => handleRename(item)}
								disabled={busyId === item.id}
								class="rounded px-2 py-1 text-xs font-medium text-gray-600 hover:bg-gray-100 disabled:opacity-40 dark:text-gray-400 dark:hover:bg-gray-700"
								>{t('home.rename')}</button
							>
							{#if item.is_deleted}
								<button
									onclick={() => handleRestore(item)}
									disabled={busyId === item.id}
									class="rounded px-2 py-1 text-xs font-medium text-green-600 hover:bg-green-50 disabled:opacity-40 dark:text-green-400 dark:hover:bg-green-900/20"
									>{t('home.restore')}</button
								>
							{:else}
								<button
									onclick={() => handleDelete(item)}
									disabled={busyId === item.id}
									class="rounded px-2 py-1 text-xs font-medium text-red-600 hover:bg-red-50 disabled:opacity-40 dark:text-red-400 dark:hover:bg-red-900/20"
									>{t('home.delete')}</button
								>
							{/if}
						</div>
					</div>
				</article>
			{/each}
		</div>

		<!-- Pagination -->
		<div class="flex flex-col items-center gap-3 sm:flex-row sm:justify-between">
			<p class="text-sm text-gray-500 dark:text-gray-400">
				{t('images.page')}
				{currentPage}
				{t('images.of')}
				{totalPages}
			</p>
			<div class="flex gap-2">
				<button
					onclick={() => goPage(currentPage - 1)}
					disabled={currentPage <= 1}
					class="rounded-lg border border-gray-300 px-3 py-1.5 text-sm font-medium text-gray-700 hover:bg-gray-50 disabled:opacity-40 dark:border-gray-600 dark:text-gray-300 dark:hover:bg-gray-700"
					>{t('images.prev')}</button
				>
				<button
					onclick={() => goPage(currentPage + 1)}
					disabled={currentPage >= totalPages}
					class="rounded-lg border border-gray-300 px-3 py-1.5 text-sm font-medium text-gray-700 hover:bg-gray-50 disabled:opacity-40 dark:border-gray-600 dark:text-gray-300 dark:hover:bg-gray-700"
					>{t('images.next')}</button
				>
			</div>
		</div>
	{/if}
</div>
