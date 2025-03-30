-- Tables for recommendation system

DO $$ BEGIN
    CREATE TYPE kueater.object_type AS ENUM (
        'ingredient', 'menuitem', 'stall'
    );
EXCEPTION
    WHEN duplicate_object THEN null;
END; $$;

CREATE TABLE IF NOT EXISTS kueater.embeddings (
    object_id UUID NOT NULL,
    object_type kueater.object_type NOT NULL,
    string TEXT NOT NULL,
    lang TEXT NOT NULL,
    embedding vector(768) NOT NULL
);

-- Index for embeddings
CREATE INDEX idx_embeddings_id_type ON kueater.embeddings (object_id, object_type);

CREATE INDEX IF NOT EXISTS idx_embeddings_menuitem_vector ON kueater.embeddings USING vectors (embedding vector_cos_ops)
WHERE (object_type = 'menuitem');

CREATE INDEX IF NOT EXISTS idx_embeddings_ingredient_vector ON kueater.embeddings USING vectors (embedding vector_cos_ops)
WHERE (object_type = 'ingredient');

CREATE INDEX IF NOT EXISTS idx_embeddings_stall_vector ON kueater.embeddings USING vectors (embedding vector_cos_ops)
WHERE (object_type = 'stall');

CREATE TABLE IF NOT EXISTS kueater.menuitem_scores (
    id SERIAL PRIMARY KEY,
    user_id UUID REFERENCES kueater.userprofile ON DELETE CASCADE,
    menu_id UUID REFERENCES kueater.menuitem ON DELETE CASCADE,
    score DECIMAL NOT NULL,
    reasoning TEXT DEFAULT "",
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    stale BOOLEAN DEFAULT FALSE
);

-- Materialized view so it's non-blocking when we generate recommendations.
CREATE MATERIALIZED VIEW kueater.current_menuitem_scores AS
SELECT
    id,
    user_id,
    menu_id,
    score,
    reasoning,
    created_at
FROM kueater.menuitem_scores
WHERE stale = FALSE
ORDER BY score DESC;

CREATE UNIQUE INDEX unique_current_menuitem_scores ON kueater.embeddings (id);

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
    score DECIMAL,
    reasoning TEXT
) LANGUAGE plpgsql AS
$$
BEGIN
    RETURN QUERY
    SELECT
        tbl.menu_id,
        tbl.score,
        tbl.reasoning
    FROM kueater.current_menuitem_scores tbl
    WHERE tbl.user_id = p_user_id
    ORDER BY tbl.score DESC
    LIMIT p_limit;
END;
$$;

CREATE OR REPLACE FUNCTION kueater.get_menuitem_score_of(p_user_id UUID, p_menu_id UUID)
RETURNS TABLE (
    score DECIMAL,
    reasoning TEXT
) LANGUAGE plpgsql AS
$$
BEGIN
    RETURN QUERY
    SELECT
        tbl.score,
        tbl.reasoning
    FROM kueater.current_menuitem_scores tbl
    WHERE tbl.user_id = p_user_id AND tbl.menu_id = p_menu_id;
END;
$$;

-- Table for storing diet score in each ingredient
CREATE TABLE IF NOT EXISTS kueater.ingredient_diet_score (
    ingredient_id UUID REFERENCES kueater.ingredient ON DELETE CASCADE,
    diet kueater.diet NOT NULL,
    score DECIMAL DEFAULT 0,
    PRIMARY KEY (ingredient_id, diet)
);

-- Table for storing allergen score in each ingredient
CREATE TABLE IF NOT EXISTS kueater.ingredient_allergen_score (
    ingredient_id UUID REFERENCES kueater.ingredient ON DELETE CASCADE,
    allergen kueater.allergen NOT NULL,
    score DECIMAL DEFAULT 1,
    PRIMARY KEY (ingredient_id, allergen)
);

CREATE OR REPLACE FUNCTION kueater.get_ingredient_compatibility_score(
    p_ingredient_id UUID
)
RETURNS TABLE (
    ingredient_id UUID,
    ingredient_name TEXT,
    diet_scores JSONB,
    allergen_scores JSONB
)
LANGUAGE plpgsql AS
$$
BEGIN
    RETURN QUERY
    WITH ingredient_data AS (
        SELECT 
            i.id AS ingredient_id,
            i.name AS ingredient_name
        FROM kueater.ingredient i
        WHERE i.id = p_ingredient_id
    ),
    diet_data AS (
        SELECT 
            id.ingredient_id,
            id.ingredient_name,
            COALESCE(
                (
                    SELECT jsonb_object_agg(
                        ids.diet::text,
                        ids.score
                    )
                    FROM kueater.ingredient_diet_score ids
                    WHERE ids.ingredient_id = id.ingredient_id
                ),
                '{}'::jsonb
            ) AS diet_scores
        FROM ingredient_data id
    ),
    allergen_data AS (
        SELECT 
            id.ingredient_id,
            COALESCE(
                (
                    SELECT jsonb_object_agg(
                        ias.allergen::text,
                        ias.score
                    )
                    FROM kueater.ingredient_allergen_score ias
                    WHERE ias.ingredient_id = id.ingredient_id
                ),
                '{}'::jsonb
            ) AS allergen_scores
        FROM ingredient_data id
    )
    SELECT 
        dd.ingredient_id,
        dd.ingredient_name,
        dd.diet_scores,
        ad.allergen_scores
    FROM diet_data dd
    JOIN allergen_data ad ON dd.ingredient_id = ad.ingredient_id;
END;
$$;

-- Function to use User Preference to list scores of Diet and Allergen of given Menu Item
CREATE OR REPLACE FUNCTION kueater.get_menuitem_compatibility_score(
    p_menu_id UUID
)
RETURNS TABLE (
    ingredient_id UUID,
    ingredient_name TEXT,
    diet_scores JSONB,
    allergen_scores JSONB
)
LANGUAGE plpgsql AS
$$
DECLARE
    user_diets kueater.diet[];
    user_allergens kueater.allergen[];
BEGIN
    RETURN QUERY
    WITH menu_ingredients AS (
        SELECT 
            mi.ingredient_id, 
            i.name AS ingredient_name
        FROM kueater.menu_ingredient AS mi
        JOIN kueater.ingredient i ON mi.ingredient_id = i.id
        WHERE mi.menu_id = p_menu_id
    ),
    diet_data AS (
        SELECT 
            mi.ingredient_id,
            mi.ingredient_name,
            COALESCE(
                (
                    SELECT jsonb_object_agg(
                        ids.diet::text,
                        ids.score
                    )
                    FROM kueater.ingredient_diet_score ids
                    WHERE ids.ingredient_id = mi.ingredient_id
                ),
                '{}'::jsonb
            ) AS diet_scores
        FROM menu_ingredients mi
    ),
    allergen_data AS (
        SELECT 
            mi.ingredient_id,
            COALESCE(
                (
                    SELECT jsonb_object_agg(
                        ias.allergen::text,
                        ias.score
                    )
                    FROM kueater.ingredient_allergen_score ias
                    WHERE ias.ingredient_id = mi.ingredient_id
                ),
                '{}'::jsonb
            ) AS allergen_scores
        FROM menu_ingredients mi
    )
    SELECT 
        dd.ingredient_id,
        dd.ingredient_name,
        dd.diet_scores,
        ad.allergen_scores
    FROM diet_data dd
    JOIN allergen_data ad ON dd.ingredient_id = ad.ingredient_id;
END;
$$;