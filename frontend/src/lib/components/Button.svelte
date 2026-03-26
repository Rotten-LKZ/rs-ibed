<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLButtonAttributes } from 'svelte/elements';

	interface Props extends HTMLButtonAttributes {
		variant?: 'primary' | 'secondary';
		children: Snippet;
	}

	let { variant = 'primary', class: className = '', children, ...rest }: Props & { class?: string } = $props();
</script>

<button
	class="cursor-pointer rounded-md px-4 py-2 text-sm font-medium transition-colors disabled:cursor-not-allowed disabled:opacity-50
		{variant === 'primary'
		? 'bg-blue-600 text-white hover:bg-blue-700 dark:bg-blue-500 dark:hover:bg-blue-600'
		: 'bg-gray-200 text-gray-800 hover:bg-gray-300 dark:bg-gray-700 dark:text-gray-200 dark:hover:bg-gray-600'}
		{className}"
	{...rest}
>
	{@render children()}
</button>
