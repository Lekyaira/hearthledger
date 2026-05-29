<script lang="ts">
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { LogIn, ShelvingUnit, UsersRound } from '@lucide/svelte';
	import { readCurrentUser, readCurrentUserId, userChangedEvent } from '$lib/auth';
	import type { User } from '$lib/users';
	import Logout from '$lib/Logout.svelte';
	import PendingBundlesIcon from '$lib/PendingBundlesIcon.svelte';

	let userId = $state('');
	let currentUser = $state<User | null>(null);

	function refreshUserId() {
		userId = readCurrentUserId();
		void refreshCurrentUser();
	}

	async function refreshCurrentUser() {
		currentUser = await readCurrentUser();
	}

	onMount(() => {
		refreshUserId();
		window.addEventListener(userChangedEvent, refreshUserId);
		window.addEventListener('storage', refreshUserId);

		return () => {
			window.removeEventListener(userChangedEvent, refreshUserId);
			window.removeEventListener('storage', refreshUserId);
		};
	});
</script>

<header class="sticky top-0 z-10 border-b border-zinc-200 bg-white/95 backdrop-blur">
	<nav class="mx-auto flex min-h-16 w-full max-w-4xl items-center justify-between gap-4 px-6">
		<a href="/" class="text-lg font-semibold text-zinc-950">Hearthledger</a>

		<div class="flex items-center gap-2">
			{#if userId}
				<button
					type="button"
					class="flex size-10 items-center justify-center rounded border border-zinc-300 text-zinc-900"
					aria-label="Inventory"
					onclick={() => goto('/inventory')}
				>
					<ShelvingUnit size={22} aria-hidden="true" />
				</button>
				<PendingBundlesIcon />
				{#if currentUser?.role === 'admin'}
					<button
						type="button"
						class="flex size-10 items-center justify-center rounded border border-zinc-300 text-zinc-900"
						aria-label="Users"
						onclick={() => goto('/users')}
					>
						<UsersRound size={22} aria-hidden="true" />
					</button>
				{/if}
				<Logout />
			{:else}
				<button
					type="button"
					class="flex size-10 items-center justify-center rounded border border-zinc-300 text-zinc-900"
					aria-label="Log in"
					onclick={() => goto('/login')}
				>
					<LogIn size={22} aria-hidden="true" />
				</button>
			{/if}
		</div>
	</nav>
</header>
