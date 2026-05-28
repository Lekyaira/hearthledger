<script lang="ts">
	import { onMount } from 'svelte';
	import {
		bundlesChangedEvent,
		readCurrentUserId,
		requestStorageKey,
		type NewBundle
	} from '$lib/bundles';
	import { quantityTypeLabels, type InventoryItem } from '$lib/inventory';

	type Props = {
		items: InventoryItem[];
	};

	let { items }: Props = $props();
	let quantities = $state<Record<string, string>>({});
	let requestState = $state<'idle' | 'saving' | 'error'>('idle');
	let errorMessage = $state('');
	let hasMounted = false;

	const numberFormatter = new Intl.NumberFormat(undefined, {
		maximumFractionDigits: 2
	});

	const requestRows = $derived(
		items
			.map((item) => ({
				item,
				quantity: Number(quantities[itemKey(item)] ?? 0)
			}))
			.filter((row) => Number.isFinite(row.quantity) && row.quantity > 0)
	);
	const hasPendingRequest = $derived(requestRows.length > 0);

	$effect(() => {
		if (!hasMounted) return;

		if (!hasPendingRequest) {
			sessionStorage.removeItem(requestStorageKey);
			return;
		}

		sessionStorage.setItem(requestStorageKey, JSON.stringify(quantities));
	});

	onMount(() => {
		hasMounted = true;
		const storedValue = sessionStorage.getItem(requestStorageKey);
		if (!storedValue) return;

		try {
			const storedQuantities = JSON.parse(storedValue) as Record<string, string>;
			if (storedQuantities && typeof storedQuantities === 'object') {
				quantities = storedQuantities;
			}
		} catch {
			sessionStorage.removeItem(requestStorageKey);
		}
	});

	function itemKey(item: InventoryItem) {
		return String(item.id ?? item.item);
	}

	function formatQuantity(item: InventoryItem) {
		return `${numberFormatter.format(item.quantity)} ${quantityTypeLabels[item.quantity_type]}`;
	}

	function updateQuantity(item: InventoryItem, quantity: string) {
		quantities = {
			...quantities,
			[itemKey(item)]: quantity
		};
	}

	function cancelRequest() {
		quantities = {};
		sessionStorage.removeItem(requestStorageKey);
		requestState = 'idle';
		errorMessage = '';
	}

	function validateRequest() {
		if (requestRows.length === 0) {
			return 'Enter at least one item quantity to request.';
		}

		if (requestRows.some(({ item }) => item.id === undefined)) {
			return 'Inventory item IDs are required to create a bundle.';
		}

		if (requestRows.some(({ item, quantity }) => quantity > item.quantity)) {
			return 'Requested quantities cannot exceed available inventory.';
		}

		return '';
	}

	async function createRequest() {
		const validationMessage = validateRequest();
		if (validationMessage) {
			requestState = 'error';
			errorMessage = validationMessage;
			return;
		}

		requestState = 'saving';
		errorMessage = '';

		const bundle: NewBundle = {
			user: readCurrentUserId(),
			bundled: false,
			items: requestRows.map(({ item, quantity }) => ({
				item_id: item.id as number,
				quantity
			}))
		};

		try {
			const response = await fetch('/api/bundle', {
				method: 'POST',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify(bundle)
			});

			if (!response.ok) {
				throw new Error(`Bundle request failed with status ${response.status}.`);
			}

			cancelRequest();
			window.dispatchEvent(new CustomEvent(bundlesChangedEvent));
		} catch (error) {
			requestState = 'error';
			errorMessage = error instanceof Error ? error.message : 'Bundle request failed.';
		}
	}
</script>

<section class="mx-auto flex w-full max-w-3xl flex-col gap-4 p-6">
	<div class="flex flex-wrap items-center justify-between gap-3">
		<h1 class="text-2xl font-semibold text-zinc-950">Inventory</h1>
		<div class="flex items-center gap-2">
			<button
				type="button"
				class="rounded border border-zinc-300 px-3 py-2 text-sm font-medium text-zinc-800 disabled:cursor-not-allowed disabled:opacity-50"
				disabled={!hasPendingRequest || requestState === 'saving'}
				onclick={cancelRequest}
			>
				cancel
			</button>
			<button
				type="button"
				class="rounded bg-zinc-950 px-3 py-2 text-sm font-medium text-white disabled:cursor-not-allowed disabled:opacity-50"
				disabled={!hasPendingRequest || requestState === 'saving'}
				onclick={createRequest}
			>
				{requestState === 'saving' ? 'requesting...' : 'request'}
			</button>
		</div>
	</div>

	{#if requestState === 'error'}
		<p class="rounded border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-700">
			{errorMessage}
		</p>
	{/if}

	{#if items.length > 0}
		<ul aria-label="Inventory items" class="divide-y divide-zinc-200 border-y border-zinc-200">
			{#each items as inventoryItem (inventoryItem.item)}
				<li class="grid gap-3 py-3 sm:grid-cols-[1fr_auto_7rem] sm:items-center">
					<span class="min-w-0 truncate font-medium text-zinc-900">{inventoryItem.item}</span>
					<span class="shrink-0 text-zinc-700 tabular-nums">{formatQuantity(inventoryItem)}</span>
					<label aria-label={`Request quantity for ${inventoryItem.item}`}>
						<input
							type="number"
							min="0"
							max={inventoryItem.quantity}
							step="any"
							class="w-full rounded border-zinc-300 text-sm text-zinc-900 tabular-nums"
							value={quantities[itemKey(inventoryItem)] ?? ''}
							placeholder="0"
							oninput={(event) => updateQuantity(inventoryItem, event.currentTarget.value)}
						/>
					</label>
				</li>
			{/each}
		</ul>
	{:else}
		<p class="border-y border-zinc-200 py-4 text-zinc-600">No inventory items yet.</p>
	{/if}
</section>
