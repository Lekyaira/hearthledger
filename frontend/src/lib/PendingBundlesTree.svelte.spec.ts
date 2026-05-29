import { page } from 'vitest/browser';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import PendingBundlesTree from './PendingBundlesTree.svelte';

describe('PendingBundlesTree.svelte', () => {
	beforeEach(() => {
		sessionStorage.clear();
		vi.restoreAllMocks();
	});

	it('lets admins stage and commit completions for all pending bundles', async () => {
		sessionStorage.setItem('hearthledger.user.id', 'admin');

		const fetchMock = vi.spyOn(window, 'fetch').mockImplementation(async (input, init) => {
			const url = String(input);

			if (url === '/api/users') {
				return Response.json([{ id: 'admin', name: 'Admin', role: 'admin' }]);
			}

			if (url === '/api/bundles') {
				return Response.json([
					{
						id: 1,
						user: 'member-a',
						items: [{ item_id: 10, item: 'Rice', quantity: 2 }],
						created_at: '2026-05-28T12:00:00Z',
						bundled: false,
						fulfilled_at: null
					},
					{
						id: 2,
						user: 'member-b',
						items: [{ item_id: 20, item: 'Beans', quantity: 3 }],
						created_at: '2026-05-28T13:00:00Z',
						bundled: true,
						fulfilled_at: null
					}
				]);
			}

			if (url === '/api/bundle' && init?.method === 'PUT') {
				return Response.json(JSON.parse(String(init.body)));
			}

			return new Response(null, { status: 404 });
		});

		render(PendingBundlesTree, { apiPath: '/api/bundle' });

		await expect.element(page.getByText('Bundle 1')).toBeInTheDocument();
		await expect.element(page.getByText('Bundle 2')).toBeInTheDocument();

		await page.getByRole('button', { name: 'Expand bundle 1' }).click();
		await page.getByRole('spinbutton', { name: 'Quantity for Rice' }).fill('5');
		await page.getByRole('button', { name: 'Mark bundle 1 complete' }).click();

		await expect.element(page.getByText('completion pending update')).toBeInTheDocument();
		await expect.element(page.getByRole('button', { name: 'Keep bundle 1 pending' })).toBeEnabled();

		await page.getByRole('button', { name: 'update' }).click();

		await vi.waitFor(() => {
			const updateCall = fetchMock.mock.calls.find(
				([input, init]) => String(input) === '/api/bundle' && init?.method === 'PUT'
			);
			expect(updateCall).toBeDefined();
			const body = JSON.parse(String(updateCall?.[1]?.body));
			expect(body.id).toBe(1);
			expect(body.fulfilled_at).toEqual(expect.any(String));
			expect(body.items).toEqual([{ item_id: 10, quantity: 5 }]);
		});
	});

	it('shows an error if quantity-only changes bypass the disabled update button', async () => {
		sessionStorage.setItem('hearthledger.user.id', 'admin');

		const fetchMock = vi.spyOn(window, 'fetch').mockImplementation(async (input) => {
			const url = String(input);

			if (url === '/api/users') {
				return Response.json([{ id: 'admin', name: 'Admin', role: 'admin' }]);
			}

			if (url === '/api/bundles') {
				return Response.json([
					{
						id: 1,
						user: 'member-a',
						items: [{ item_id: 10, item: 'Rice', quantity: 2 }],
						created_at: '2026-05-28T12:00:00Z',
						bundled: false,
						fulfilled_at: null
					}
				]);
			}

			return new Response(null, { status: 404 });
		});

		render(PendingBundlesTree, { apiPath: '/api/bundle' });

		await expect.element(page.getByText('Bundle 1')).toBeInTheDocument();
		await page.getByRole('button', { name: 'Expand bundle 1' }).click();

		const quantityInput = page.getByRole('spinbutton', { name: 'Quantity for Rice' });
		await quantityInput.fill('5');
		await expect.element(quantityInput).toHaveValue(5);

		const updateButton = page.getByRole('button', { name: 'update' });
		await expect.element(updateButton).toBeDisabled();

		const updateButtonElement = (await updateButton.findElement()) as HTMLButtonElement;
		updateButtonElement.disabled = false;
		updateButtonElement.click();

		await expect
			.element(page.getByText('Mark the bundle complete before updating quantity changes.'))
			.toBeInTheDocument();
		expect(
			fetchMock.mock.calls.some(
				([input, init]) => String(input) === '/api/bundle' && init?.method === 'PUT'
			)
		).toBe(false);
	});

	it('does not commit admin quantity edits when another pending change enables update', async () => {
		sessionStorage.setItem('hearthledger.user.id', 'admin');

		const fetchMock = vi.spyOn(window, 'fetch').mockImplementation(async (input, init) => {
			const url = String(input);

			if (url === '/api/users') {
				return Response.json([{ id: 'admin', name: 'Admin', role: 'admin' }]);
			}

			if (url === '/api/bundles') {
				return Response.json([
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
					}
				]);
			}

			if (url === '/api/bundle' && init?.method === 'PUT') {
				return Response.json(JSON.parse(String(init.body)));
			}

			return new Response(null, { status: 404 });
		});

		render(PendingBundlesTree, { apiPath: '/api/bundle' });

		await expect.element(page.getByText('Bundle 1')).toBeInTheDocument();
		await page.getByRole('button', { name: 'Expand bundle 1' }).click();
		await page.getByRole('spinbutton', { name: 'Quantity for Rice' }).fill('5');
		await page.getByRole('button', { name: 'Delete Beans from bundle 1' }).click();
		await page.getByRole('button', { name: 'update' }).click();

		await vi.waitFor(() => {
			const updateCall = fetchMock.mock.calls.find(
				([input, init]) => String(input) === '/api/bundle' && init?.method === 'PUT'
			);
			expect(updateCall).toBeDefined();
			const body = JSON.parse(String(updateCall?.[1]?.body));
			expect(body.fulfilled_at).toBeNull();
			expect(body.items).toEqual([{ item_id: 10, quantity: 2 }]);
		});
	});
});
