<script lang="ts">
	import { goto } from '$app/navigation';
	import type { User } from '$lib/users';
	import { setCurrentUserId } from '$lib/auth';

	type Props = {
		users: User[];
	};

	let { users }: Props = $props();
	let selectedUserId = $state('');

	$effect(() => {
		if (users.length === 0) {
			selectedUserId = '';
			return;
		}

		if (!users.some((user) => user.id === selectedUserId)) {
			selectedUserId = users[0].id;
		}
	});

	function login() {
		if (!selectedUserId) return;

		setCurrentUserId(selectedUserId);
		void goto('/bundles');
	}
</script>

<form
	class="flex w-full max-w-sm flex-col gap-4 rounded border border-zinc-200 bg-white p-5 shadow-sm"
	onsubmit={(event) => {
		event.preventDefault();
		login();
	}}
>
	<label class="flex flex-col gap-2 text-sm font-medium text-zinc-800">
		User
		<select
			class="rounded border-zinc-300 text-zinc-900"
			bind:value={selectedUserId}
			disabled={users.length === 0}
		>
			{#each users as user (user.id)}
				<option value={user.id}>{user.name}</option>
			{/each}
		</select>
	</label>

	<button
		type="submit"
		class="rounded bg-zinc-950 px-3 py-2 text-sm font-medium text-white disabled:cursor-not-allowed disabled:opacity-50"
		disabled={!selectedUserId}
	>
		login
	</button>
</form>
