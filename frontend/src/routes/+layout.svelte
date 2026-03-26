<script lang="ts">
	import './layout.css';
	import favicon from '$lib/assets/favicon.svg';
	import { initTheme } from '$lib/stores/theme.svelte';
	import { initLocale } from '$lib/i18n/index.svelte';

	let { children } = $props();

	$effect(() => {
		initTheme();
		initLocale();
	});
</script>

<svelte:head><link rel="icon" href={favicon} /></svelte:head>
{@render children()}
