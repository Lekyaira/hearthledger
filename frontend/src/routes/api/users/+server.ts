import { env } from '$env/dynamic/private';
import { json, type RequestHandler } from '@sveltejs/kit';

const defaultBackendOrigin = 'http://127.0.0.1:3000';

function usersUrl() {
	return new URL('/v1/users', env.BACKEND_ORIGIN ?? defaultBackendOrigin);
}

async function forwardUsersRequest(request: Request, method: 'POST' | 'DELETE') {
	const response = await fetch(usersUrl(), {
		method,
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify(await request.json())
	});

	if (!response.ok) {
		return json({ message: 'Failed to save users' }, { status: response.status });
	}

	if (response.status === 204) {
		return new Response(null, { status: 204 });
	}

	return json(await response.json(), { status: response.status });
}

export const GET: RequestHandler = async () => {
	const response = await fetch(usersUrl());

	if (!response.ok) {
		return json({ message: 'Failed to load users' }, { status: response.status });
	}

	return json(await response.json());
};

export const POST: RequestHandler = async ({ request }) => {
	return forwardUsersRequest(request, 'POST');
};

export const DELETE: RequestHandler = async ({ request }) => {
	return forwardUsersRequest(request, 'DELETE');
};
