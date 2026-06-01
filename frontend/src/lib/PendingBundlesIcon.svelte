<script lang="ts">
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { Handbag } from '@lucide/svelte';
	import { readCurrentUser, userChangedEvent } from '$lib/auth';
	import { bundlesChangedEvent, readCurrentUserId, type Bundle } from '$lib/bundles';

	type Props = {
		apiPath?: string;
		href?: string;
	};

	let { apiPath = '/api/bundles', href = '/bundles' }: Props = $props();
	let openBundleCount = $state(0);
	let readyBundleCount = $state(0);

	onMount(() => {
		void loadOpenBundleCount();
		window.addEventListener(bundlesChangedEvent, loadOpenBundleCount);
		window.addEventListener(userChangedEvent, loadOpenBundleCount);
		window.addEventListener('storage', loadOpenBundleCount);

		return () => {
			window.removeEventListener(bundlesChangedEvent, loadOpenBundleCount);
			window.removeEventListener(userChangedEvent, loadOpenBundleCount);
			window.removeEventListener('storage', loadOpenBundleCount);
		};
	});

	async function loadOpenBundleCount() {
		const currentUser = await readCurrentUser();
		const userId = readCurrentUserId();
		const response = await fetch(apiPath);
		if (!response.ok) return;

		const bundles = (await response.json()) as Bundle[];
		const visibleBundles = bundles.filter(
			(bundle) => currentUser?.role === 'admin' || bundle.user === userId
		);
		openBundleCount =
			currentUser?.role === 'admin'
				? visibleBundles.length
				: visibleBundles.filter((bundle) => !bundle.bundled).length;
		readyBundleCount =
			currentUser?.role === 'admin' ? 0 : visibleBundles.filter((bundle) => bundle.bundled).length;
	}
</script>

<button
	type="button"
	class="relative flex size-10 items-center justify-center rounded border border-zinc-300 text-zinc-900"
	aria-label="Pending bundles"
	onclick={() => goto(href)}
>
	<Handbag size={22} aria-hidden="true" />
	{#if openBundleCount > 0}
		<span
			class="absolute -top-2 -right-2 flex min-w-5 items-center justify-center rounded-full bg-red-600 px-1 text-xs leading-5 font-semibold text-white"
			aria-label={`${openBundleCount} open bundles`}
		>
			{openBundleCount}
		</span>
	{/if}
	{#if readyBundleCount > 0}
		<span
			class="absolute -right-2 -bottom-2 flex min-w-5 items-center justify-center rounded-full bg-green-600 px-1 text-xs leading-5 font-semibold text-white"
			aria-label={`${readyBundleCount} bundles ready for pickup`}
		>
			{readyBundleCount}
		</span>
	{/if}
</button>
