import { beforeEach, describe, expect, it, vi } from 'vitest';
import { load } from './+page.server';

describe('+page.server load', () => {
	beforeEach(() => {
		vi.restoreAllMocks();
	});

	it('does not load inventory before the client session redirect', async () => {
		const fetch = vi.fn();

		const result = await load();

		expect(fetch).not.toHaveBeenCalled();
		expect(result).toEqual({});
	});
});
