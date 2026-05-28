import { listUsers } from '$lib/server/db';

export const load = async ({ fetch }) => {
	return {
		users: await listUsers(fetch)
	};
};
