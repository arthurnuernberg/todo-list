CREATE TABLE IF NOT EXISTS todos
(
    id          VARCHAR(36) PRIMARY KEY,
    list_id     TEXT        NOT NULL REFERENCES todo_lists (id) ON DELETE CASCADE,
    title       TEXT        NOT NULL,
    description TEXT,
    due_date    TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed   BOOLEAN     NOT NULL DEFAULT FALSE,
    is_overdue  BOOLEAN     NOT NULL DEFAULT FALSE
);

CREATE TABLE IF NOT EXISTS tags
(
    id         VARCHAR(36) PRIMARY KEY UNIQUE,
    name       TEXT        NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS todo_tags
(
    todo_id VARCHAR(36) REFERENCES todos (id) ON DELETE CASCADE,
    tag_id  VARCHAR(36) REFERENCES tags (id) ON DELETE CASCADE,
    PRIMARY KEY (todo_id, tag_id)
);

CREATE TABLE IF NOT EXISTS todo_lists
(
    id    TEXT PRIMARY KEY UNIQUE,
    title TEXT NOT NULL
);

-- Indizes für bessere Performance
CREATE INDEX IF NOT EXISTS idx_todos_due_date ON todos (due_date);
CREATE INDEX IF NOT EXISTS idx_todos_completed ON todos (completed);
CREATE INDEX IF NOT EXISTS idx_todo_tags_tag_id ON todo_tags (tag_id);
CREATE UNIQUE INDEX tags_name_lower_unique ON tags (LOWER(TRIM(name)));