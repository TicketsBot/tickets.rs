CREATE TABLE IF NOT EXISTS patreon_keys (
    "client_id" VARCHAR(255) NOT NULL,
    "access_token" VARCHAR(255) NOT NULL,
    "refresh_token" VARCHAR(255) NOT NULL,
    "expires" TIMESTAMPTZ NOT NULL,
    PRIMARY KEY("client_id")
);
