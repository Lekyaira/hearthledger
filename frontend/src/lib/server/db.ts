import { env } from '$env/dynamic/private';
import { error } from '@sveltejs/kit';
import type { InventoryItem } from '$lib/inventory';
import type { User } from '$lib/users';

const defaultBackendOrigin = 'http://127.0.0.1:3000';

export async function listInventory(fetchImpl: typeof fetch): Promise<InventoryItem[]> {
	const backendOrigin = env.BACKEND_ORIGIN ?? defaultBackendOrigin;
	const response = await fetchImpl(new URL('/v1/inventory', backendOrigin));

	if (!response.ok) {
		error(response.status, 'Failed to load inventory');
	}

	return (await response.json()) as InventoryItem[];
}

export async function listUsers(fetchImpl: typeof fetch): Promise<User[]> {
	const backendOrigin = env.BACKEND_ORIGIN ?? defaultBackendOrigin;
	const response = await fetchImpl(new URL('/v1/users', backendOrigin));

	if (!response.ok) {
		error(response.status, 'Failed to load users');
	}

	return (await response.json()) as User[];
}
