import { expect, type Page, test } from '@playwright/test';

type Bundle = {
	id: number;
	user: string;
	items: Array<{ item_id: number; item: string; quantity: number }>;
	created_at: string;
	bundled: boolean;
	fulfilled_at: string | null;
};

const users = [
	{ id: 'admin', name: 'Admin', role: 'admin' },
	{ id: 'member-a', name: 'Member A', role: 'member' },
	{ id: 'member-b', name: 'Member B', role: 'member' }
];

function createBundles(): Bundle[] {
	return [
		{
			id: 1,
			user: 'member-a',
			items: [
				{ item_id: 10, item: 'Rice', quantity: 2 },
				{ item_id: 20, item: 'Beans', quantity: 3 }
			],
			created_at: '2026-05-28T12:00:00Z',
			bundled: false,
			fulfilled_at: null
		},
		{
			id: 2,
			user: 'member-b',
			items: [{ item_id: 30, item: 'Flour', quantity: 4 }],
			created_at: '2026-05-28T13:00:00Z',
			bundled: true,
			fulfilled_at: null
		}
	];
}

async function openBundlesPage(page: Page, userId: string, bundles = createBundles()) {
	const updates: unknown[] = [];

	await page.addInitScript((id) => {
		sessionStorage.setItem('hearthledger.user.id', id);
	}, userId);

	await page.route('**/api/users', async (route) => {
		await route.fulfill({ json: users });
	});

	await page.route('**/api/bundles', async (route) => {
		await route.fulfill({ json: bundles.filter((bundle) => bundle.fulfilled_at === null) });
	});

	await page.route('**/api/bundle', async (route) => {
		if (route.request().method() !== 'PUT') {
			await route.fulfill({ status: 204 });
			return;
		}

		const payload = route.request().postDataJSON();
		updates.push(payload);

		const bundle = bundles.find((entry) => entry.id === payload.id);
		if (bundle) {
			bundle.bundled = payload.bundled;
			bundle.fulfilled_at = payload.fulfilled_at;
			bundle.items = payload.items.map((item: { item_id: number; quantity: number }) => {
				const existing = bundle.items.find((entry) => entry.item_id === item.item_id);
				return {
					item_id: item.item_id,
					item: existing?.item ?? `Item ${item.item_id}`,
					quantity: item.quantity
				};
			});
		}

		await route.fulfill({ json: payload });
	});

	await page.goto('/bundles');
	await expect(page.getByRole('heading', { name: 'Pending bundles' })).toBeVisible();

	return { updates };
}

test('admin users see all pending bundles and members see only their own', async ({ page }) => {
	await openBundlesPage(page, 'admin');

	await expect(page.getByText('Bundle 1')).toBeVisible();
	await expect(page.getByText('Bundle 2')).toBeVisible();
	await expect(page.getByLabel('2 open bundles')).toBeVisible();

	const memberPage = await page.context().newPage();
	await openBundlesPage(memberPage, 'member-a');

	await expect(memberPage.getByText('Bundle 1')).toBeVisible();
	await expect(memberPage.getByText('Bundle 2')).not.toBeVisible();
	await expect(memberPage.getByLabel('1 open bundles')).toBeVisible();

	const readyMemberPage = await page.context().newPage();
	await openBundlesPage(readyMemberPage, 'member-b');

	await expect(readyMemberPage.getByLabel('1 bundles ready for pickup')).toBeVisible();
});

test('admin ready status commits edited quantities without fulfilling the bundle', async ({
	page
}) => {
	const { updates } = await openBundlesPage(page, 'admin');

	await page.getByRole('button', { name: 'Expand bundle 1' }).click();
	await page.getByRole('spinbutton', { name: 'Quantity for Rice' }).fill('5');
	await page.getByRole('button', { name: 'Mark bundle 1 ready for pickup' }).click();
	await expect(page.getByText('ready status pending update')).toBeVisible();

	await page.getByRole('button', { name: 'update' }).click();

	await expect.poll(() => updates.length).toBe(1);
	expect(updates[0]).toMatchObject({
		id: 1,
		bundled: true,
		fulfilled_at: null,
		items: [
			{ item_id: 10, quantity: 5 },
			{ item_id: 20, quantity: 3 }
		]
	});
	await expect(page.getByText('Bundle 1')).toBeVisible();
});

test('members confirm pickup of read-only ready bundles', async ({ page }) => {
	const { updates } = await openBundlesPage(page, 'member-b');

	await page.getByRole('button', { name: 'Expand bundle 2' }).click();
	await expect(page.getByRole('spinbutton', { name: 'Quantity for Flour' })).toBeDisabled();
	await page.getByRole('button', { name: 'Mark bundle 2 picked up' }).click();
	await expect(page.getByText('pickup pending update')).toBeVisible();
	await page.getByRole('button', { name: 'update' }).click();

	await expect.poll(() => updates.length).toBe(1);
	expect(updates[0]).toMatchObject({
		id: 2,
		bundled: true,
		fulfilled_at: expect.any(String)
	});
	await expect(page.getByText('Bundle 2')).not.toBeVisible();
});

test('admin quantity-only changes cannot be saved without completion', async ({ page }) => {
	const { updates } = await openBundlesPage(page, 'admin');

	await page.getByRole('button', { name: 'Expand bundle 1' }).click();
	await page.getByRole('spinbutton', { name: 'Quantity for Rice' }).fill('5');
	await expect(page.getByRole('button', { name: 'update' })).toBeDisabled();

	await page.getByRole('button', { name: 'update' }).evaluate((button) => {
		(button as HTMLButtonElement).disabled = false;
		(button as HTMLButtonElement).click();
	});

	await expect(
		page.getByText('Mark the bundle complete before updating quantity changes.')
	).toBeVisible();
	expect(updates).toHaveLength(0);
});

test('uncompleted updates keep original quantities when another change enables save', async ({
	page
}) => {
	const { updates } = await openBundlesPage(page, 'admin');

	await page.getByRole('button', { name: 'Expand bundle 1' }).click();
	await page.getByRole('spinbutton', { name: 'Quantity for Rice' }).fill('5');
	await page.getByRole('button', { name: 'Delete Beans from bundle 1' }).click();
	await page.getByRole('button', { name: 'update' }).click();

	await expect.poll(() => updates.length).toBe(1);
	expect(updates[0]).toMatchObject({
		id: 1,
		fulfilled_at: null,
		items: [{ item_id: 10, quantity: 2 }]
	});
});
