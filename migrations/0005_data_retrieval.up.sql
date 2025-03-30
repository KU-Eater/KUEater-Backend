-- Function to help with data retrieval

CREATE OR REPLACE FUNCTION kueater.get_all_menuitems_with_ingredients ()
RETURNS TABLE (
    menu_id UUID,
    menu_name TEXT,
    ingredients JSONB
)
LANGUAGE plpgsql AS
$$
BEGIN
    RETURN QUERY
    SELECT
        m.id AS menu_id,
        m.name AS menu_name,
        (
            SELECT jsonb_agg(
                jsonb_build_object(
                    'id', mi.ingredient_id,
                    'name', i.name
                )
            )
            FROM kueater.menu_ingredient mi
            JOIN kueater.ingredient i ON mi.ingredient_id = i.id
            WHERE mi.menu_id = m.id
        ) AS ingredients
    FROM kueater.menuitem m;
END; $$;