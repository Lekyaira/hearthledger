import { page } from 'vitest/browser';
import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import InventoryList from './InventoryList.svelte';

describe('InventoryList.svelte', () => {
	it('renders inventory items with quantities', async () => {
		render(InventoryList, {
			items: [
				{ item: 'Canned tomatoes', quantity: 24, quantity_type: 'count' },
				{ item: 'Flour', quantity: 4.5, quantity_type: 'pounds' }
			]
		});

		await expect.element(page.getByRole('heading', { level: 1 })).toHaveTextContent('Inventory');
		await expect.element(page.getByText('Canned tomatoes')).toBeInTheDocument();
		await expect.element(page.getByText('24 count')).toBeInTheDocument();
		await expect.element(page.getByText('Flour')).toBeInTheDocument();
		await expect.element(page.getByText('4.5 lb')).toBeInTheDocument();
	});

	it('renders an empty state', async () => {
		render(InventoryList, { items: [] });

		await expect.element(page.getByText('No inventory items yet.')).toBeInTheDocument();
	});
});
