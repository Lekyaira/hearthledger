import { expect, type Locator, type Page, test } from '@playwright/test';

const backendOrigin = 'http://127.0.0.1:3100';
const userStorageKey = 'hearthledger.user.id';

async function resetBackend(page: Page) {
	await page.request.post(`${backendOrigin}/__reset`);
}

async function setCurrentUser(page: Page, userId: string) {
	await page.addInitScript(
		({ key, id }) => {
			sessionStorage.setItem(key, id);
		},
		{ key: userStorageKey, id: userId }
	);
}

async function loginAs(page: Page, userName: string) {
	await page.goto('/login');
	await page.getByLabel('User').selectOption({ label: userName });
	await page.getByRole('button', { name: 'login' }).click();
	await expect(page).toHaveURL('/inventory');
}

async function inventoryRow(page: Page, itemName: string): Promise<Locator> {
	const rows = page.getByRole('list', { name: 'Inventory items' }).getByRole('listitem');

	await expect
		.poll(async () => {
			const values = await inventoryItemNames(rows);
			return values.includes(itemName);
		})
		.toBe(true);

	const values = await inventoryItemNames(rows);
	const index = values.indexOf(itemName);

	return rows.nth(index);
}

async function inventoryItemNames(rows: Locator) {
	return rows.evaluateAll((rowElements) =>
		rowElements.map((row) => row.querySelector<HTMLInputElement>('input[type="text"]')?.value ?? '')
	);
}

async function replaceInputValue(input: Locator, value: string) {
	await input.click();
	await input.press(process.platform === 'darwin' ? 'Meta+A' : 'Control+A');
	await input.pressSequentially(value);
	await expect(input).toHaveValue(value);
}

test.beforeEach(async ({ page }) => {
	await resetBackend(page);
});

test('root sends guests to login and login/logout updates the session navigation', async ({
	page
}) => {
	await page.goto('/');
	await expect(page).toHaveURL('/login');

	await loginAs(page, 'Test Admin');

	await expect(page.getByRole('button', { name: 'Inventory', exact: true })).toBeVisible();
	await expect(page.getByRole('button', { name: 'Pending bundles' })).toBeVisible();
	await expect(page.getByRole('button', { name: 'Users' })).toBeVisible();
	await expect(page.getByRole('button', { name: 'Log out' })).toBeVisible();

	await page.getByRole('button', { name: 'Log out' }).click();

	await expect(page).toHaveURL('/login');
	await expect(page.getByRole('button', { name: 'Log in' })).toBeVisible();
});

test('members see available inventory and can request a pending bundle', async ({ page }) => {
	await loginAs(page, 'Test Member');

	await expect(page.getByRole('button', { name: 'Users' })).not.toBeVisible();
	await expect(page.getByText('Canned tomatoes')).toBeVisible();
	await expect(page.getByText('24 count')).toBeVisible();
	await expect(page.getByRole('button', { name: 'request' })).toBeDisabled();

	await page.getByRole('spinbutton', { name: 'Request quantity for Canned tomatoes' }).fill('2');
	await page.getByRole('button', { name: 'request' }).click();

	await expect(page.getByRole('button', { name: 'request' })).toBeDisabled();
	await expect(page.getByLabel('1 open bundles')).toBeVisible();

	await page.getByRole('button', { name: 'Pending bundles' }).click();

	await expect(page).toHaveURL('/bundles');
	await expect(page.getByText('Bundle 1')).toBeVisible();
	await expect(page.getByRole('button', { name: 'Expand bundle 1' })).toBeVisible();
	await expect(page.getByRole('button', { name: 'Mark bundle 1 complete' })).not.toBeVisible();
});

test('admins can add and remove users', async ({ page }) => {
	await setCurrentUser(page, '2');
	await page.goto('/users');

	await expect(page.getByRole('heading', { name: 'Users' })).toBeVisible();
	await expect(page.getByText('Test Member')).toBeVisible();

	await page.getByRole('textbox', { name: 'Name' }).fill('Kitchen Lead');
	await page.getByLabel('Role').selectOption('admin');
	await page.getByRole('button', { name: 'add' }).click();

	await expect(page.getByText('Kitchen Lead')).toBeVisible();
	await expect(page.getByText('4')).toBeVisible();
	await expect(page.getByRole('button', { name: 'Remove Kitchen Lead' })).toBeVisible();

	await page.getByRole('button', { name: 'Remove Kitchen Lead' }).click();
	await expect(page.getByText('Kitchen Lead')).not.toBeVisible();
});

test('member request validation prevents quantities above available inventory', async ({
	page
}) => {
	await loginAs(page, 'Test Member');

	await page.getByRole('spinbutton', { name: 'Request quantity for Canned tomatoes' }).fill('25');
	await page.getByRole('button', { name: 'request' }).click();

	await expect(
		page.getByText('Requested quantities cannot exceed available inventory.')
	).toBeVisible();
	await expect(page.getByLabel('1 open bundles')).not.toBeVisible();
});

test('admins can create, update, cancel, and delete inventory rows', async ({ page }) => {
	await setCurrentUser(page, '2');
	await page.goto('/inventory');

	await expect(page.getByRole('heading', { name: 'Inventory' })).toBeVisible();
	await expect(await inventoryRow(page, 'Canned tomatoes')).toBeVisible();

	await page.getByRole('button', { name: 'Add inventory line' }).click();
	await page.getByRole('textbox', { name: 'Item' }).last().fill('Brown rice');
	await page.getByRole('spinbutton', { name: 'Quantity' }).last().fill('7.5');
	await page.getByLabel('Quantity type').last().selectOption('pounds');
	await page.getByRole('button', { name: 'update' }).click();

	const savedBrownRiceRow = await inventoryRow(page, 'Brown rice');
	await expect(savedBrownRiceRow.getByRole('spinbutton', { name: 'Quantity' })).toHaveValue('7.5');

	await replaceInputValue(savedBrownRiceRow.getByRole('spinbutton', { name: 'Quantity' }), '8');
	await expect(savedBrownRiceRow.getByLabel('Pending edit')).toBeVisible();
	await page.getByRole('button', { name: 'cancel changes' }).click();
	const resetBrownRiceRow = await inventoryRow(page, 'Brown rice');
	await expect(resetBrownRiceRow.getByRole('spinbutton', { name: 'Quantity' })).toHaveValue('7.5');

	await replaceInputValue(resetBrownRiceRow.getByRole('spinbutton', { name: 'Quantity' }), '8');
	await expect(resetBrownRiceRow.getByLabel('Pending edit')).toBeVisible();
	await page.getByRole('button', { name: 'update' }).click();
	const updatedBrownRiceRow = await inventoryRow(page, 'Brown rice');
	await expect(updatedBrownRiceRow.getByRole('spinbutton', { name: 'Quantity' })).toHaveValue('8');

	await page.getByRole('button', { name: 'Delete Brown rice' }).click();
	await page.getByRole('button', { name: 'update' }).click();

	const rows = page.getByRole('list', { name: 'Inventory items' }).getByRole('listitem');
	await expect
		.poll(async () => {
			const values = await inventoryItemNames(rows);
			return values.includes('Brown rice');
		})
		.toBe(false);
});

test('admins can fulfill pending bundles and committed quantities remain reflected in inventory', async ({
	page
}) => {
	await loginAs(page, 'Test Member');
	await page.getByRole('spinbutton', { name: 'Request quantity for Canned tomatoes' }).fill('2');
	await page.getByRole('button', { name: 'request' }).click();
	await expect(page.getByLabel('1 open bundles')).toBeVisible();

	await page.getByRole('button', { name: 'Log out' }).click();
	await loginAs(page, 'Test Admin');
	await page.getByRole('button', { name: 'Pending bundles' }).click();

	await page.getByRole('button', { name: 'Expand bundle 1' }).click();
	await page.getByRole('spinbutton', { name: 'Quantity for Canned tomatoes' }).fill('1');
	await page.getByRole('button', { name: 'Mark bundle 1 complete' }).click();
	await expect(page.getByText('completion pending update')).toBeVisible();

	await page.getByRole('button', { name: 'update' }).click();

	await expect(page.getByText('No pending bundles.')).toBeVisible();
	await page.getByRole('button', { name: 'Inventory', exact: true }).click();
	const tomatoesRow = await inventoryRow(page, 'Canned tomatoes');
	await expect(tomatoesRow.getByRole('spinbutton', { name: 'Quantity' })).toHaveValue('23');
});
