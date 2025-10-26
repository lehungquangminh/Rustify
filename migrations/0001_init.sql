CREATE TABLE IF NOT EXISTS links (
  alias text PRIMARY KEY,
  url text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS clicks (
  id bigserial PRIMARY KEY,
  alias text NOT NULL REFERENCES links(alias) ON DELETE CASCADE,
  ts timestamptz NOT NULL,
  n int NOT NULL DEFAULT 1
);

CREATE INDEX IF NOT EXISTS idx_clicks_alias_ts ON clicks(alias, ts);
