import { env } from '$env/dynamic/private';
import { json, type RequestHandler } from '@sveltejs/kit';

const defaultBackendOrigin = 'http://127.0.0.1:3000';

function bundleUrl(id?: string | null) {
	const url = new URL('/v1/bundle', env.BACKEND_ORIGIN ?? defaultBackendOrigin);
	if (id) url.searchParams.set('id', id);
	return url;
}

async function proxyBundle(request: Request, method: 'POST' | 'PUT') {
	const response = await fetch(bundleUrl(), {
		method,
		headers: { 'content-type': 'application/json' },
		body: await request.text()
	});

	if (!response.ok) {
		return json({ message: 'Failed to save bundle changes' }, { status: response.status });
	}

	return json(await response.json(), { status: response.status });
}

export const POST: RequestHandler = async ({ request }) => proxyBundle(request, 'POST');
export const PUT: RequestHandler = async ({ request }) => proxyBundle(request, 'PUT');

export const DELETE: RequestHandler = async ({ url }) => {
	const response = await fetch(bundleUrl(url.searchParams.get('id')), {
		method: 'DELETE'
	});

	if (!response.ok) {
		return json({ message: 'Failed to delete bundle' }, { status: response.status });
	}

	return new Response(null, { status: 204 });
};
