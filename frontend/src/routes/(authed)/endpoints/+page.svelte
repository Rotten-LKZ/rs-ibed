<script lang="ts">
	import { listStorageEndpoints, updateStorageEndpoint } from '$lib/sdk';
	import { t } from '$lib/i18n/index.svelte';
	import { onMount } from 'svelte';
	import type { StorageEndpointResponse } from '$lib/sdk';

	let endpoints = $state<StorageEndpointResponse[]>([]);
	let loading = $state(true);
	let error = $state('');
	let busyName = $state<string | null>(null);

	async function load() {
		loading = true;
		error = '';
		try {
			const res = await listStorageEndpoints({ throwOnError: true });
			endpoints = res.data ?? [];
		} catch {
			error = t('endpoints.actionError');
		} finally {
			loading = false;
		}
	}

	onMount(load);

	async function toggleStatus(ep: StorageEndpointResponse) {
		busyName = ep.name;
		const newStatus = ep.status === 'active' ? 'disabled' : 'active';
		try {
			await updateStorageEndpoint({
				throwOnError: true,
				path: { name: ep.name },
				body: { status: newStatus }
			});
			await load();
		} catch {
			error = t('endpoints.actionError');
		} finally {
			busyName = null;
		}
	}

	async function editDescription(ep: StorageEndpointResponse) {
		const desc = prompt(t('endpoints.descriptionPrompt'), ep.description);
		if (desc === null || desc === ep.description) return;
		busyName = ep.name;
		try {
			await updateStorageEndpoint({
				throwOnError: true,
				path: { name: ep.name },
				body: { description: desc }
			});
			await load();
		} catch {
			error = t('endpoints.actionError');
		} finally {
			busyName = null;
		}
	}

	function formatBytes(bytes: number): string {
		if (bytes <= 0) return '0 B';
		const units = ['B', 'KB', 'MB', 'GB', 'TB'];
		const i = Math.floor(Math.log(bytes) / Math.log(1024));
		return (bytes / Math.pow(1024, i)).toFixed(1) + ' ' + units[i];
	}

	function usagePct(used: number, capacity: number): number {
		if (capacity <= 0) return 0;
		return Math.min(100, Math.round((used / capacity) * 100));
	}

	function usageColor(pct: number): string {
		if (pct >= 90) return 'bg-red-500';
		if (pct >= 70) return 'bg-amber-500';
		return 'bg-blue-500';
	}
</script>

<div class="space-y-4">
	<div class="flex items-center justify-between">
		<h1 class="text-2xl font-bold text-gray-900 dark:text-white">{t('endpoints.title')}</h1>
		<button
			onclick={load}
			class="rounded-lg border border-gray-300 px-3 py-1.5 text-sm font-medium text-gray-700 hover:bg-gray-50 dark:border-gray-600 dark:text-gray-300 dark:hover:bg-gray-700"
		>
			{t('home.refresh')}
		</button>
	</div>

	<p class="text-sm text-gray-500 dark:text-gray-400">{t('endpoints.immutableHint')}</p>

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
			{t('endpoints.loading')}
		</div>
	{:else if endpoints.length === 0}
		<div
			class="rounded-xl border border-gray-200 bg-white p-6 text-sm text-gray-500 dark:border-gray-700 dark:bg-gray-800"
		>
			{t('endpoints.empty')}
		</div>
	{:else}
		<div class="space-y-3">
			{#each endpoints as ep (ep.name)}
				{@const pct = usagePct(ep.used_size, ep.capacity_bytes)}
				{@const free = ep.capacity_bytes - ep.used_size}
				<div
					class="rounded-xl border border-gray-200 bg-white p-4 dark:border-gray-700 dark:bg-gray-800"
				>
					<!-- Header row -->
					<div class="flex flex-wrap items-start justify-between gap-2">
						<div class="min-w-0">
							<div class="flex items-center gap-2">
								<span class="font-mono text-sm font-semibold text-gray-900 dark:text-white">
									{ep.name}
								</span>
								<!-- Type badge -->
								<span
									class="rounded-full px-2 py-0.5 text-xs font-medium
									{ep.endpoint_type === 'Local'
										? 'bg-purple-100 text-purple-700 dark:bg-purple-950/50 dark:text-purple-300'
										: 'bg-sky-100 text-sky-700 dark:bg-sky-950/50 dark:text-sky-300'}"
								>
									{ep.endpoint_type}
								</span>
								<!-- Direct mode badge -->
								{#if ep.direct_mode !== 'proxy'}
									<span
										class="rounded-full px-2 py-0.5 text-xs font-medium
									{ep.direct_mode === 'presigned'
											? 'bg-amber-100 text-amber-700 dark:bg-amber-950/50 dark:text-amber-300'
											: 'bg-teal-100 text-teal-700 dark:bg-teal-950/50 dark:text-teal-300'}"
									>
										{ep.direct_mode}
									</span>
								{/if}
								<!-- Status badge -->
								<span
									class="rounded-full px-2 py-0.5 text-xs font-medium
									{ep.status === 'active'
										? 'bg-green-100 text-green-700 dark:bg-green-950/50 dark:text-green-300'
										: 'bg-gray-100 text-gray-600 dark:bg-gray-700 dark:text-gray-400'}"
								>
									{ep.status === 'active' ? t('endpoints.active') : t('endpoints.disabled')}
								</span>
							</div>
							{#if ep.description}
								<p class="mt-0.5 text-xs text-gray-500 dark:text-gray-400">{ep.description}</p>
							{/if}
						</div>

						<!-- Actions -->
						<div class="flex shrink-0 gap-2">
							<button
								onclick={() => editDescription(ep)}
								disabled={busyName === ep.name}
								class="rounded px-2 py-1 text-xs font-medium text-gray-600 hover:bg-gray-100 disabled:opacity-40 dark:text-gray-400 dark:hover:bg-gray-700"
							>
								{t('endpoints.editDescription')}
							</button>
							<button
								onclick={() => toggleStatus(ep)}
								disabled={busyName === ep.name}
								class="rounded px-2 py-1 text-xs font-medium disabled:opacity-40
								{ep.status === 'active'
									? 'text-amber-600 hover:bg-amber-50 dark:text-amber-400 dark:hover:bg-amber-900/20'
									: 'text-green-600 hover:bg-green-50 dark:text-green-400 dark:hover:bg-green-900/20'}"
							>
								{ep.status === 'active' ? t('endpoints.disable') : t('endpoints.enable')}
							</button>
						</div>
					</div>

					<!-- Stats row -->
					<dl class="mt-3 grid grid-cols-2 gap-x-4 gap-y-1 text-xs sm:grid-cols-4">
						<div>
							<dt class="text-gray-500 dark:text-gray-400">{t('endpoints.priority')}</dt>
							<dd class="font-medium text-gray-900 dark:text-white">{ep.priority}</dd>
						</div>
						<div>
							<dt class="text-gray-500 dark:text-gray-400">{t('endpoints.capacity')}</dt>
							<dd class="font-medium text-gray-900 dark:text-white">
								{formatBytes(ep.capacity_bytes)}
							</dd>
						</div>
						<div>
							<dt class="text-gray-500 dark:text-gray-400">{t('endpoints.used')}</dt>
							<dd class="font-medium text-gray-900 dark:text-white">{formatBytes(ep.used_size)}</dd>
						</div>
						<div>
							<dt class="text-gray-500 dark:text-gray-400">{t('endpoints.free')}</dt>
							<dd class="font-medium text-gray-900 dark:text-white">{formatBytes(free)}</dd>
						</div>
					</dl>

					<!-- Usage bar -->
					<div class="mt-3">
						<div class="mb-1 flex justify-between text-xs text-gray-500 dark:text-gray-400">
							<span>{t('endpoints.usage')}</span>
							<span>{pct}%</span>
						</div>
						<div class="h-2 w-full overflow-hidden rounded-full bg-gray-200 dark:bg-gray-700">
							<div
								class="h-full rounded-full transition-all {usageColor(pct)}"
								style="width: {pct}%"
							></div>
						</div>
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>
