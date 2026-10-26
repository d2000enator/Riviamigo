-- Personal assistant extension v0.3.0. Additive, backed up with the riviamigo
-- schema, and versioned separately from the preserved upstream SQLx ledger.
-- Older application images ignore these tables; rollback never deletes them.
CREATE TABLE riviamigo.personal_assistant_schema (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    version integer NOT NULL CHECK (version > 0)
);
CREATE TABLE riviamigo.personal_assistant_documents (
    user_id uuid NOT NULL REFERENCES riviamigo.users(id) ON DELETE CASCADE,
    vehicle_id uuid NOT NULL REFERENCES riviamigo.vehicles(id) ON DELETE CASCADE,
    revision bigint NOT NULL DEFAULT 1 CHECK (revision > 0),
    document jsonb NOT NULL,
    observation jsonb NOT NULL DEFAULT '{}'::jsonb,
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, vehicle_id)
);
CREATE TABLE riviamigo.personal_assistant_alerts (
    id uuid PRIMARY KEY,
    user_id uuid NOT NULL REFERENCES riviamigo.users(id) ON DELETE CASCADE,
    vehicle_id uuid NOT NULL REFERENCES riviamigo.vehicles(id) ON DELETE CASCADE,
    rule_key text NOT NULL,
    severity text NOT NULL CHECK (severity IN ('info','warning','attention')),
    title text NOT NULL,
    message text NOT NULL,
    is_demo boolean NOT NULL,
    first_seen_at timestamptz NOT NULL DEFAULT now(),
    last_seen_at timestamptz NOT NULL DEFAULT now(),
    acknowledged_at timestamptz,
    resolved_at timestamptz
);
CREATE UNIQUE INDEX personal_assistant_one_active_rule
    ON riviamigo.personal_assistant_alerts(user_id, vehicle_id, rule_key)
    WHERE resolved_at IS NULL;
CREATE INDEX personal_assistant_recent_alerts
    ON riviamigo.personal_assistant_alerts(user_id, vehicle_id, first_seen_at DESC);
CREATE TABLE riviamigo.personal_assistant_receipts (
    id uuid PRIMARY KEY,
    user_id uuid NOT NULL REFERENCES riviamigo.users(id) ON DELETE CASCADE,
    vehicle_id uuid NOT NULL REFERENCES riviamigo.vehicles(id) ON DELETE CASCADE,
    entry_id uuid NOT NULL,
    filename text NOT NULL,
    mime_type text NOT NULL CHECK (mime_type IN ('application/pdf','image/jpeg','image/png')),
    payload bytea NOT NULL CHECK (octet_length(payload) > 0 AND octet_length(payload) <= 5242880),
    sha256 bytea NOT NULL CHECK (octet_length(sha256) = 32),
    created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX personal_assistant_receipts_by_vehicle
    ON riviamigo.personal_assistant_receipts(user_id, vehicle_id);
INSERT INTO riviamigo.personal_assistant_schema(singleton, version) VALUES (true, 1);
