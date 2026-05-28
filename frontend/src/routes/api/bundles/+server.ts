import { env } from '$env/dynamic/private';
import { json, type RequestHandler } from '@sveltejs/kit';

const defaultBackendOrigin = 'http://127.0.0.1:3000';

function bundlesUrl() {
	return new URL('/v1/bundles', env.BACKEND_ORIGIN ?? defaultBackendOrigin);
}

export const GET: RequestHandler = async () => {
	const response = await fetch(bundlesUrl());

	if (!response.ok) {
		return json({ message: 'Failed to load bundles' }, { status: response.status });
	}

	return json(await response.json());
};
