import { page } from 'vitest/browser';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import EditableInventoryList from './EditableInventoryList.svelte';

describe('EditableInventoryList.svelte', () => {
	beforeEach(() => {
		sessionStorage.clear();
		vi.restoreAllMocks();
	});

	it('renders inventory rows as editable fields', async () => {
		render(EditableInventoryList, {
			items: [
				{ item: 'Canned tomatoes', quantity: 24, quantity_type: 'count' },
				{ item: 'Flour', quantity: 4.5, quantity_type: 'pounds' }
			]
		});

		await expect.element(page.getByRole('heading', { level: 1 })).toHaveTextContent('Inventory');
		await expect
			.element(page.getByRole('textbox', { name: 'Item' }).first())
			.toHaveValue('Canned tomatoes');
		await expect
			.element(page.getByRole('spinbutton', { name: 'Quantity' }).first())
			.toHaveValue(24);
		await expect.element(page.getByRole('textbox', { name: 'Item' }).nth(1)).toHaveValue('Flour');
		await expect
			.element(page.getByRole('spinbutton', { name: 'Quantity' }).nth(1))
			.toHaveValue(4.5);
	});

	it('adds an editable inventory line', async () => {
		render(EditableInventoryList, { items: [] });

		await page.getByRole('button', { name: 'Add inventory line' }).click();

		await expect.element(page.getByRole('spinbutton', { name: 'Quantity' })).toHaveValue(0);
		await expect
			.element(page.getByRole('button', { name: 'Delete inventory line' }))
			.toBeInTheDocument();
	});

	it('changes the delete button to add when a delete is pending', async () => {
		render(EditableInventoryList, {
			items: [{ item: 'Flour', quantity: 4.5, quantity_type: 'pounds' }]
		});

		const deleteButton = page.getByRole('button', { name: 'Delete Flour' });
		await expect.element(deleteButton).toHaveTextContent('-');

		await deleteButton.click();

		await expect
			.element(page.getByRole('button', { name: 'Undelete Flour' }))
			.toHaveTextContent('+');
	});

	it('cancels back to the latest committed inventory after update', async () => {
		vi.spyOn(window, 'fetch').mockImplementation(async (_input, init) => {
			if (init?.method === 'PUT') {
				return new Response('[]', { status: 200 });
			}

			if (init?.method === 'GET' || init?.method === undefined) {
				return Response.json([{ item: 'Flour', quantity: 2, quantity_type: 'pounds' }]);
			}

			return new Response('[]', { status: 200 });
		});

		render(EditableInventoryList, {
			apiPath: '/api/inventory',
			items: [{ item: 'Flour', quantity: 1, quantity_type: 'pounds' }]
		});

		const quantityInput = page.getByRole('spinbutton', { name: 'Quantity' });
		await quantityInput.fill('2');
		await page.getByRole('button', { name: 'update' }).click();
		await expect.element(quantityInput).toHaveValue(2);
		expect(sessionStorage.getItem('hearthledger.inventory.pending.v1')).toBeNull();

		await quantityInput.fill('3');
		await page.getByRole('button', { name: 'cancel changes' }).click();

		await expect.element(quantityInput).toHaveValue(2);
	});
});
