<script lang="ts">
	import Login from '$lib/Login.svelte';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();
</script>

<section class="flex min-h-[calc(100vh-4rem)] items-center justify-center p-6">
	<Login users={data.users} />
</section>
