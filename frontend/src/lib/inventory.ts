export type QuantityType =
	| 'count'
	| 'grams'
	| 'ounces'
	| 'pounds'
	| 'liters'
	| 'milliliters'
	| 'gallons';

export type InventoryItem = {
	item: string;
	quantity: number;
	quantity_type: QuantityType;
};
