<script lang="ts">
	import InventoryList from '$lib/InventoryList.svelte';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();
</script>

<InventoryList items={data.inventory} />
