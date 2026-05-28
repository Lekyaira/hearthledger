CREATE TABLE bundles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    bundled INTEGER NOT NULL DEFAULT 0 CHECK (bundled IN (0, 1)),
    fulfilled_at TEXT,
    FOREIGN KEY (user) REFERENCES users(id)
);

CREATE TABLE bundled_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    bundle_id INTEGER NOT NULL,
    item_id INTEGER NOT NULL,
    quantity REAL NOT NULL CHECK (quantity > 0),
    FOREIGN KEY (bundle_id) REFERENCES bundles(id) ON DELETE CASCADE,
    FOREIGN KEY (item_id) REFERENCES inventory(id),
    UNIQUE (bundle_id, item_id)
);
