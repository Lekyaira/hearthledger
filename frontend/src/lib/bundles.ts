export type BundledItem = {
	item_id: number;
	item: string;
	quantity: number;
};

export type Bundle = {
	id: number;
	user: string;
	items: BundledItem[];
	created_at: string;
	bundled: boolean;
	fulfilled_at?: string | null;
};

export type NewBundle = {
	user: string;
	bundled?: boolean;
	items: Array<Pick<BundledItem, 'item_id' | 'quantity'>>;
};

export type UpdatedBundle = {
	id: number;
	user: string;
	bundled: boolean;
	fulfilled_at: string | null;
	items: Array<Pick<BundledItem, 'item_id' | 'quantity'>>;
};

export const requestStorageKey = 'hearthledger.bundles.request.pending.v1';
export const pendingBundlesStorageKey = 'hearthledger.bundles.pending.v1';
export { readCurrentUserId } from '$lib/auth';
