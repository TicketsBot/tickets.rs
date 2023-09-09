INSERT INTO patreon_keys
VALUES ($1, $2, $3, $4)
ON CONFLICT ("client_id") DO UPDATE
    SET "access_token"  = EXCLUDED.access_token,
        "refresh_token" = EXCLUDED.refresh_token,
        "expires"       = EXCLUDED.expires