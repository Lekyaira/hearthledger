<script lang="ts">
	import { quantityTypeLabels, type InventoryItem } from '$lib/inventory';

	type Props = {
		items: InventoryItem[];
	};

	let { items }: Props = $props();

	const numberFormatter = new Intl.NumberFormat(undefined, {
		maximumFractionDigits: 2
	});

	function formatQuantity(item: InventoryItem) {
		return `${numberFormatter.format(item.quantity)} ${quantityTypeLabels[item.quantity_type]}`;
	}
</script>

<section class="mx-auto flex w-full max-w-3xl flex-col gap-4 p-6">
	<h1 class="text-2xl font-semibold text-zinc-950">Inventory</h1>

	{#if items.length > 0}
		<ul aria-label="Inventory items" class="divide-y divide-zinc-200 border-y border-zinc-200">
			{#each items as inventoryItem (inventoryItem.item)}
				<li class="flex items-center justify-between gap-4 py-3">
					<span class="min-w-0 truncate font-medium text-zinc-900">{inventoryItem.item}</span>
					<span class="shrink-0 text-zinc-700 tabular-nums">{formatQuantity(inventoryItem)}</span>
				</li>
			{/each}
		</ul>
	{:else}
		<p class="border-y border-zinc-200 py-4 text-zinc-600">No inventory items yet.</p>
	{/if}
</section>
