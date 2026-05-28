<script lang="ts">
	import { goto } from '$app/navigation';
	import { LogOut } from '@lucide/svelte';
	import { clearCurrentUserId } from '$lib/auth';

	function logout() {
		clearCurrentUserId();
		void goto('/login');
	}
</script>

<button
	type="button"
	class="flex size-10 items-center justify-center rounded border border-zinc-300 text-zinc-900"
	aria-label="Log out"
	onclick={logout}
>
	<LogOut size={22} aria-hidden="true" />
</button>
