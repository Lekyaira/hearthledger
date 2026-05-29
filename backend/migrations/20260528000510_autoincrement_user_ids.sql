PRAGMA foreign_keys = OFF;

CREATE TABLE user_id_map (
    old_id TEXT PRIMARY KEY,
    new_id INTEGER NOT NULL UNIQUE
);

INSERT INTO user_id_map (old_id, new_id)
SELECT id, ROW_NUMBER() OVER (ORDER BY rowid)
FROM users;

CREATE TABLE users_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'member' CHECK (role IN ('member', 'admin'))
);

INSERT INTO users_new (id, name, role)
SELECT user_id_map.new_id, users.name, users.role
FROM users
JOIN user_id_map ON user_id_map.old_id = users.id
ORDER BY user_id_map.new_id;

CREATE TABLE bundles_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    bundled INTEGER NOT NULL DEFAULT 0 CHECK (bundled IN (0, 1)),
    fulfilled_at TEXT,
    FOREIGN KEY (user) REFERENCES users_new(id)
);

INSERT INTO bundles_new (id, user, created_at, bundled, fulfilled_at)
SELECT bundles.id, user_id_map.new_id, bundles.created_at, bundles.bundled, bundles.fulfilled_at
FROM bundles
JOIN user_id_map ON user_id_map.old_id = bundles.user;

DROP TABLE bundles;
ALTER TABLE bundles_new RENAME TO bundles;

DROP TABLE users;
ALTER TABLE users_new RENAME TO users;

DROP TABLE user_id_map;

PRAGMA foreign_keys = ON;
