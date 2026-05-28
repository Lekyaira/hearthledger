<script lang="ts">
	import { onMount } from 'svelte';
	import {
		quantityTypeLabels,
		quantityTypes,
		type InventoryItem,
		type QuantityType
	} from '$lib/inventory';

	type Props = {
		items: InventoryItem[];
		apiPath?: string;
	};

	type InventoryRow = {
		id: string;
		original: InventoryItem | null;
		item: string;
		quantity: string;
		quantity_type: QuantityType;
		deleted: boolean;
	};

	type StoredInventoryRow = Omit<InventoryRow, 'original'> & {
		original: InventoryItem | null;
	};

	let { items, apiPath = '/api/inventory' }: Props = $props();
	let committedItems = $derived(items);
	let rows = $derived(createRows(committedItems));
	let saveState = $state<'idle' | 'saving' | 'error'>('idle');
	let errorMessage = $state('');
	let nextRowId = $state(0);
	let hasMounted = false;

	const pendingStorageKey = 'hearthledger.inventory.pending.v1';

	const hasPendingChanges = $derived(rows.some((row) => hasPendingChange(row)));

	$effect(() => {
		if (!hasMounted) return;

		const pendingRows = rows.filter((row) => hasPendingChange(row));

		if (pendingRows.length === 0) {
			sessionStorage.removeItem(pendingStorageKey);
			return;
		}

		sessionStorage.setItem(pendingStorageKey, JSON.stringify(rows satisfies StoredInventoryRow[]));
	});

	onMount(() => {
		hasMounted = true;

		const storedRows = readStoredRows();
		if (storedRows) {
			rows = mergeStoredRows(storedRows, committedItems);
		}
	});

	function createRows(sourceItems: InventoryItem[]): InventoryRow[] {
		return sourceItems.map((inventoryItem) => ({
			id: `existing:${inventoryItem.item}`,
			original: inventoryItem,
			item: inventoryItem.item,
			quantity: String(inventoryItem.quantity),
			quantity_type: inventoryItem.quantity_type,
			deleted: false
		}));
	}

	function readStoredRows() {
		const storedValue = sessionStorage.getItem(pendingStorageKey);
		if (!storedValue) return null;

		try {
			const storedRows = JSON.parse(storedValue) as StoredInventoryRow[];
			if (!Array.isArray(storedRows)) return null;
			return storedRows;
		} catch {
			return null;
		}
	}

	function mergeStoredRows(
		storedRows: StoredInventoryRow[],
		currentItems: InventoryItem[]
	): InventoryRow[] {
		const currentByName = new Map(
			currentItems.map((inventoryItem) => [inventoryItem.item, inventoryItem])
		);

		return storedRows.map((storedRow) => {
			const original = storedRow.original
				? (currentByName.get(storedRow.original.item) ?? null)
				: null;

			return {
				...storedRow,
				original
			};
		});
	}

	function addRow() {
		rows = [
			...rows,
			{
				id: `new:${nextRowId++}`,
				original: null,
				item: '',
				quantity: '0',
				quantity_type: 'count',
				deleted: false
			}
		];
	}

	function toggleDeleted(row: InventoryRow) {
		updateRow(row.id, { deleted: !row.deleted });
	}

	function updateRow(rowId: string, changes: Partial<InventoryRow>) {
		rows = rows.map((row) => (row.id === rowId ? { ...row, ...changes } : row));
	}

	function hasPendingChange(row: InventoryRow) {
		if (row.original === null) {
			return true;
		}

		return (
			row.deleted ||
			row.item.trim() !== row.original.item ||
			Number(row.quantity) !== row.original.quantity ||
			row.quantity_type !== row.original.quantity_type
		);
	}

	function toInventoryItem(row: InventoryRow): InventoryItem {
		return {
			item: row.item.trim(),
			quantity: Number(row.quantity),
			quantity_type: row.quantity_type
		};
	}

	function validatePendingRows() {
		for (const row of rows) {
			if (!hasPendingChange(row) || row.deleted) continue;

			if (row.item.trim().length === 0) {
				return 'Inventory item names are required.';
			}

			if (!Number.isFinite(Number(row.quantity)) || Number(row.quantity) < 0) {
				return 'Quantities must be zero or greater.';
			}
		}

		return '';
	}

	async function sendChanges(method: 'POST' | 'PUT' | 'DELETE', body: InventoryItem[] | string[]) {
		if (body.length === 0) return;

		const response = await fetch(apiPath, {
			method,
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify(body)
		});

		if (!response.ok) {
			throw new Error(`Inventory save failed with status ${response.status}`);
		}
	}

	async function updateInventory() {
		const validationMessage = validatePendingRows();
		if (validationMessage) {
			saveState = 'error';
			errorMessage = validationMessage;
			return;
		}

		saveState = 'saving';
		errorMessage = '';

		const creates = rows
			.filter((row) => hasPendingChange(row) && !row.deleted && row.original === null)
			.map(toInventoryItem);
		const renames = rows
			.filter((row) => hasPendingChange(row) && !row.deleted && row.original !== null)
			.filter((row) => row.item.trim() !== row.original?.item);
		const updates = rows
			.filter((row) => hasPendingChange(row) && !row.deleted && row.original !== null)
			.filter((row) => row.item.trim() === row.original?.item)
			.map(toInventoryItem);
		const deletes = rows
			.filter((row) => row.original !== null && (row.deleted || renames.includes(row)))
			.map((row) => row.original?.item)
			.filter((item): item is string => Boolean(item));

		try {
			await sendChanges('POST', [...creates, ...renames.map(toInventoryItem)]);
			await sendChanges('PUT', updates);
			await sendChanges('DELETE', deletes);

			const response = await fetch(apiPath);
			if (!response.ok) {
				throw new Error(`Inventory refresh failed with status ${response.status}`);
			}

			const savedItems = (await response.json()) as InventoryItem[];
			sessionStorage.removeItem(pendingStorageKey);
			committedItems = savedItems;
			rows = createRows(savedItems);
			saveState = 'idle';
		} catch (error) {
			saveState = 'error';
			errorMessage = error instanceof Error ? error.message : 'Inventory save failed.';
		}
	}

	function cancelChanges() {
		rows = createRows(committedItems);
		sessionStorage.removeItem(pendingStorageKey);
		saveState = 'idle';
		errorMessage = '';
	}
</script>

<section class="mx-auto flex w-full max-w-3xl flex-col gap-4 p-6">
	<div class="flex flex-wrap items-center justify-between gap-3">
		<h1 class="text-2xl font-semibold text-zinc-950">Inventory</h1>
		<div class="flex items-center gap-2">
			<button
				type="button"
				class="rounded border border-zinc-300 px-3 py-2 text-sm font-medium text-zinc-800 disabled:cursor-not-allowed disabled:opacity-50"
				disabled={!hasPendingChanges || saveState === 'saving'}
				onclick={cancelChanges}
			>
				cancel changes
			</button>
			<button
				type="button"
				class="rounded bg-zinc-950 px-3 py-2 text-sm font-medium text-white disabled:cursor-not-allowed disabled:opacity-50"
				disabled={!hasPendingChanges || saveState === 'saving'}
				onclick={updateInventory}
			>
				{saveState === 'saving' ? 'updating...' : 'update'}
			</button>
		</div>
	</div>

	{#if saveState === 'error'}
		<p class="rounded border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-700">
			{errorMessage}
		</p>
	{/if}

	{#if rows.length > 0}
		<ul aria-label="Inventory items" class="divide-y divide-zinc-200 border-y border-zinc-200">
			{#each rows as row (row.id)}
				<li class="grid gap-3 py-3 sm:grid-cols-[1fr_7rem_9rem_auto] sm:items-center">
					<label class="min-w-0">
						<span class="sr-only">Item</span>
						<input
							type="text"
							class:line-through={row.deleted}
							class="w-full rounded border-zinc-300 text-sm font-medium text-zinc-900 disabled:bg-zinc-100"
							value={row.item}
							disabled={row.deleted}
							oninput={(event) => updateRow(row.id, { item: event.currentTarget.value })}
						/>
					</label>
					<label>
						<span class="sr-only">Quantity</span>
						<input
							type="number"
							min="0"
							step="any"
							class:line-through={row.deleted}
							class="w-full rounded border-zinc-300 text-sm text-zinc-900 tabular-nums disabled:bg-zinc-100"
							value={row.quantity}
							disabled={row.deleted}
							oninput={(event) => updateRow(row.id, { quantity: event.currentTarget.value })}
						/>
					</label>
					<label>
						<span class="sr-only">Quantity type</span>
						<select
							class:line-through={row.deleted}
							class="w-full rounded border-zinc-300 text-sm text-zinc-900 disabled:bg-zinc-100"
							value={row.quantity_type}
							disabled={row.deleted}
							onchange={(event) =>
								updateRow(row.id, {
									quantity_type: event.currentTarget.value as QuantityType
								})}
						>
							{#each quantityTypes as quantityType (quantityType)}
								<option value={quantityType}>{quantityTypeLabels[quantityType]}</option>
							{/each}
						</select>
					</label>
					<div class="flex items-center justify-between gap-3 sm:justify-end">
						<span
							aria-label={hasPendingChange(row) ? 'Pending edit' : 'No pending edit'}
							class="w-4 text-center text-lg font-semibold text-zinc-950"
						>
							{hasPendingChange(row) ? '*' : ''}
						</span>
						<button
							type="button"
							class="flex size-9 items-center justify-center rounded border border-zinc-300 text-lg font-semibold text-zinc-800"
							aria-label={row.deleted
								? `Undelete ${row.item || 'inventory line'}`
								: `Delete ${row.item || 'inventory line'}`}
							onclick={() => toggleDeleted(row)}
						>
							-
						</button>
					</div>
				</li>
			{/each}
		</ul>
	{:else}
		<p class="border-y border-zinc-200 py-4 text-zinc-600">No inventory items yet.</p>
	{/if}

	<button
		type="button"
		class="flex size-10 items-center justify-center rounded border border-zinc-300 text-xl font-semibold text-zinc-900"
		aria-label="Add inventory line"
		onclick={addRow}
	>
		+
	</button>
</section>
