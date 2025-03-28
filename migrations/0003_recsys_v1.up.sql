-- Tables for recommendation system

DO $$ BEGIN
    CREATE TYPE kueater.object_type AS ENUM (
        'ingredient', 'menuitem', 'stall'
    )
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

CREATE TABLE IF NOT EXISTS kueater.embeddings (
    object_id UUID NOT NULL,
    object_type kueater.object_type NOT NULL,
    lang TEXT NOT NULL,
    embedding vector(768) NOT NULL
);

CREATE TABLE IF NOT EXISTS kueater.menuitem_scores (
    id SERIAL PRIMARY KEY,
    user_id UUID REFERENCES kueater.userprofile ON DELETE CASCADE,
    menu_id UUID REFERENCES kueater.menuitem ON DELETE CASCADE,
    score DECIMAL NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    stale BOOLEAN DEFAULT FALSE
);

-- Materialized view so it's non-blocking when we generate recommendations.
CREATE MATERIALIZED VIEW kueater.current_menuitem_scores AS
SELECT
    user_id,
    menu_id,
    score,
    created_at
FROM kueater.menuitem_scores
WHERE stale = FALSE
ORDER BY score DESC;

-- Function to stale the scores of user
CREATE OR REPLACE FUNCTION kueater.stale_menuitem_scores_of(p_user_id UUID)
RETURNS VOID
LANGUAGE plpgsql
AS
$$
BEGIN
    UPDATE kueater.menuitem_scores
    SET stale = TRUE
    WHERE user_id = p_user_id AND stale = FALSE;
END;
$$;

-- Refresh materialized view
CREATE OR REPLACE FUNCTION kueater.refresh_menuitem_scores()
RETURNS VOID
LANGUAGE plpgsql
AS
$$
BEGIN
    REFRESH MATERIALIZED VIEW CONCURRENTLY kueater.current_menuitem_scores;
END;
$$;

-- Function to get user's item scores
CREATE OR REPLACE FUNCTION kueater.get_menuitem_scores_of(p_user_id UUID, p_limit INTEGER DEFAULT 10)
RETURNS TABLE (
    menu_id UUID,
    score DECIMAL
) LANGUAGE plpgsql AS
$$
BEGIN
    RETURN QUERY
    SELECT
        tbl.menu_id,
        tbl.score,
    FROM kueater.current_menuitem_scores tbl
    WHERE tbl.user_id = p_user_id
    ORDER BY tbl.score DESC
    LIMIT p_limit;
END;
$$;

-- Table for storing diet score in each ingredient
CREATE TABLE IF NOT EXISTS kueater.ingredient_diet_score (
    ingredient_id UUID REFERENCES kueater.ingredient ON DELETE CASCADE,
    diet kueater.diet NOT NULL,
    score DECIMAL DEFAULT 0
);

-- Table for storing allergen score in each ingredient
CREATE TABLE IF NOT EXISTS kueater.ingredient_allergen_score (
    ingredient_id UUID REFERENCES kueater.ingredient ON DELETE CASCADE,
    allergen kueater.allergen NOT NULL,
    score DECIMAL DEFAULT 1
);