<script lang="ts">
	import { uploadImage } from '$lib/api';
	import { t } from '$lib/i18n/index.svelte';
	import { SvelteSet } from 'svelte/reactivity';
	import type { UploadResponse } from '$lib/sdk';

	const METADATA_FIELDS = ['camera', 'settings', 'time', 'copyright', 'location', 'others'] as const;
	type MetadataField = (typeof METADATA_FIELDS)[number];

	let files = $state<FileList | null>(null);
	let uploading = $state(false);
	let results = $state<Array<{ name: string; result?: UploadResponse; error?: string }>>([]);
	let dragover = $state(false);
	let fileInput: HTMLInputElement;
	let keepFields = new SvelteSet<MetadataField>();

	function toggleField(field: MetadataField) {
		if (keepFields.has(field)) keepFields.delete(field);
		else keepFields.add(field);
	}

	async function handleUpload() {
		if (!files || files.length === 0) return;
		uploading = true;
		results = [];
		const list = Array.from(files);
		const fields = keepFields.size > 0 ? [...keepFields] : undefined;
		for (const file of list) {
			try {
				const result = await uploadImage(file, fields);
				results = [...results, { name: file.name, result }];
			} catch {
				results = [...results, { name: file.name, error: t('home.uploadError') }];
			}
		}
		uploading = false;
		files = null;
		if (fileInput) fileInput.value = '';
	}

	function handleDrop(e: DragEvent) {
		e.preventDefault();
		dragover = false;
		if (e.dataTransfer?.files) {
			files = e.dataTransfer.files;
		}
	}

	function handleDragover(e: DragEvent) {
		e.preventDefault();
		dragover = true;
	}

	function copyUrl(url: string) {
		navigator.clipboard.writeText(url);
	}
</script>

<div class="space-y-6 max-w-2xl">
	<h1 class="text-2xl font-bold text-gray-900 dark:text-white">{t('upload.title')}</h1>

	<!-- Drop zone -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class="rounded-xl border-2 border-dashed p-8 text-center transition-colors
			{dragover
				? 'border-blue-500 bg-blue-50 dark:bg-blue-950/20'
				: 'border-gray-300 dark:border-gray-600 hover:border-blue-400 dark:hover:border-blue-500'}"
		ondrop={handleDrop}
		ondragover={handleDragover}
		ondragleave={() => (dragover = false)}
	>
		<div class="space-y-3">
			<div class="text-4xl text-gray-400">↑</div>
			<p class="text-sm font-medium text-gray-700 dark:text-gray-300">{t('upload.dropzone')}</p>
			<p class="text-xs text-gray-500 dark:text-gray-400">{t('upload.dropzoneHint')}</p>
			<input
				bind:this={fileInput}
				type="file"
				multiple
				accept="image/*"
				onchange={(e) => { files = (e.currentTarget as HTMLInputElement).files; }}
				class="hidden"
				id="file-input"
			/>
			<label
				for="file-input"
				class="inline-flex cursor-pointer items-center gap-2 rounded-lg border border-gray-300 dark:border-gray-600 px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700"
			>
				{t('upload.select')}
			</label>
		</div>
	</div>

	<!-- Metadata options -->
	<div class="rounded-xl border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 p-4 space-y-3">
		<div>
			<p class="text-sm font-medium text-gray-900 dark:text-white">{t('upload.metadata.label')}</p>
			<p class="text-xs text-gray-500 dark:text-gray-400 mt-0.5">{t('upload.metadata.hint')}</p>
		</div>
		<div class="flex flex-wrap gap-2">
			{#each METADATA_FIELDS as field (field)}
				<button
					type="button"
					onclick={() => toggleField(field)}
					class="rounded-full border px-3 py-1 text-xs font-medium transition-colors
						{keepFields.has(field)
							? 'border-blue-500 bg-blue-500 text-white'
							: 'border-gray-300 dark:border-gray-600 text-gray-600 dark:text-gray-400 hover:border-blue-400 dark:hover:border-blue-500'}"
				>
					{t(`upload.metadata.${field}`)}
				</button>
			{/each}
		</div>
	</div>

	<!-- Selected files -->
	{#if files && files.length > 0}
		<div class="rounded-xl border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 p-4 space-y-2">
			<p class="text-sm font-medium text-gray-900 dark:text-white">{t('upload.selected')}: {files.length}</p>
			<ul class="space-y-1">
				{#each Array.from(files) as file (file.name)}
					<li class="text-sm text-gray-600 dark:text-gray-400">· {file.name} ({(file.size / 1024).toFixed(1)} KB)</li>
				{/each}
			</ul>
			<button
				onclick={handleUpload}
				disabled={uploading}
				class="mt-2 rounded-lg bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700 disabled:opacity-60"
			>
				{uploading ? t('home.uploading') : t('upload.submit')}
			</button>
		</div>
	{/if}

	<!-- Results -->
	{#if results.length > 0}
		<div class="space-y-3">
			<h2 class="text-base font-semibold text-gray-900 dark:text-white">{t('upload.results')}</h2>
			{#each results as r (r.name)}
				{#if r.error}
					<div class="rounded-xl border border-red-200 dark:border-red-900 bg-red-50 dark:bg-red-950/40 p-4">
						<p class="text-sm font-medium text-red-700 dark:text-red-300">{r.name}</p>
						<p class="text-sm text-red-600 dark:text-red-400">{r.error}</p>
					</div>
				{:else if r.result}
					<div class="rounded-xl border border-green-200 dark:border-green-900 bg-green-50 dark:bg-green-950/40 p-4 space-y-2">
						<p class="text-sm font-medium text-green-800 dark:text-green-200">{r.name} — {t('upload.success')}</p>
						<div class="flex items-center gap-2">
							<code class="flex-1 min-w-0 truncate rounded bg-green-100 dark:bg-green-900/40 px-2 py-1 text-xs text-green-900 dark:text-green-200">{r.result.url}</code>
							<button onclick={() => copyUrl(r.result!.url)} class="shrink-0 rounded px-2 py-1 text-xs font-medium text-green-700 dark:text-green-300 hover:bg-green-100 dark:hover:bg-green-900/40">{t('images.copyLink')}</button>
						</div>
					</div>
				{/if}
			{/each}
		</div>
	{/if}
</div>
