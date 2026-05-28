export type QuantityType =
	| 'count'
	| 'grams'
	| 'ounces'
	| 'pounds'
	| 'liters'
	| 'milliliters'
	| 'gallons';

export type InventoryItem = {
	id?: number;
	item: string;
	quantity: number;
	quantity_type: QuantityType;
};

export const quantityTypes: QuantityType[] = [
	'count',
	'grams',
	'ounces',
	'pounds',
	'liters',
	'milliliters',
	'gallons'
];

export const quantityTypeLabels: Record<QuantityType, string> = {
	count: 'count',
	grams: 'g',
	ounces: 'oz',
	pounds: 'lb',
	liters: 'L',
	milliliters: 'mL',
	gallons: 'gal'
};
