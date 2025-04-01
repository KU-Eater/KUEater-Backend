CREATE OR REPLACE FUNCTION kueater.toggle_like_menu(
    p_user_id UUID,
    p_menu_id UUID
)
RETURNS VOID AS $$
DECLARE
    liked BOOLEAN;
    disliked BOOLEAN;
BEGIN
    SELECT EXISTS (
        SELECT 1
        FROM kueater.liked_item
        WHERE user_id = p_user_id
        AND menu_id = p_menu_id
    ) INTO liked;

    SELECT EXISTS (
        SELECT 1
        FROM kueater.disliked_item
        WHERE user_id = p_user_id
        AND menu_id = p_menu_id
    ) INTO disliked;

    -- reset
    IF liked AND disliked THEN
        DELETE FROM kueater.liked_item
        WHERE user_id = p_user_id
        AND menu_id = p_menu_id;

        DELETE FROM kueater.disliked_item
        WHERE user_id = p_user_id
        AND menu_id = p_menu_id;

        RETURN;
    END IF;

    -- remove dislike
    IF disliked THEN
        DELETE FROM kueater.disliked_item
        WHERE user_id = p_user_id
        AND menu_id = p_menu_id;
    END IF;

    -- is there - delete
    IF liked THEN
        DELETE FROM kueater.liked_item
        WHERE user_id = p_user_id
        AND menu_id = p_menu_id;
        RETURN;
    ELSE
        INSERT INTO kueater.liked_item (user_id, menu_id) VALUES
        (p_user_id, p_menu_id);
        RETURN;
    END IF;
END;
$$ LANGUAGE plpgsql;



CREATE OR REPLACE FUNCTION kueater.toggle_dislike_menu(
    p_user_id UUID,
    p_menu_id UUID
)
RETURNS VOID AS $$
DECLARE
    liked BOOLEAN;
    disliked BOOLEAN;
BEGIN
    SELECT EXISTS (
        SELECT 1
        FROM kueater.liked_item
        WHERE user_id = p_user_id
        AND menu_id = p_menu_id
    ) INTO liked;

    SELECT EXISTS (
        SELECT 1
        FROM kueater.disliked_item
        WHERE user_id = p_user_id
        AND menu_id = p_menu_id
    ) INTO disliked;

    -- reset
    IF liked AND disliked THEN
        DELETE FROM kueater.liked_item
        WHERE user_id = p_user_id
        AND menu_id = p_menu_id;

        DELETE FROM kueater.disliked_item
        WHERE user_id = p_user_id
        AND menu_id = p_menu_id;

        RETURN;
    END IF;

    -- remove like
    IF liked THEN
        DELETE FROM kueater.liked_item
        WHERE user_id = p_user_id
        AND menu_id = p_menu_id;
    END IF;

    -- is there - delete
    IF disliked THEN
        DELETE FROM kueater.disliked_item
        WHERE user_id = p_user_id
        AND menu_id = p_menu_id;
        RETURN;
    ELSE
        INSERT INTO kueater.disliked_item (user_id, menu_id) VALUES
        (p_user_id, p_menu_id);
        RETURN;
    END IF;
END;
$$ LANGUAGE plpgsql;




CREATE OR REPLACE FUNCTION kueater.toggle_save_menu(
    p_user_id UUID,
    p_menu_id UUID
)
RETURNS VOID AS $$
DECLARE
    saved BOOLEAN;
BEGIN
    SELECT EXISTS (
        SELECT 1
        FROM kueater.saved_item
        WHERE user_id = p_user_id
        AND menu_id = p_menu_id
    ) INTO saved;

    -- is there - delete
    IF saved THEN
        DELETE FROM kueater.saved_item
        WHERE user_id = p_user_id
        AND menu_id = p_menu_id;
        RETURN;
    ELSE
        INSERT INTO kueater.saved_item (user_id, menu_id) VALUES
        (p_user_id, p_menu_id);
        RETURN;
    END IF;
END;
$$ LANGUAGE plpgsql;




CREATE OR REPLACE FUNCTION kueater.toggle_like_stall(
    p_user_id UUID,
    p_stall_id UUID
)
RETURNS VOID AS $$
DECLARE
    liked BOOLEAN;
BEGIN
    SELECT EXISTS (
        SELECT 1
        FROM kueater.liked_stall
        WHERE user_id = p_user_id
        AND stall_id = p_stall_id
    ) INTO liked;

    -- is there - delete
    IF liked THEN
        DELETE FROM kueater.liked_stall
        WHERE user_id = p_user_id
        AND stall_id = p_stall_id;
        RETURN;
    ELSE
        INSERT INTO kueater.liked_stall (user_id, stall_id) VALUES
        (p_user_id, p_stall_id);
        RETURN;
    END IF;
END;
$$ LANGUAGE plpgsql;



CREATE OR REPLACE FUNCTION kueater.toggle_save_stall(
    p_user_id UUID,
    p_stall_id UUID
)
RETURNS VOID AS $$
DECLARE
    saved BOOLEAN;
BEGIN
    SELECT EXISTS (
        SELECT 1
        FROM kueater.saved_stall
        WHERE user_id = p_user_id
        AND stall_id = p_stall_id
    ) INTO saved;

    -- is there - delete
    IF saved THEN
        DELETE FROM kueater.saved_stall
        WHERE user_id = p_user_id
        AND stall_id = p_stall_id;
        RETURN;
    ELSE
        INSERT INTO kueater.saved_stall (user_id, stall_id) VALUES
        (p_user_id, p_stall_id);
        RETURN;
    END IF;
END;
$$ LANGUAGE plpgsql;