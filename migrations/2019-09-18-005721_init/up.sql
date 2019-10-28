CREATE TABLE files (
    id              INTEGER NOT NULL PRIMARY KEY,
    kind            INTEGER NOT NULL DEFAULT 0,
    path            TEXT NOT NULL UNIQUE
);

CREATE TABLE tags (
    id              INTEGER NOT NULL PRIMARY KEY,
    name            TEXT NOT NULL UNIQUE
);

CREATE TABLE file_tags (
    file_id         INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    tag_id          INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY     (file_id, tag_id)
);
