import { env } from '$env/dynamic/private';
import { json, type RequestHandler } from '@sveltejs/kit';

const defaultBackendOrigin = 'http://127.0.0.1:3000';

function inventoryUrl() {
	return new URL('/v1/inventory', env.BACKEND_ORIGIN ?? defaultBackendOrigin);
}

async function proxyInventory(request: Request, method: 'POST' | 'PUT' | 'DELETE') {
	const response = await fetch(inventoryUrl(), {
		method,
		headers: { 'content-type': 'application/json' },
		body: await request.text()
	});

	if (!response.ok) {
		return json({ message: 'Failed to save inventory changes' }, { status: response.status });
	}

	if (response.status === 204) {
		return new Response(null, { status: 204 });
	}

	return json(await response.json(), { status: response.status });
}

export const GET: RequestHandler = async () => {
	const response = await fetch(inventoryUrl());

	if (!response.ok) {
		return json({ message: 'Failed to load inventory' }, { status: response.status });
	}

	return json(await response.json());
};

export const POST: RequestHandler = async ({ request }) => proxyInventory(request, 'POST');
export const PUT: RequestHandler = async ({ request }) => proxyInventory(request, 'PUT');
export const DELETE: RequestHandler = async ({ request }) => proxyInventory(request, 'DELETE');
