-- Initialize database with tables

CREATE SCHEMA IF NOT EXISTS kueater;

CREATE TABLE IF NOT EXISTS kueater.ingredient (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v7(),
    name TEXT NOT NULL -- fallback translation
);

CREATE TABLE IF NOT EXISTS kueater.menuitem (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v7(),
    name TEXT NOT NULL,
    price NUMERIC(6, 2) NOT NULL,
    image TEXT
);

CREATE TABLE IF NOT EXISTS kueater.menu_ingredient (
    menu_id UUID REFERENCES kueater.menuitem ON DELETE CASCADE,
    ingredient_id UUID REFERENCES kueater.ingredient ON DELETE RESTRICT,  -- will affect a lot if deleted, maybe let admins fix the issue with data?
    PRIMARY KEY (menu_id, ingredient_id)
);

CREATE TABLE IF NOT EXISTS kueater.stall (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v7(),
    name TEXT NOT NULL,
    lock INTEGER,
    image TEXT,
    dish_type TEXT
);

CREATE TABLE IF NOT EXISTS kueater.stall_menu (
    stall_id UUID REFERENCES kueater.stall ON DELETE CASCADE,
    menu_id UUID REFERENCES kueater.menuitem ON DELETE CASCADE,
    PRIMARY KEY (stall_id, menu_id)
);

-- User personal domain

DO $$ BEGIN
    CREATE TYPE kueater.restraint_type AS ENUM ('allergic', 'religional');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

CREATE TABLE IF NOT EXISTS kueater.dietary_restriction (
    id BIGSERIAL PRIMARY KEY,
    ingredient_id UUID REFERENCES kueater.ingredient ON DELETE RESTRICT,
    type kueater.restraint_type NOT NULL
);

CREATE TABLE IF NOT EXISTS kueater.userprofile (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v7(),
    name VARCHAR(30) NOT NULL
);

CREATE TABLE IF NOT EXISTS kueater.user_dietary_restraint (
    user_id UUID REFERENCES kueater.userprofile ON DELETE CASCADE,
    restraint_id BIGINT REFERENCES kueater.dietary_restriction ON DELETE CASCADE,
    PRIMARY KEY (user_id, restraint_id)
);

-- relationship profiles and items/stalls

CREATE TABLE IF NOT EXISTS kueater.liked_item (
    user_id UUID REFERENCES kueater.userprofile ON DELETE CASCADE,
    menu_id UUID REFERENCES kueater.menuitem ON DELETE CASCADE,
    PRIMARY KEY (user_id, menu_id)
);

CREATE TABLE IF NOT EXISTS kueater.liked_stall (
    user_id UUID REFERENCES kueater.userprofile ON DELETE CASCADE,
    stall_id UUID REFERENCES kueater.stall ON DELETE CASCADE,
    PRIMARY KEY (user_id, stall_id)
);

CREATE TABLE IF NOT EXISTS kueater.disliked_item (
    user_id UUID REFERENCES kueater.userprofile ON DELETE CASCADE,
    menu_id UUID REFERENCES kueater.menuitem ON DELETE CASCADE,
    PRIMARY KEY (user_id, menu_id)
);

CREATE TABLE IF NOT EXISTS kueater.favorite_item (
    user_id UUID REFERENCES kueater.userprofile ON DELETE CASCADE,
    menu_id UUID REFERENCES kueater.menuitem ON DELETE CASCADE,
    PRIMARY KEY (user_id, menu_id)
);

CREATE TABLE IF NOT EXISTS kueater.favorite_stall (
    user_id UUID REFERENCES kueater.userprofile ON DELETE CASCADE,
    stall_id UUID REFERENCES kueater.stall ON DELETE CASCADE,
    PRIMARY KEY (user_id, stall_id)
);

-- Reviews

CREATE TABLE IF NOT EXISTS kueater.review (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v7(),
    author UUID REFERENCES kueater.userprofile ON DELETE SET NULL,
    stall UUID REFERENCES kueater.stall ON DELETE CASCADE,
    content TEXT NOT NULL,
    score INT NOT NULL,
    created TIMESTAMP NOT NULL,
    updated TIMESTAMP
);

CREATE TABLE IF NOT EXISTS kueater.review_context (
    review_id UUID REFERENCES kueater.review ON DELETE CASCADE,
    menu_id UUID REFERENCES kueater.menuitem ON DELETE CASCADE,
    PRIMARY KEY (review_id, menu_id)
);