-- Function to see if user is ready to be used.
CREATE OR REPLACE FUNCTION kueater.get_user_readiness (
    p_user_id UUID
)
RETURNS BOOLEAN
AS
$$
DECLARE
    username_check BOOLEAN;
    preference_exists BOOLEAN;
BEGIN
    -- Check if username is not blank for user entry
    SELECT
    BOOL_AND(LENGTH(name) > 0) INTO username_check
    FROM kueater.userprofile
    WHERE id = p_user_id;

    -- Check if preference exist for user id
    SELECT
    COUNT(*) INTO preference_exists
    FROM kueater.user_profile_preferences
    WHERE user_id = p_user_id;

    RETURN username_check AND preference_exists;
END;
$$ LANGUAGE plpgsql;

-- Function to update profile.
CREATE OR REPLACE FUNCTION kueater.update_profile (
    p_user_id UUID,
    p_username VARCHAR(30),
    p_gender kueater.gender,
    p_role kueater.role
) RETURNS VOID
AS $$
BEGIN
    UPDATE kueater.userprofile
    SET name = p_username,
        gender = p_gender,
        role = p_role
    WHERE id = p_user_id;
END;
$$ LANGUAGE plpgsql;

-- Function to create preferences data.
CREATE OR REPLACE FUNCTION kueater.create_preferences (
    p_user_id UUID,
    p_diets kueater.diet ARRAY,
    p_allergies kueater.allergen ARRAY,
    p_cuisines TEXT ARRAY,
    p_disliked_ingredients TEXT ARRAY,
    p_favorite_dishes TEXT ARRAY
) RETURNS VOID
AS $$
DECLARE
    new_pref_id BIGINT;
BEGIN
    -- Ensure no data really exist
    DELETE FROM kueater.user_profile_preferences
    WHERE user_id = p_user_id;

    -- Create new preference object
    INSERT INTO kueater.user_preferences (
        diets, allergies, cuisines, disliked_ingredients, favorite_dishes
    ) VALUES (
        p_diets, p_allergies, p_cuisines, p_disliked_ingredients, p_favorite_dishes
    ) RETURNING id INTO new_pref_id;

    -- Link between two objects
    INSERT INTO kueater.user_profile_preferences (
        user_id, preferences_id
    ) VALUES (
        p_user_id, new_pref_id
    );
END;
$$ LANGUAGE plpgsql;

-- Function to update preferences data.
CREATE OR REPLACE FUNCTION kueater.update_preferences (
    p_user_id UUID,
    p_diets kueater.diet ARRAY,
    p_allergies kueater.allergen ARRAY,
    p_cuisines TEXT ARRAY,
    p_disliked_ingredients TEXT ARRAY,
    p_favorite_dishes TEXT ARRAY
) RETURNS VOID
AS $$
DECLARE
    existing_pref_id BIGINT;
BEGIN
    SELECT preferences_id INTO existing_pref_id
    FROM kueater.user_profile_preferences
    WHERE user_id = p_user_id;

    IF existing_pref_id IS NULL THEN
        RAISE EXCEPTION 'Non existent preference for user %', p_user_id;
    ELSE
        UPDATE kueater.user_preferences
        SET 
            diets = p_diets,
            allergies = p_allergies,
            cuisines = p_cuisines,
            disliked_ingredients = p_disliked_ingredients,
            favorite_dishes = p_favorite_dishes
        WHERE id = existing_pref_id;
    END IF;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION kueater.get_menu_card_props(
    p_menu_id UUID,
    p_user_id UUID
)
RETURNS TABLE (
    uuid TEXT,
    name TEXT,
    price DOUBLE PRECISION,
    likes INTEGER,
    dislikes INTEGER,
    stall_id TExt,
    stall_name TEXT,
    stall_lock TEXT,
    image_url TEXT,
    score FLOAT,
    reason TEXT,
    liked BOOLEAN,
    disliked BOOLEAN,
    saved BOOLEAN
)
LANGUAGE plpgsql
AS $$
BEGIN
    RETURN QUERY
    WITH likes_count AS (
        SELECT menu_id, COUNT(*) as count
        FROM kueater.liked_item
        GROUP BY menu_id
    ),
    dislikes_count AS (
        SELECT menu_id, COUNT(*) as count
        FROM kueater.disliked_item
        GROUP BY menu_id
    ),
    recommendation_data AS (
        SELECT menu_id, cms.score, reasoning
        FROM kueater.current_menuitem_scores cms
        WHERE user_id = p_user_id AND menu_id = p_menu_id
    )
    SELECT
        m.id::TEXT,                                    -- uuid
        m.name,                                        -- name
        m.price::DOUBLE PRECISION,                     -- price
        COALESCE(lc.count, 0)::INTEGER,                -- likes
        COALESCE(dc.count, 0)::INTEGER,                -- dislikes
        s.id::TEXT,
        s.name,                                         -- stall_name
        s.lock::TEXT,                                   -- stall_lock (converted to text)
        m.image,                                        -- image_url
        r.score::FLOAT,                                 -- score (optional)
        r.reasoning,                                    -- reason (optional)
        (EXISTS (SELECT 1 FROM kueater.liked_item li 
                WHERE li.menu_id = m.id AND li.user_id = p_user_id))::BOOLEAN, -- liked
        (EXISTS (SELECT 1 FROM kueater.disliked_item di
                WHERE di.menu_id = m.id AND di.user_id = p_user_id))::BOOLEAN, -- disliked
        (EXISTS (SELECT 1 FROM kueater.saved_item si
                WHERE si.menu_id = m.id AND si.user_id = p_user_id))::BOOLEAN  -- saved
    FROM kueater.menuitem m
    LEFT JOIN kueater.stall_menu sm ON m.id = sm.menu_id
    LEFT JOIN kueater.stall s ON sm.stall_id = s.id
    LEFT JOIN likes_count lc ON m.id = lc.menu_id
    LEFT JOIN dislikes_count dc ON m.id = dc.menu_id
    LEFT JOIN recommendation_data r ON m.id = r.menu_id
    WHERE m.id = p_menu_id;
END;
$$;



-- Stall ranking algorithm
CREATE OR REPLACE FUNCTION kueater.get_stall_data_props(
    p_stall_id UUID,
    p_user_id UUID
)
RETURNS TABLE (
    uuid TEXT,                 -- stall ID
    rank INT4,                 -- rank position as int32
    name TEXT,                 -- stall name
    image_url TEXT,            -- stall image
    location TEXT,             
    operating_hours TEXT,      -- Combining open_hour and close_hour
    price_range TEXT,          -- min - max format
    tags TEXT,                 -- stall tags
    reviews INT4,              -- count of reviews as int32
    likes INT4,                -- count of likes as int32
    rating FLOAT4,             -- average review score as float
    saved BOOLEAN              -- whether the user has saved this stall
) AS $$
BEGIN
    RETURN QUERY
    WITH stall_reviews AS (
        SELECT 
            stall,
            COUNT(*)::INT4 AS review_count,
            COALESCE(AVG(score), 0)::FLOAT4 AS avg_score
        FROM 
            kueater.review
        WHERE 
            stall = p_stall_id
        GROUP BY 
            stall
    ),
    stall_likes AS (
        SELECT 
            stall_id,
            COUNT(*)::INT4 AS like_count
        FROM 
            kueater.liked_stall
        WHERE 
            stall_id = p_stall_id
        GROUP BY 
            stall_id
    ),
    stall_price_ranges AS (
        SELECT 
            sm.stall_id,
            MIN(mi.price)::INT4 AS min_price,
            MAX(mi.price)::INT4 AS max_price
        FROM 
            kueater.stall_menu sm
        JOIN 
            kueater.menuitem mi ON sm.menu_id = mi.id
        WHERE 
            sm.stall_id = p_stall_id
        GROUP BY 
            sm.stall_id
    ),
    -- Subquery to get the stall's rank among all stalls
    all_stalls_ranked AS (
        SELECT 
            s.id,
            ROW_NUMBER() OVER (ORDER BY (
                COALESCE((SELECT COUNT(*) FROM kueater.liked_stall WHERE stall_id = s.id), 0) * 0.4 + 
                COALESCE((SELECT COUNT(*) FROM kueater.review WHERE stall = s.id), 0) * 0.3 + 
                COALESCE((SELECT AVG(score) FROM kueater.review WHERE stall = s.id), 0) * 0.3
            ) DESC)::INT4 AS rank
        FROM 
            kueater.stall s
    )
    
    SELECT 
        s.id::TEXT AS uuid,
        asr.rank AS rank,
        s.name AS name,
        s.image AS image_url,
        s.lock::TEXT AS location,  -- Not in original schema
        CASE 
            WHEN s.open_hour IS NOT NULL AND s.close_hour IS NOT NULL THEN
                s.open_hour || ' - ' || s.close_hour
            ELSE NULL
        END AS operating_hours,
        CASE
            WHEN spr.min_price = spr.max_price THEN spr.min_price::TEXT
            WHEN spr.min_price IS NULL OR spr.max_price IS NULL THEN NULL
            ELSE spr.min_price::TEXT || ' - ' || spr.max_price::TEXT
        END AS price_range,
        s.tags AS tags,
        COALESCE(sr.review_count, 0)::INT4 AS reviews,
        COALESCE(sl.like_count, 0)::INT4 AS likes,
        COALESCE(sr.avg_score, 0)::FLOAT4 AS rating,
        CASE 
            WHEN p_user_id IS NOT NULL THEN 
                EXISTS (
                    SELECT 1 FROM kueater.saved_stall ss 
                    WHERE ss.stall_id = s.id AND ss.user_id = p_user_id
                )
            ELSE false
        END AS saved
    FROM 
        kueater.stall s
    LEFT JOIN 
        stall_reviews sr ON s.id = sr.stall
    LEFT JOIN 
        stall_likes sl ON s.id = sl.stall_id
    LEFT JOIN
        stall_price_ranges spr ON s.id = spr.stall_id
    LEFT JOIN
        all_stalls_ranked asr ON s.id = asr.id
    WHERE 
        s.id = p_stall_id;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION kueater.multi_stall_data_props(
    p_user_id UUID,  -- Optional user ID to check for saved status
    p_limit INTEGER DEFAULT 20
)
RETURNS TABLE (
    uuid TEXT,                 -- stall ID
    rank INT4,                 -- rank position as int32
    name TEXT,                 -- stall name
    image_url TEXT,            -- stall image
    location TEXT,             -- Not in original schema, leaving as NULL
    operating_hours TEXT,      -- Combining open_hour and close_hour
    price_range TEXT,          -- min - max format
    tags TEXT,                 -- stall tags
    reviews INT4,              -- count of reviews as int32
    likes INT4,                -- count of likes as int32
    rating FLOAT4,             -- average review score as float
    saved BOOLEAN              -- whether the user has saved this stall
) AS $$
BEGIN
    RETURN QUERY
    WITH stall_reviews AS (
        SELECT 
            stall,
            COUNT(*)::INT4 AS review_count,
            COALESCE(AVG(score), 0)::FLOAT4 AS avg_score
        FROM 
            kueater.review
        GROUP BY 
            stall
    ),
    stall_likes AS (
        SELECT 
            stall_id,
            COUNT(*)::INT4 AS like_count
        FROM 
            kueater.liked_stall
        GROUP BY 
            stall_id
    ),
    stall_price_ranges AS (
        SELECT 
            sm.stall_id,
            MIN(mi.price)::INT4 AS min_price,
            MAX(mi.price)::INT4 AS max_price
        FROM 
            kueater.stall_menu sm
        JOIN 
            kueater.menuitem mi ON sm.menu_id = mi.id
        GROUP BY 
            sm.stall_id
    ),
    stall_rank_data AS (
        SELECT 
            s.id,
            s.name,
            s.image,
            s.lock::TEXT AS location,
            s.open_hour,
            s.close_hour,
            s.tags,
            COALESCE(sr.review_count, 0)::INT4 AS review_count,
            COALESCE(sl.like_count, 0)::INT4 AS like_count,
            COALESCE(sr.avg_score, 0)::FLOAT4 AS avg_score,
            COALESCE(spr.min_price, 0)::INT4 AS min_price,
            COALESCE(spr.max_price, 0)::INT4 AS max_price,
            ROW_NUMBER() OVER (ORDER BY (COALESCE(sl.like_count, 0) * 0.4 + 
                                       COALESCE(sr.review_count, 0) * 0.3 + 
                                       COALESCE(sr.avg_score, 0) * 0.3) DESC)::INT4 AS rank,
            CASE 
                WHEN p_user_id IS NOT NULL THEN 
                    EXISTS (
                        SELECT 1 FROM kueater.saved_stall ss 
                        WHERE ss.stall_id = s.id AND ss.user_id = p_user_id
                    )
                ELSE false
            END AS is_saved
        FROM 
            kueater.stall s
        LEFT JOIN 
            stall_reviews sr ON s.id = sr.stall
        LEFT JOIN 
            stall_likes sl ON s.id = sl.stall_id
        LEFT JOIN
            stall_price_ranges spr ON s.id = spr.stall_id
    )
    
    SELECT 
        srd.id::TEXT AS uuid,
        srd.rank AS rank,
        srd.name AS name,
        srd.image AS image_url,
        srd.location AS location,  -- Not in original schema
        CASE 
            WHEN srd.open_hour IS NOT NULL AND srd.close_hour IS NOT NULL THEN
                srd.open_hour || ' - ' || srd.close_hour
            ELSE NULL
        END AS operating_hours,
        CASE
            WHEN srd.min_price = srd.max_price THEN srd.min_price::TEXT
            WHEN srd.min_price = 0 AND srd.max_price = 0 THEN NULL
            ELSE srd.min_price::TEXT || ' - ' || srd.max_price::TEXT
        END AS price_range,
        srd.tags AS tags,
        srd.review_count AS reviews,
        srd.like_count AS likes,
        srd.avg_score AS rating,
        srd.is_saved AS saved
    FROM 
        stall_rank_data srd
    ORDER BY 
        rank
    LIMIT p_limit;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION kueater.get_stall_rating_summary(
    p_stall_id UUID
)
RETURNS TABLE (
    avg_stall_rating DOUBLE PRECISION,
    total_reviews INTEGER,
    total_likes INTEGER,
    total_menu_saved INTEGER,
    total_stall_saved INTEGER,
    one_star_total INTEGER,
    one_star_percent DOUBLE PRECISION,
    two_star_total INTEGER,
    two_star_percent DOUBLE PRECISION,
    three_star_total INTEGER,
    three_star_percent DOUBLE PRECISION,
    four_star_total INTEGER,
    four_star_percent DOUBLE PRECISION,
    five_star_total INTEGER,
    five_star_percent DOUBLE PRECISION
)
LANGUAGE plpgsql
AS $$
DECLARE
    v_total_reviews INTEGER;
BEGIN
    -- Get total number of reviews for percentage calculations
    SELECT COUNT(*) INTO v_total_reviews
    FROM kueater.review
    WHERE stall = p_stall_id;
    
    RETURN QUERY
    WITH 
    -- Get star counts
    star_counts AS (
        SELECT
            COUNT(*) FILTER (WHERE score = 1) AS one_star,
            COUNT(*) FILTER (WHERE score = 2) AS two_star,
            COUNT(*) FILTER (WHERE score = 3) AS three_star,
            COUNT(*) FILTER (WHERE score = 4) AS four_star,
            COUNT(*) FILTER (WHERE score = 5) AS five_star,
            COUNT(*) AS total,
            COALESCE(AVG(score), 0) AS avg_rating
        FROM
            kueater.review
        WHERE
            stall = p_stall_id
    ),
    -- Get all menu items for this stall
    stall_menus AS (
        SELECT
            menu_id
        FROM
            kueater.stall_menu
        WHERE
            stall_id = p_stall_id
    ),
    -- Sum of likes for all menu items associated with this stall
    stall_menu_likes AS (
        SELECT
            COALESCE(SUM(
                (SELECT COUNT(*) 
                 FROM kueater.liked_item 
                 WHERE menu_id = sm.menu_id)
            ), 0) AS count
        FROM
            stall_menus sm
    ),
    -- Count saved stalls
    saved_stalls AS (
        SELECT
            COUNT(*) AS count
        FROM
            kueater.saved_stall
        WHERE
            stall_id = p_stall_id
    ),
    -- Count saved menus for this stall
    saved_menus AS (
        SELECT
            COUNT(*) AS count
        FROM
            kueater.saved_item si
        JOIN
            stall_menus sm ON si.menu_id = sm.menu_id
    )
    
    SELECT
        sc.avg_rating::DOUBLE PRECISION,
        sc.total::INTEGER,
        COALESCE(sml.count, 0)::INTEGER,
        COALESCE(saved_menus.count, 0)::INTEGER,
        COALESCE(saved_stalls.count, 0)::INTEGER,
        sc.one_star::INTEGER,
        CASE WHEN v_total_reviews > 0 THEN (sc.one_star::DOUBLE PRECISION / v_total_reviews) * 100 ELSE 0 END,
        sc.two_star::INTEGER,
        CASE WHEN v_total_reviews > 0 THEN (sc.two_star::DOUBLE PRECISION / v_total_reviews) * 100 ELSE 0 END,
        sc.three_star::INTEGER,
        CASE WHEN v_total_reviews > 0 THEN (sc.three_star::DOUBLE PRECISION / v_total_reviews) * 100 ELSE 0 END,
        sc.four_star::INTEGER,
        CASE WHEN v_total_reviews > 0 THEN (sc.four_star::DOUBLE PRECISION / v_total_reviews) * 100 ELSE 0 END,
        sc.five_star::INTEGER,
        CASE WHEN v_total_reviews > 0 THEN (sc.five_star::DOUBLE PRECISION / v_total_reviews) * 100 ELSE 0 END
    FROM
        star_counts sc
    CROSS JOIN
        stall_menu_likes sml
    CROSS JOIN
        saved_stalls
    CROSS JOIN
        saved_menus;
END;
$$;