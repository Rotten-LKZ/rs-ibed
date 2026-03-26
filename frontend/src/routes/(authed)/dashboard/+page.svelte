<script lang="ts">
	import { resolve } from '$app/paths';
	import { fetchImages } from '$lib/api';
	import { t } from '$lib/i18n/index.svelte';
	import { onMount } from 'svelte';
	import type { ImageModel } from '$lib/sdk';

	let totalImages = $state(0);
	let totalDeleted = $state(0);
	let recentItems = $state<ImageModel[]>([]);
	let loading = $state(true);

	onMount(async () => {
		try {
			const result = await fetchImages({ perPage: 100 });
			totalImages = result.total;
			totalDeleted = result.items.filter((i) => i.is_deleted).length;
			recentItems = result.items.filter((i) => !i.is_deleted).slice(0, 6);
		} finally {
			loading = false;
		}
	});
</script>

<div class="space-y-6">
	<h1 class="text-2xl font-bold text-gray-900 dark:text-white">{t('dashboard.title')}</h1>

	<!-- Stats cards -->
	<div class="grid grid-cols-1 gap-4 sm:grid-cols-3">
		<div class="rounded-xl border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 p-5">
			<p class="text-sm font-medium text-gray-500 dark:text-gray-400">{t('dashboard.totalImages')}</p>
			{#if loading}
				<div class="mt-2 h-8 w-16 animate-pulse rounded bg-gray-200 dark:bg-gray-700"></div>
			{:else}
				<p class="mt-2 text-3xl font-bold text-gray-900 dark:text-white">{totalImages}</p>
			{/if}
		</div>
		<div class="rounded-xl border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 p-5">
			<p class="text-sm font-medium text-gray-500 dark:text-gray-400">{t('dashboard.activeImages')}</p>
			{#if loading}
				<div class="mt-2 h-8 w-16 animate-pulse rounded bg-gray-200 dark:bg-gray-700"></div>
			{:else}
				<p class="mt-2 text-3xl font-bold text-green-600 dark:text-green-400">{totalImages - totalDeleted}</p>
			{/if}
		</div>
		<div class="rounded-xl border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 p-5">
			<p class="text-sm font-medium text-gray-500 dark:text-gray-400">{t('dashboard.deletedImages')}</p>
			{#if loading}
				<div class="mt-2 h-8 w-16 animate-pulse rounded bg-gray-200 dark:bg-gray-700"></div>
			{:else}
				<p class="mt-2 text-3xl font-bold text-red-500">{totalDeleted}</p>
			{/if}
		</div>
	</div>

	<!-- Recent uploads -->
	<div class="space-y-3">
		<div class="flex items-center justify-between">
			<h2 class="text-lg font-semibold text-gray-900 dark:text-white">{t('dashboard.recentUploads')}</h2>
			<a href={resolve('/images')} class="text-sm text-blue-600 dark:text-blue-400 hover:underline">{t('dashboard.viewAll')}</a>
		</div>
		{#if loading}
			<div class="rounded-xl border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 p-6 text-sm text-gray-500 dark:text-gray-400">
				{t('home.loading')}
			</div>
		{:else if recentItems.length === 0}
			<div class="rounded-xl border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 p-6 text-sm text-gray-500 dark:text-gray-400">
				{t('dashboard.noRecent')}
			</div>
		{:else}
			<div class="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-6">
				{#each recentItems as item (item.id)}
					<a href={resolve(`/images/${item.id}`)} class="group overflow-hidden rounded-xl border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800">
						<div class="aspect-square bg-gray-100 dark:bg-gray-900">
							<img
								src={`/v/${item.hash}.${item.extension}`}
								alt={item.display_name}
								class="h-full w-full object-cover transition-transform group-hover:scale-105"
							/>
						</div>
						<div class="p-2">
							<p class="truncate text-xs font-medium text-gray-700 dark:text-gray-300">{item.display_name}</p>
						</div>
					</a>
				{/each}
			</div>
		{/if}
	</div>

	<!-- Quick actions -->
	<div class="flex flex-wrap gap-3">
		<a href={resolve('/images')} class="inline-flex items-center gap-2 rounded-lg bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700 transition-colors">{t('nav.images')}</a>
		<a href={resolve('/upload')} class="inline-flex items-center gap-2 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors">{t('nav.upload')}</a>
		<a href={resolve('/trash')} class="inline-flex items-center gap-2 rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-800 px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors">{t('nav.trash')}</a>
	</div>
</div>
