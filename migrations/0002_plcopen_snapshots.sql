-- Snapshots PLCopenXML associés à un commit stu-vcs (hash SHA-256).
-- L'agent uploade le XML après export depuis Control Expert (via UDE ou manuel).
CREATE TABLE plcopen_snapshots (
    id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    commit_hash TEXT        NOT NULL UNIQUE,
    xml_content TEXT        NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
