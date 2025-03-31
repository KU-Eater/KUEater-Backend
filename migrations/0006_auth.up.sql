CREATE TABLE IF NOT EXISTS kueater.google_access_token (
    user_id UUID PRIMARY KEY REFERENCES kueater.userprofile ON DELETE CASCADE,
    token TEXT
);