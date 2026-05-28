CREATE TABLE inventory_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    item TEXT NOT NULL UNIQUE,
    quantity REAL NOT NULL DEFAULT 0 CHECK (quantity >= 0),
    quantity_type TEXT NOT NULL DEFAULT 'count' CHECK (
        quantity_type IN (
            'count',
            'grams',
            'ounces',
            'pounds',
            'liters',
            'milliliters',
            'gallons'
        )
    )
);

INSERT INTO inventory_new (id, item, quantity, quantity_type)
SELECT id, item, CAST(quantity AS REAL), 'count'
FROM inventory;

DROP TABLE inventory;

ALTER TABLE inventory_new RENAME TO inventory;

UPDATE inventory
SET quantity = 10.5,
    quantity_type = 'pounds'
WHERE item = 'All-purpose flour';
