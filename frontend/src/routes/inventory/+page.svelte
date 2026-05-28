<script lang="ts">
	import { onMount } from 'svelte';
	import EditableInventoryList from '$lib/EditableInventoryList.svelte';
	import InventoryList from '$lib/InventoryList.svelte';
	import { readCurrentUserId, userChangedEvent } from '$lib/auth';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();
	let userId = $state('');
	let hasMounted = $state(false);
	const currentUser = $derived(data.users.find((user) => user.id === userId));

	function refreshUserId() {
		userId = readCurrentUserId();
	}

	onMount(() => {
		hasMounted = true;
		refreshUserId();
		window.addEventListener(userChangedEvent, refreshUserId);
		window.addEventListener('storage', refreshUserId);

		return () => {
			window.removeEventListener(userChangedEvent, refreshUserId);
			window.removeEventListener('storage', refreshUserId);
		};
	});
</script>

{#if hasMounted && currentUser?.role === 'admin'}
	<EditableInventoryList items={data.inventory} />
{:else if hasMounted && currentUser?.role === 'member'}
	<InventoryList items={data.inventory} />
{:else}
	<section class="mx-auto flex w-full max-w-3xl flex-col gap-4 p-6">
		<h1 class="text-2xl font-semibold text-zinc-950">Inventory</h1>
		<p class="border-y border-zinc-200 py-4 text-zinc-600">Select a user to view inventory.</p>
	</section>
{/if}
