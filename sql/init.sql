-- StreamInd Database Initialization

CREATE TABLE IF NOT EXISTS agents (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    memory TEXT DEFAULT '',
    goal TEXT DEFAULT '',
    create_time TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    stream_id INTEGER NOT NULL,
    event_type TEXT NOT NULL,
    content TEXT NOT NULL,
    response TEXT DEFAULT '',
    source_event_id INTEGER,
    time TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS rule_groups (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    create_time TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS trigger_rules (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    group_id INTEGER NOT NULL,
    expr TEXT NOT NULL,
    hook TEXT NOT NULL,
    create_time TEXT NOT NULL,
    trigger_times INTEGER DEFAULT 0,
    expire_time TEXT NOT NULL,
    FOREIGN KEY (group_id) REFERENCES rule_groups(id)
);

CREATE INDEX IF NOT EXISTS idx_events_stream ON events(stream_id);
CREATE INDEX IF NOT EXISTS idx_rules_group ON trigger_rules(group_id);
CREATE INDEX IF NOT EXISTS idx_rules_expire ON trigger_rules(expire_time);