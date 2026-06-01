DELETE FROM bundled_items;
DELETE FROM bundles;
DELETE FROM inventory;
DELETE FROM users;

DELETE FROM sqlite_sequence
WHERE name IN ('bundled_items', 'bundles', 'inventory', 'users');

INSERT INTO users (name, role)
VALUES ('Admin', 'admin');
