import { env } from '$env/dynamic/private';
import { error } from '@sveltejs/kit';
import type { InventoryItem } from '$lib/inventory';

const defaultBackendOrigin = 'http://127.0.0.1:3000';

export async function listInventory(fetchImpl: typeof fetch): Promise<InventoryItem[]> {
	const backendOrigin = env.BACKEND_ORIGIN ?? defaultBackendOrigin;
	const response = await fetchImpl(new URL('/v1/inventory', backendOrigin));

	if (!response.ok) {
		error(response.status, 'Failed to load inventory');
	}

	return (await response.json()) as InventoryItem[];
}
