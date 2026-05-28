import { beforeEach, describe, expect, it, vi } from 'vitest';
import { listInventory } from '$lib/server/db';
import { load } from './+page.server';

vi.mock('$lib/server/db', () => ({
	listInventory: vi.fn()
}));

const mockedListInventory = vi.mocked(listInventory);

describe('+page.server load', () => {
	beforeEach(() => {
		mockedListInventory.mockReset();
	});

	it('loads inventory through the server data module', async () => {
		const inventory = [{ item: 'Canned tomatoes', quantity: 24, quantity_type: 'count' as const }];
		const fetch = vi.fn();
		mockedListInventory.mockResolvedValue(inventory);

		const result = await load({ fetch } as unknown as Parameters<typeof load>[0]);

		expect(mockedListInventory).toHaveBeenCalledWith(fetch);
		expect(result).toEqual({ inventory });
	});
});
