import { listInventory, listUsers } from '$lib/server/db';

export const load = async ({ fetch }) => {
	return {
		inventory: await listInventory(fetch),
		users: await listUsers(fetch)
	};
};
