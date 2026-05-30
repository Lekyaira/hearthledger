<script lang="ts">
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { LogIn, Moon, ShelvingUnit, Sun, UsersRound } from '@lucide/svelte';
	import { readCurrentUser, readCurrentUserId, userChangedEvent } from '$lib/auth';
	import type { User } from '$lib/users';
	import Logout from '$lib/Logout.svelte';
	import PendingBundlesIcon from '$lib/PendingBundlesIcon.svelte';

	const themeStorageKey = 'hearthledger-theme';

	let userId = $state('');
	let currentUser = $state<User | null>(null);
	let theme = $state<'light' | 'dark'>('light');

	function refreshUserId() {
		userId = readCurrentUserId();
		void refreshCurrentUser();
	}

	async function refreshCurrentUser() {
		currentUser = await readCurrentUser();
	}

	function applyTheme(nextTheme: 'light' | 'dark') {
		theme = nextTheme;
		document.documentElement.dataset.theme = nextTheme;
		localStorage.setItem(themeStorageKey, nextTheme);
	}

	function toggleTheme() {
		applyTheme(theme === 'light' ? 'dark' : 'light');
	}

	function refreshTheme(event?: StorageEvent) {
		if (event && event.key !== themeStorageKey) return;

		const storedTheme = localStorage.getItem(themeStorageKey);
		const nextTheme = storedTheme === 'dark' ? 'dark' : 'light';
		theme = nextTheme;
		document.documentElement.dataset.theme = nextTheme;
	}

	onMount(() => {
		refreshUserId();
		refreshTheme();
		window.addEventListener(userChangedEvent, refreshUserId);
		window.addEventListener('storage', refreshUserId);
		window.addEventListener('storage', refreshTheme);

		return () => {
			window.removeEventListener(userChangedEvent, refreshUserId);
			window.removeEventListener('storage', refreshUserId);
			window.removeEventListener('storage', refreshTheme);
		};
	});
</script>

<header class="sticky top-0 z-10 border-b border-zinc-200 bg-white/95 backdrop-blur">
	<nav class="mx-auto flex min-h-16 w-full max-w-4xl items-center justify-between gap-4 px-6">
		<a href="/" class="text-lg font-semibold text-zinc-950">Hearthledger</a>

		<div class="flex items-center gap-2">
			<button
				type="button"
				class="flex size-10 items-center justify-center rounded border border-zinc-300 text-zinc-900"
				aria-label={theme === 'light' ? 'Switch to dark mode' : 'Switch to light mode'}
				onclick={toggleTheme}
			>
				{#if theme === 'light'}
					<Moon size={22} aria-hidden="true" />
				{:else}
					<Sun size={22} aria-hidden="true" />
				{/if}
			</button>
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
