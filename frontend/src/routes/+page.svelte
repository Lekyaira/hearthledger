<script lang="ts">
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { readCurrentUserId } from '$lib/auth';

	onMount(() => {
		const userId = readCurrentUserId();
		void goto(userId ? '/inventory' : '/login', { replaceState: true });
	});
</script>

<section class="mx-auto flex w-full max-w-3xl flex-col gap-4 p-6">
	<h1 class="text-2xl font-semibold text-zinc-950">Hearthledger</h1>
</section>
