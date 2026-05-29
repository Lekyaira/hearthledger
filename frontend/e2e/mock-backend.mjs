import http from 'node:http';

const host = '127.0.0.1';
const port = Number(process.env.E2E_BACKEND_PORT ?? 3100);

const seedUsers = [
	{ id: '1', name: 'Test Member', role: 'member' },
	{ id: '2', name: 'Test Admin', role: 'admin' },
	{ id: '3', name: 'Second Member', role: 'member' }
];

const seedInventory = [
	{ id: 1, item: 'All-purpose flour', quantity: 10.5, quantity_type: 'pounds' },
	{ id: 2, item: 'Canned tomatoes', quantity: 24, quantity_type: 'count' },
	{ id: 3, item: 'Dried black beans', quantity: 18, quantity_type: 'count' },
	{ id: 4, item: 'Laundry detergent', quantity: 6, quantity_type: 'count' },
	{ id: 5, item: 'Paper towels', quantity: 10, quantity_type: 'count' }
];

let users;
let inventory;
let bundles;
let nextUserId;
let nextInventoryId;
let nextBundleId;

function resetState() {
	users = structuredClone(seedUsers);
	inventory = structuredClone(seedInventory);
	bundles = [];
	nextUserId = Math.max(...users.map((user) => Number(user.id))) + 1;
	nextInventoryId = Math.max(...inventory.map((item) => item.id)) + 1;
	nextBundleId = 1;
}

function sendJson(response, status, body) {
	response.writeHead(status, { 'content-type': 'application/json' });
	response.end(JSON.stringify(body));
}

function sendEmpty(response, status = 204) {
	response.writeHead(status);
	response.end();
}

async function readJson(request) {
	const chunks = [];
	for await (const chunk of request) chunks.push(chunk);
	const rawBody = Buffer.concat(chunks).toString('utf8');
	return rawBody ? JSON.parse(rawBody) : null;
}

function openBundles() {
	return bundles.filter((bundle) => bundle.fulfilled_at === null);
}

function usedQuantityByItem(excludedBundleId = null) {
	const used = new Map();

	for (const bundle of openBundles()) {
		if (bundle.id === excludedBundleId) continue;

		for (const item of bundle.items) {
			used.set(item.item_id, (used.get(item.item_id) ?? 0) + item.quantity);
		}
	}

	return used;
}

function listAvailableInventory() {
	const used = usedQuantityByItem();

	return inventory
		.map((item) => ({
			...item,
			quantity: item.quantity - (used.get(item.id) ?? 0)
		}))
		.sort((a, b) => a.item.localeCompare(b.item, undefined, { sensitivity: 'base' }));
}

function bundleResponse(bundle) {
	return {
		...bundle,
		items: bundle.items.map((item) => {
			const inventoryItem = inventory.find((entry) => entry.id === item.item_id);
			return {
				item_id: item.item_id,
				item: inventoryItem?.item ?? `Item ${item.item_id}`,
				quantity: item.quantity
			};
		})
	};
}

function validateBundleItems(items, excludedBundleId = null) {
	const requested = new Map();

	for (const item of items) {
		if (!Number.isFinite(item.quantity) || item.quantity <= 0) return false;
		requested.set(item.item_id, (requested.get(item.item_id) ?? 0) + item.quantity);
	}

	const used = usedQuantityByItem(excludedBundleId);
	for (const [itemId, quantity] of requested) {
		const inventoryItem = inventory.find((item) => item.id === itemId);
		if (!inventoryItem) return false;
		if (quantity > inventoryItem.quantity - (used.get(itemId) ?? 0)) return false;
	}

	return true;
}

async function handleUsers(request, response) {
	if (request.method === 'GET') {
		sendJson(response, 200, users);
		return;
	}

	const payload = await readJson(request);

	if (request.method === 'POST') {
		const createdUsers = [];

		for (const user of payload) {
			const createdUser = { id: String(nextUserId++), name: user.name, role: user.role };
			users.push(createdUser);
			createdUsers.push(createdUser);
		}

		sendJson(response, 201, createdUsers);
		return;
	}

	if (request.method === 'DELETE') {
		for (const user of payload) {
			const index = users.findIndex((entry) => entry.id === user.id && entry.name === user.name);
			if (index === -1) {
				sendJson(response, 404, { message: 'User not found' });
				return;
			}

			users.splice(index, 1);
		}

		sendEmpty(response);
		return;
	}

	sendEmpty(response, 405);
}

async function handleInventory(request, response) {
	if (request.method === 'GET') {
		sendJson(response, 200, listAvailableInventory());
		return;
	}

	const payload = await readJson(request);

	if (request.method === 'POST') {
		const createdItems = [];

		for (const item of payload) {
			if (inventory.some((entry) => entry.item === item.item)) {
				sendJson(response, 409, { message: 'Inventory item already exists' });
				return;
			}

			const createdItem = { id: nextInventoryId++, ...item };
			inventory.push(createdItem);
			createdItems.push(createdItem);
		}

		sendJson(response, 201, createdItems);
		return;
	}

	if (request.method === 'PUT') {
		const updatedItems = [];

		for (const item of payload) {
			const existing = inventory.find((entry) => entry.item === item.item);
			if (!existing) {
				sendJson(response, 404, { message: 'Inventory item not found' });
				return;
			}

			existing.quantity = item.quantity;
			existing.quantity_type = item.quantity_type;
			updatedItems.push({ ...existing });
		}

		sendJson(response, 200, updatedItems);
		return;
	}

	if (request.method === 'DELETE') {
		for (const itemName of payload) {
			const index = inventory.findIndex((entry) => entry.item === itemName);
			if (index === -1) {
				sendJson(response, 404, { message: 'Inventory item not found' });
				return;
			}

			inventory.splice(index, 1);
		}

		sendEmpty(response);
		return;
	}

	sendEmpty(response, 405);
}

async function handleBundle(request, response, url) {
	if (request.method === 'GET') {
		const bundle = bundles.find((entry) => entry.id === Number(url.searchParams.get('id')));
		if (!bundle) {
			sendJson(response, 404, { message: 'Bundle not found' });
			return;
		}

		sendJson(response, 200, bundleResponse(bundle));
		return;
	}

	if (request.method === 'DELETE') {
		const index = bundles.findIndex((entry) => entry.id === Number(url.searchParams.get('id')));
		if (index === -1) {
			sendJson(response, 404, { message: 'Bundle not found' });
			return;
		}

		bundles.splice(index, 1);
		sendEmpty(response);
		return;
	}

	const payload = await readJson(request);

	if (request.method === 'POST') {
		if (!validateBundleItems(payload.items)) {
			sendJson(response, 409, { message: 'Bundle quantities exceed available inventory' });
			return;
		}

		const bundle = {
			id: nextBundleId++,
			user: payload.user,
			items: payload.items,
			created_at: new Date('2026-05-28T12:00:00.000Z').toISOString(),
			bundled: Boolean(payload.bundled),
			fulfilled_at: null
		};
		bundles.push(bundle);
		sendJson(response, 201, bundleResponse(bundle));
		return;
	}

	if (request.method === 'PUT') {
		const bundle = bundles.find((entry) => entry.id === payload.id);
		if (!bundle) {
			sendJson(response, 404, { message: 'Bundle not found' });
			return;
		}

		if (!validateBundleItems(payload.items, bundle.id)) {
			sendJson(response, 409, { message: 'Bundle quantities exceed available inventory' });
			return;
		}

		bundle.user = payload.user;
		bundle.bundled = payload.bundled;
		bundle.items = payload.items;
		bundle.fulfilled_at = payload.fulfilled_at;

		if (payload.fulfilled_at) {
			for (const item of payload.items) {
				const inventoryItem = inventory.find((entry) => entry.id === item.item_id);
				if (inventoryItem) inventoryItem.quantity -= item.quantity;
			}
		}

		sendJson(response, 200, bundleResponse(bundle));
		return;
	}

	sendEmpty(response, 405);
}

resetState();

const server = http.createServer(async (request, response) => {
	try {
		const url = new URL(request.url ?? '/', `http://${request.headers.host}`);

		if (url.pathname === '/__health') {
			sendJson(response, 200, { ok: true });
			return;
		}

		if (url.pathname === '/__reset' && request.method === 'POST') {
			resetState();
			sendJson(response, 200, { ok: true });
			return;
		}

		if (url.pathname === '/v1/users') {
			await handleUsers(request, response);
			return;
		}

		if (url.pathname === '/v1/inventory') {
			await handleInventory(request, response);
			return;
		}

		if (url.pathname === '/v1/bundles' && request.method === 'GET') {
			sendJson(response, 200, openBundles().map(bundleResponse));
			return;
		}

		if (url.pathname === '/v1/bundle') {
			await handleBundle(request, response, url);
			return;
		}

		sendJson(response, 404, { message: 'Not found' });
	} catch (error) {
		sendJson(response, 500, {
			message: error instanceof Error ? error.message : 'Unexpected mock backend error'
		});
	}
});

server.listen(port, host, () => {
	console.log(`mock backend listening on http://${host}:${port}`);
});
