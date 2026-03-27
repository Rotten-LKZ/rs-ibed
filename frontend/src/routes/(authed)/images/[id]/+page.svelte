<script lang="ts">
	import { resolve } from '$app/paths';
	import { page } from '$app/stores';
	import {
		fetchImageDetail,
		deleteManagedImage,
		renameManagedImage,
		restoreManagedImage
	} from '$lib/api';
	import { t } from '$lib/i18n/index.svelte';
	import type { ImageModel } from '$lib/sdk';

	let image = $state<ImageModel | null>(null);
	let viewUrl = $state('');
	let loading = $state(true);
	let error = $state('');
	let busy = $state(false);

	$effect(() => {
		const id = Number($page.params.id);
		if (!isNaN(id)) loadDetail(id);
	});

	async function loadDetail(id: number) {
		loading = true;
		error = '';
		try {
			const result = await fetchImageDetail(id);
			image = result.image;
			viewUrl = result.view_url;
		} catch {
			error = t('detail.notFound');
		} finally {
			loading = false;
		}
	}

	async function handleRename() {
		if (!image) return;
		const name = prompt(t('home.renamePrompt'), image.display_name);
		if (!name || name === image.display_name) return;
		busy = true;
		try {
			await renameManagedImage(image.id, name);
			await loadDetail(image.id);
		} catch {
			error = t('home.actionError');
		} finally {
			busy = false;
		}
	}

	async function handleDelete() {
		if (!image) return;
		busy = true;
		try {
			await deleteManagedImage(image.id);
			await loadDetail(image.id);
		} catch {
			error = t('home.actionError');
		} finally {
			busy = false;
		}
	}

	async function handleRestore() {
		if (!image) return;
		busy = true;
		try {
			await restoreManagedImage(image.id);
			await loadDetail(image.id);
		} catch {
			error = t('home.actionError');
		} finally {
			busy = false;
		}
	}

	function formatSize(bytes: number): string {
		if (bytes < 1024) return bytes + ' B';
		if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB';
		return (bytes / (1024 * 1024)).toFixed(1) + ' MB';
	}
</script>

<div class="space-y-4">
	<a
		href={resolve('/images')}
		class="inline-flex items-center gap-1 text-sm text-blue-600 hover:underline dark:text-blue-400"
	>
		&larr; {t('detail.back')}
	</a>

	{#if loading}
		<div
			class="rounded-xl border border-gray-200 bg-white p-6 text-sm text-gray-500 dark:border-gray-700 dark:bg-gray-800"
		>
			{t('home.loading')}
		</div>
	{:else if error || !image}
		<div
			class="rounded-xl border border-red-200 bg-red-50 p-6 text-sm text-red-700 dark:border-red-900 dark:bg-red-950/40 dark:text-red-300"
		>
			{error || t('detail.notFound')}
		</div>
	{:else}
		<div class="grid gap-6 lg:grid-cols-2">
			<!-- Preview -->
			<div
				class="overflow-hidden rounded-xl border border-gray-200 bg-gray-100 dark:border-gray-700 dark:bg-gray-900"
			>
				{#if !image.is_deleted}
					<img src={viewUrl} alt={image.display_name} class="max-h-[70vh] w-full object-contain" />
				{:else}
					<div class="flex h-64 items-center justify-center text-gray-400">
						{t('home.deletedBadge')}
					</div>
				{/if}
			</div>

			<!-- Metadata -->
			<div class="space-y-4">
				<div class="flex items-center gap-3">
					<h1 class="text-xl font-bold text-gray-900 dark:text-white">{image.display_name}</h1>
					{#if image.is_deleted}
						<span
							class="rounded-full bg-red-100 px-2 py-0.5 text-xs font-medium text-red-700 dark:bg-red-950/50 dark:text-red-300"
							>{t('detail.deleted')}</span
						>
					{:else}
						<span
							class="rounded-full bg-green-100 px-2 py-0.5 text-xs font-medium text-green-700 dark:bg-green-950/50 dark:text-green-300"
							>{t('detail.active')}</span
						>
					{/if}
				</div>

				<dl
					class="divide-y divide-gray-200 rounded-xl border border-gray-200 bg-white dark:divide-gray-700 dark:border-gray-700 dark:bg-gray-800"
				>
					<div class="flex justify-between px-4 py-3">
						<dt class="text-sm text-gray-500 dark:text-gray-400">{t('detail.fileName')}</dt>
						<dd class="text-sm font-medium text-gray-900 dark:text-white">{image.file_name}</dd>
					</div>
					<div class="flex justify-between px-4 py-3">
						<dt class="text-sm text-gray-500 dark:text-gray-400">{t('detail.dimensions')}</dt>
						<dd class="text-sm font-medium text-gray-900 dark:text-white">
							{image.width} × {image.height}
						</dd>
					</div>
					<div class="flex justify-between px-4 py-3">
						<dt class="text-sm text-gray-500 dark:text-gray-400">{t('home.mime')}</dt>
						<dd class="text-sm font-medium text-gray-900 dark:text-white">{image.mime_type}</dd>
					</div>
					<div class="flex justify-between px-4 py-3">
						<dt class="text-sm text-gray-500 dark:text-gray-400">{t('home.size')}</dt>
						<dd class="text-sm font-medium text-gray-900 dark:text-white">
							{formatSize(image.size)}
						</dd>
					</div>
					<div class="flex justify-between px-4 py-3">
						<dt class="text-sm text-gray-500 dark:text-gray-400">{t('detail.hash')}</dt>
						<dd class="max-w-[200px] truncate font-mono text-sm text-gray-900 dark:text-white">
							{image.hash}
						</dd>
					</div>
					<div class="flex justify-between px-4 py-3">
						<dt class="text-sm text-gray-500 dark:text-gray-400">{t('detail.created')}</dt>
						<dd class="text-sm font-medium text-gray-900 dark:text-white">
							{new Date(image.created_at).toLocaleString()}
						</dd>
					</div>
					<div class="flex justify-between px-4 py-3">
						<dt class="text-sm text-gray-500 dark:text-gray-400">{t('detail.updated')}</dt>
						<dd class="text-sm font-medium text-gray-900 dark:text-white">
							{new Date(image.updated_at).toLocaleString()}
						</dd>
					</div>
				</dl>

				{#if !image.is_deleted}
					<div
						class="space-y-2 rounded-xl border border-gray-200 bg-white p-4 dark:border-gray-700 dark:bg-gray-800"
					>
						<div>
							<p class="text-xs text-gray-500 dark:text-gray-400">{t('detail.viewUrl')}</p>
							<p class="font-mono text-sm break-all text-blue-600 dark:text-blue-400">{viewUrl}</p>
						</div>
						<div>
							<p class="text-xs text-gray-500 dark:text-gray-400">{t('detail.downloadUrl')}</p>
							<p class="font-mono text-sm break-all text-blue-600 dark:text-blue-400">
								{viewUrl.replace('/v/', '/d/')}
							</p>
						</div>
					</div>
				{/if}

				<div class="flex flex-wrap gap-2">
					<button
						onclick={handleRename}
						disabled={busy}
						class="rounded-lg border border-gray-300 px-4 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50 disabled:opacity-40 dark:border-gray-600 dark:text-gray-300 dark:hover:bg-gray-700"
						>{t('home.rename')}</button
					>
					{#if image.is_deleted}
						<button
							onclick={handleRestore}
							disabled={busy}
							class="rounded-lg bg-green-600 px-4 py-2 text-sm font-medium text-white hover:bg-green-700 disabled:opacity-40"
							>{t('home.restore')}</button
						>
					{:else}
						<button
							onclick={handleDelete}
							disabled={busy}
							class="rounded-lg bg-red-600 px-4 py-2 text-sm font-medium text-white hover:bg-red-700 disabled:opacity-40"
							>{t('home.delete')}</button
						>
					{/if}
				</div>
			</div>
		</div>
	{/if}
</div>
