<script lang="ts">
	import { onMount } from 'svelte';
	import { ArchiveRestore, ChevronRight, PackageCheck, X } from '@lucide/svelte';
	import { readCurrentUser } from '$lib/auth';
	import {
		bundlesChangedEvent,
		pendingBundlesStorageKey,
		readCurrentUserId,
		type Bundle,
		type UpdatedBundle
	} from '$lib/bundles';

	type Props = {
		apiPath?: string;
	};

	type BundleRow = Omit<Bundle, 'items'> & {
		deleted: boolean;
		items: ItemRow[];
	};

	type ItemRow = Bundle['items'][number] & {
		deleted: boolean;
		quantityValue: string;
	};

	let { apiPath = '/api/bundle' }: Props = $props();
	let bundles = $state<BundleRow[]>([]);
	let expandedBundleIds = $state<Set<number>>(new Set());
	let loadState = $state<'loading' | 'idle' | 'saving' | 'error'>('loading');
	let errorMessage = $state('');
	let isAdmin = $state(false);
	let hasMounted = false;

	const listApiPath = '/api/bundles';
	const dateFormatter = new Intl.DateTimeFormat(undefined, {
		dateStyle: 'medium',
		timeStyle: 'short'
	});
	const hasPendingChanges = $derived(bundles.some((bundle) => hasPendingChange(bundle)));

	$effect(() => {
		if (!hasMounted) return;

		if (!hasPendingChanges) {
			sessionStorage.removeItem(pendingBundlesStorageKey);
			return;
		}

		sessionStorage.setItem(pendingBundlesStorageKey, JSON.stringify(bundles));
	});

	onMount(() => {
		hasMounted = true;
		void loadBundles();
	});

	function createRows(sourceBundles: Bundle[]): BundleRow[] {
		return sourceBundles.map((bundle) => ({
			...bundle,
			deleted: false,
			items: bundle.items.map((item) => ({
				...item,
				deleted: false,
				quantityValue: String(item.quantity)
			}))
		}));
	}

	function readStoredRows() {
		const storedValue = sessionStorage.getItem(pendingBundlesStorageKey);
		if (!storedValue) return null;

		try {
			const storedRows = JSON.parse(storedValue) as BundleRow[];
			return Array.isArray(storedRows) ? storedRows : null;
		} catch {
			return null;
		}
	}

	function mergeStoredRows(loadedRows: BundleRow[]) {
		const storedRows = readStoredRows();
		if (!storedRows) return loadedRows;

		const loadedIds = new Set(loadedRows.map((bundle) => bundle.id));
		const storedRowsById = new Map(
			storedRows.filter((bundle) => loadedIds.has(bundle.id)).map((bundle) => [bundle.id, bundle])
		);

		return loadedRows.map((bundle) => storedRowsById.get(bundle.id) ?? bundle);
	}

	async function loadBundles() {
		loadState = 'loading';
		errorMessage = '';

		try {
			const response = await fetch(listApiPath);
			if (!response.ok) {
				throw new Error(`Pending bundles failed to load with status ${response.status}.`);
			}

			const currentUser = await readCurrentUser();
			isAdmin = currentUser?.role === 'admin';
			const userId = readCurrentUserId();
			const loadedBundles = ((await response.json()) as Bundle[]).filter(
				(bundle) => isAdmin || bundle.user === userId
			);
			bundles = mergeStoredRows(createRows(loadedBundles));
			expandedBundleIds = new Set();
			loadState = 'idle';
		} catch (error) {
			loadState = 'error';
			errorMessage = error instanceof Error ? error.message : 'Pending bundles failed to load.';
		}
	}

	function formatCreatedAt(value: string) {
		const date = new Date(value);
		if (Number.isNaN(date.getTime())) return value;
		return dateFormatter.format(date);
	}

	function toggleExpanded(bundleId: number) {
		const nextExpandedBundleIds = new Set(expandedBundleIds);
		if (nextExpandedBundleIds.has(bundleId)) {
			nextExpandedBundleIds.delete(bundleId);
		} else {
			nextExpandedBundleIds.add(bundleId);
		}
		expandedBundleIds = nextExpandedBundleIds;
	}

	function updateBundle(bundleId: number, changes: Partial<BundleRow>) {
		bundles = bundles.map((bundle) =>
			bundle.id === bundleId ? { ...bundle, ...changes } : bundle
		);
	}

	function updateItem(bundleId: number, itemId: number, changes: Partial<ItemRow>) {
		bundles = bundles.map((bundle) =>
			bundle.id === bundleId
				? {
						...bundle,
						items: bundle.items.map((item) =>
							item.item_id === itemId ? { ...item, ...changes } : item
						)
					}
				: bundle
		);
	}

	function hasPendingChange(bundle: BundleRow) {
		return (
			bundle.deleted ||
			Boolean(bundle.fulfilled_at) ||
			bundle.items.some((item) => item.deleted || Number(item.quantityValue) !== item.quantity)
		);
	}

	function validateBundles() {
		for (const bundle of bundles) {
			if (bundle.deleted || bundle.bundled || bundle.fulfilled_at) continue;

			const activeItems = bundle.items.filter((item) => !item.deleted);
			if (activeItems.length === 0) {
				return `Bundle ${bundle.id} needs at least one item or should be deleted.`;
			}

			if (
				activeItems.some(
					(item) => !Number.isFinite(Number(item.quantityValue)) || Number(item.quantityValue) <= 0
				)
			) {
				return 'Bundle quantities must be greater than zero.';
			}
		}

		return '';
	}

	async function saveChanges() {
		const validationMessage = validateBundles();
		if (validationMessage) {
			loadState = 'error';
			errorMessage = validationMessage;
			return;
		}

		loadState = 'saving';
		errorMessage = '';

		try {
			for (const bundle of bundles.filter(hasPendingChange)) {
				if (bundle.bundled && !bundle.fulfilled_at) continue;

				if (bundle.deleted) {
					const response = await fetch(`${apiPath}?id=${bundle.id}`, { method: 'DELETE' });
					if (!response.ok) {
						throw new Error(`Bundle ${bundle.id} could not be deleted.`);
					}
					continue;
				}

				const updatedBundle: UpdatedBundle = {
					id: bundle.id,
					user: bundle.user,
					bundled: bundle.bundled,
					fulfilled_at: bundle.fulfilled_at ?? null,
					items: bundle.items
						.filter((item) => !item.deleted)
						.map((item) => ({
							item_id: item.item_id,
							quantity: Number(item.quantityValue)
						}))
				};
				const response = await fetch(apiPath, {
					method: 'PUT',
					headers: { 'content-type': 'application/json' },
					body: JSON.stringify(updatedBundle)
				});

				if (!response.ok) {
					throw new Error(`Bundle ${bundle.id} update failed with status ${response.status}.`);
				}
			}

			sessionStorage.removeItem(pendingBundlesStorageKey);
			window.dispatchEvent(new CustomEvent(bundlesChangedEvent));
			await loadBundles();
		} catch (error) {
			loadState = 'error';
			errorMessage = error instanceof Error ? error.message : 'Bundle update failed.';
		}
	}

	function cancelChanges() {
		sessionStorage.removeItem(pendingBundlesStorageKey);
		void loadBundles();
	}

	function toggleCompleted(bundle: BundleRow) {
		updateBundle(bundle.id, {
			deleted: false,
			fulfilled_at: bundle.fulfilled_at ? null : new Date().toISOString()
		});
	}
</script>

<section class="mx-auto flex w-full max-w-4xl flex-col gap-4 p-6">
	<div class="flex flex-wrap items-center justify-between gap-3">
		<h1 class="text-2xl font-semibold text-zinc-950">Pending bundles</h1>
		<div class="flex items-center gap-2">
			<button
				type="button"
				class="rounded border border-zinc-300 px-3 py-2 text-sm font-medium text-zinc-800 disabled:cursor-not-allowed disabled:opacity-50"
				disabled={!hasPendingChanges || loadState === 'saving'}
				onclick={cancelChanges}
			>
				cancel
			</button>
			<button
				type="button"
				class="rounded bg-zinc-950 px-3 py-2 text-sm font-medium text-white disabled:cursor-not-allowed disabled:opacity-50"
				disabled={!hasPendingChanges || loadState === 'saving'}
				onclick={saveChanges}
			>
				{loadState === 'saving' ? 'updating...' : 'update'}
			</button>
		</div>
	</div>

	{#if loadState === 'error'}
		<p class="rounded border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-700">
			{errorMessage}
		</p>
	{/if}

	{#if loadState === 'loading'}
		<p class="border-y border-zinc-200 py-4 text-zinc-600">Loading pending bundles...</p>
	{:else if bundles.length === 0}
		<p class="border-y border-zinc-200 py-4 text-zinc-600">No pending bundles.</p>
	{:else}
		<ul aria-label="Pending bundles" class="divide-y divide-zinc-200 border-y border-zinc-200">
			{#each bundles as bundle (bundle.id)}
				<li class="py-3">
					<div class="flex items-center gap-3">
						<button
							type="button"
							class="flex size-8 items-center justify-center rounded border border-zinc-300 text-zinc-800"
							aria-label={expandedBundleIds.has(bundle.id)
								? `Collapse bundle ${bundle.id}`
								: `Expand bundle ${bundle.id}`}
							onclick={() => toggleExpanded(bundle.id)}
						>
							<ChevronRight
								size={18}
								class={expandedBundleIds.has(bundle.id) ? 'rotate-90' : ''}
								aria-hidden="true"
							/>
						</button>
						<div class="min-w-0 flex-1">
							<p
								class:line-through={bundle.deleted || bundle.fulfilled_at}
								class="font-medium text-zinc-950"
							>
								Bundle {bundle.id}
							</p>
							<p class="text-sm text-zinc-600">{formatCreatedAt(bundle.created_at)}</p>
							{#if bundle.fulfilled_at}
								<p class="text-sm font-medium text-green-700">completion pending update</p>
							{/if}
						</div>
						{#if bundle.bundled}
							<span class="flex items-center gap-2 text-sm font-medium text-green-700">
								<ArchiveRestore size={18} aria-hidden="true" />
								ready for pickup
							</span>
						{/if}
						{#if isAdmin}
							<button
								type="button"
								class="flex size-9 items-center justify-center rounded border border-green-300 text-green-700 disabled:cursor-not-allowed disabled:opacity-50"
								aria-label={bundle.fulfilled_at
									? `Keep bundle ${bundle.id} pending`
									: `Mark bundle ${bundle.id} complete`}
								disabled={bundle.deleted}
								onclick={() => toggleCompleted(bundle)}
							>
								<PackageCheck size={18} aria-hidden="true" />
							</button>
						{/if}
						{#if !bundle.bundled}
							<button
								type="button"
								class="flex size-9 items-center justify-center rounded border border-red-300 text-red-700 disabled:cursor-not-allowed disabled:opacity-50"
								aria-label={bundle.deleted
									? `Keep bundle ${bundle.id}`
									: `Delete bundle ${bundle.id}`}
								disabled={Boolean(bundle.fulfilled_at)}
								onclick={() => updateBundle(bundle.id, { deleted: !bundle.deleted })}
							>
								<X size={18} aria-hidden="true" />
							</button>
						{/if}
					</div>

					{#if expandedBundleIds.has(bundle.id)}
						<ul class="mt-3 divide-y divide-zinc-100 pl-11">
							{#each bundle.items as item (item.item_id)}
								<li class="grid gap-3 py-2 sm:grid-cols-[1fr_7rem_auto] sm:items-center">
									<span
										class:line-through={bundle.deleted ||
											Boolean(bundle.fulfilled_at) ||
											item.deleted}
										class="min-w-0 truncate text-sm font-medium text-zinc-900"
									>
										{item.item}
									</span>
									<label>
										<span class="sr-only">Quantity for {item.item}</span>
										<input
											type="number"
											min="0"
											step="any"
											class:line-through={bundle.deleted ||
												Boolean(bundle.fulfilled_at) ||
												item.deleted}
											class="w-full rounded border-zinc-300 text-sm text-zinc-900 tabular-nums disabled:bg-zinc-100"
											value={item.quantityValue}
											disabled={bundle.bundled ||
												bundle.deleted ||
												Boolean(bundle.fulfilled_at) ||
												item.deleted}
											oninput={(event) =>
												updateItem(bundle.id, item.item_id, {
													quantityValue: event.currentTarget.value
												})}
										/>
									</label>
									{#if !bundle.bundled}
										<button
											type="button"
											class="flex size-9 items-center justify-center rounded border border-red-300 text-red-700 disabled:cursor-not-allowed disabled:opacity-50"
											disabled={bundle.deleted || Boolean(bundle.fulfilled_at)}
											aria-label={item.deleted
												? `Keep ${item.item} in bundle ${bundle.id}`
												: `Delete ${item.item} from bundle ${bundle.id}`}
											onclick={() =>
												updateItem(bundle.id, item.item_id, {
													deleted: !item.deleted
												})}
										>
											<X size={18} aria-hidden="true" />
										</button>
									{/if}
								</li>
							{/each}
						</ul>
					{/if}
				</li>
			{/each}
		</ul>
	{/if}
</section>
