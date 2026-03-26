<script lang="ts">
	import { page } from '$app/stores';
	import { resolve } from '$app/paths';
	import { t } from '$lib/i18n/index.svelte';

	let { onclose = () => {} }: { onclose?: () => void } = $props();

	const links = [
		{ href: '/dashboard', label: () => t('nav.dashboard'), icon: '▦' },
		{ href: '/images', label: () => t('nav.images'), icon: '🖼' },
		{ href: '/upload', label: () => t('nav.upload'), icon: '↑' },
		{ href: '/trash', label: () => t('nav.trash'), icon: '🗑' },
	];
</script>

<nav class="flex flex-col h-full bg-white dark:bg-gray-900 border-r border-gray-200 dark:border-gray-700 w-64">
	<div class="flex items-center gap-2 px-6 py-5 border-b border-gray-200 dark:border-gray-700">
		<span class="font-bold text-lg text-gray-900 dark:text-white">rs-ibed</span>
	</div>
	<ul class="flex-1 py-4 space-y-1 px-3">
		{#each links as link (link.href)}
			{@const active = $page.url.pathname === link.href || $page.url.pathname.startsWith(link.href + '/')}
			<li>
				<a
					href={resolve(link.href)}
					onclick={onclose}
					class="flex items-center gap-3 px-3 py-2 rounded-lg text-sm font-medium transition-colors
						{active
							? 'bg-blue-50 dark:bg-blue-900/30 text-blue-700 dark:text-blue-300'
							: 'text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800'}"
				>
					<span class="w-5 text-center text-base">{link.icon}</span>
					{link.label()}
				</a>
			</li>
		{/each}
	</ul>
</nav>
