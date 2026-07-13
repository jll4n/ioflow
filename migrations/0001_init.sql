CREATE TABLE organizations (
    id         UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    name       TEXT        NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE users (
    id            UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id        UUID        NOT NULL REFERENCES organizations(id),
    email         TEXT        NOT NULL UNIQUE,
    password_hash TEXT        NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE projects (
    id         UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id     UUID        NOT NULL REFERENCES organizations(id),
    name       TEXT        NOT NULL,
    git_url    TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE agents (
    id           UUID        PRIMARY KEY,
    org_id       UUID        NOT NULL REFERENCES organizations(id),
    hostname     TEXT        NOT NULL,
    version      TEXT        NOT NULL,
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE jobs (
    id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id  UUID        NOT NULL REFERENCES projects(id),
    kind        TEXT        NOT NULL DEFAULT 'build',
    status      TEXT        NOT NULL DEFAULT 'queued',
    agent_id    UUID        REFERENCES agents(id),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at  TIMESTAMPTZ,
    finished_at TIMESTAMPTZ
);

CREATE TABLE diagnostics (
    id       UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    job_id   UUID NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
    level    TEXT NOT NULL,  -- 'error' | 'warning' | 'info'
    message  TEXT NOT NULL,
    location TEXT
);
