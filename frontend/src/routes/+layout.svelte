<script lang="ts">
	import './layout.css';
	import favicon from '$lib/assets/favicon.svg';
	import PendingBundlesIcon from '$lib/PendingBundlesIcon.svelte';

	let { children } = $props();
</script>

<svelte:head><link rel="icon" href={favicon} /></svelte:head>
<header class="mx-auto flex w-full max-w-4xl justify-end px-6 pt-4">
	<PendingBundlesIcon />
</header>
{@render children()}
