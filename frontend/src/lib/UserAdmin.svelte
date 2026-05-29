<script lang="ts">
	import type { User, UserRole } from '$lib/users';

	type Props = {
		users: User[];
		currentUserId: string;
		apiPath?: string;
	};

	type NewUser = {
		name: string;
		role: UserRole;
	};

	let { users, currentUserId, apiPath = '/api/users' }: Props = $props();
	let committedUsers = $derived(users);
	let visibleUsers = $derived([...committedUsers].sort(compareUsers));
	let newUserName = $state('');
	let newUserRole = $state<UserRole>('member');
	let saveState = $state<'idle' | 'saving' | 'error'>('idle');
	let errorMessage = $state('');

	function compareUsers(a: User, b: User) {
		return (
			a.name.localeCompare(b.name, undefined, { sensitivity: 'base' }) || a.id.localeCompare(b.id)
		);
	}

	function validateNewUser() {
		const name = newUserName.trim();

		if (!name) {
			return 'Name is required.';
		}

		return '';
	}

	async function refreshUsers() {
		const response = await fetch(apiPath);
		if (!response.ok) {
			throw new Error(`User refresh failed with status ${response.status}`);
		}

		committedUsers = (await response.json()) as User[];
	}

	async function addUser() {
		const validationMessage = validateNewUser();
		if (validationMessage) {
			saveState = 'error';
			errorMessage = validationMessage;
			return;
		}

		saveState = 'saving';
		errorMessage = '';

		const user: NewUser = {
			name: newUserName.trim(),
			role: newUserRole
		};

		try {
			const response = await fetch(apiPath, {
				method: 'POST',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify([user])
			});

			if (!response.ok) {
				throw new Error(`User add failed with status ${response.status}`);
			}

			await refreshUsers();
			newUserName = '';
			newUserRole = 'member';
			saveState = 'idle';
		} catch (error) {
			saveState = 'error';
			errorMessage = error instanceof Error ? error.message : 'User add failed.';
		}
	}

	async function removeUser(user: User) {
		if (user.id === currentUserId) return;

		saveState = 'saving';
		errorMessage = '';

		try {
			const response = await fetch(apiPath, {
				method: 'DELETE',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify([user])
			});

			if (!response.ok) {
				throw new Error(`User remove failed with status ${response.status}`);
			}

			await refreshUsers();
			saveState = 'idle';
		} catch (error) {
			saveState = 'error';
			errorMessage = error instanceof Error ? error.message : 'User remove failed.';
		}
	}
</script>

<section class="mx-auto flex w-full max-w-3xl flex-col gap-4 p-6">
	<div class="flex flex-wrap items-center justify-between gap-3">
		<h1 class="text-2xl font-semibold text-zinc-950">Users</h1>
	</div>

	{#if saveState === 'error'}
		<p class="rounded border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-700">
			{errorMessage}
		</p>
	{/if}

	<form
		class="grid gap-3 border-y border-zinc-200 py-4 sm:grid-cols-[1fr_9rem_auto] sm:items-end"
		onsubmit={(event) => {
			event.preventDefault();
			void addUser();
		}}
	>
		<label class="flex min-w-0 flex-col gap-1 text-sm font-medium text-zinc-800">
			Name
			<input
				type="text"
				class="w-full rounded border-zinc-300 text-sm text-zinc-900"
				bind:value={newUserName}
				disabled={saveState === 'saving'}
			/>
		</label>
		<label class="flex flex-col gap-1 text-sm font-medium text-zinc-800">
			Role
			<select
				class="w-full rounded border-zinc-300 text-sm text-zinc-900"
				bind:value={newUserRole}
				disabled={saveState === 'saving'}
			>
				<option value="member">member</option>
				<option value="admin">admin</option>
			</select>
		</label>
		<button
			type="submit"
			class="rounded bg-zinc-950 px-3 py-2 text-sm font-medium text-white disabled:cursor-not-allowed disabled:opacity-50"
			disabled={saveState === 'saving'}
		>
			{saveState === 'saving' ? 'adding...' : 'add'}
		</button>
	</form>

	{#if visibleUsers.length > 0}
		<ul aria-label="Users" class="divide-y divide-zinc-200 border-y border-zinc-200">
			{#each visibleUsers as user (user.id)}
				<li class="grid gap-3 py-3 sm:grid-cols-[1fr_7rem_auto] sm:items-center">
					<div class="min-w-0">
						<p class="truncate text-sm font-medium text-zinc-950">{user.name}</p>
						<p class="truncate text-xs text-zinc-500">{user.id}</p>
					</div>
					<p class="text-sm text-zinc-700">{user.role}</p>
					<button
						type="button"
						class="rounded border border-zinc-300 px-3 py-2 text-sm font-medium text-zinc-800 disabled:cursor-not-allowed disabled:opacity-50"
						aria-label={`Remove ${user.name}`}
						disabled={saveState === 'saving' || user.id === currentUserId}
						onclick={() => void removeUser(user)}
					>
						remove
					</button>
				</li>
			{/each}
		</ul>
	{:else}
		<p class="border-y border-zinc-200 py-4 text-zinc-600">No users yet.</p>
	{/if}
</section>
