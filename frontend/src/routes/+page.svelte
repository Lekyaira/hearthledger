<script lang="ts">
	import EditableInventoryList from '$lib/EditableInventoryList.svelte';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();
</script>

<EditableInventoryList items={data.inventory} />
