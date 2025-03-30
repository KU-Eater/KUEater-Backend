-- Add activity tally counter to user, so that we can trigger recommendations

CREATE TABLE IF NOT EXISTS kueater.activity_tally_counter (
    user_id UUID PRIMARY KEY REFERENCES kueater.userprofile ON DELETE CASCADE,
    count SMALLINT DEFAULT 0
);

-- Increments and check if count is over threshold
CREATE OR REPLACE FUNCTION tally(
    p_user_id UUID,
    threshold SMALLINT
) RETURNS BOOLEAN AS $$
DECLARE
    new_count SMALLINT;
BEGIN
    UPDATE kueater.activity_tally_counter
    SET count = count + 1
    WHERE user_id = p_user_id
    RETURNING count INTO new_count;

    IF new_count IS NULL THEN
        RETURN FALSE;   -- If null = no user
    END IF;

    RETURN new_count >= threshold;
END;
$$ LANGUAGE plpgsql;

-- Reset count for user
CREATE OR REPLACE FUNCTION reset_tally(
    p_user_id UUID
) RETURNS VOID
AS $$
BEGIN
    UPDATE kueater.activity_tally_counter
    SET count = 0
    WHERE user_id = p_user_id;
END;
$$ LANGUAGE plpgsql;