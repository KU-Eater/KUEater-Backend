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
    image TEXT,
    cuisine TEXT,
    food_type TEXT
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
    open_hour TEXT,
    close_hour TEXT,
    tags TEXT
);

CREATE TABLE IF NOT EXISTS kueater.stall_menu (
    stall_id UUID REFERENCES kueater.stall ON DELETE CASCADE,
    menu_id UUID REFERENCES kueater.menuitem ON DELETE CASCADE,
    PRIMARY KEY (stall_id, menu_id)
);

-- User personalization domain

DO $$ BEGIN
    CREATE TYPE kueater.diet AS ENUM (
        'Halal', 'Vegetarian', 'Vegan', 'Pescatarian', 
        'Pollotarian', 'Low-Carb', 'Keto', 'Low-Fat', 'High-Protein'
    );
    CREATE TYPE kueater.allergen AS ENUM (
        'Lactose', 'Eggs', 'Shellfish', 'Fishes', 'Seafood',
        'Peanuts', 'Gluten', 'Sesame', 'Nuts', 'Soy', 'Rice',
        'Red Meat', 'Corn', 'Wheat', 'Fructose', 'Chocolate', 
        'Msg'
    );
    CREATE TYPE kueater.gender AS ENUM (
        'Male', 'Female', 'Non-Binary', 'Prefer not to say'
    );
    CREATE TYPE kueater.role AS ENUM (
        'KU Student', 'Exchange Student', 'KU Professor', 'KU Staff', 'Guest'
    );
EXCEPTION
    WHEN duplicate_object THEN null;
END; $$;

CREATE TABLE IF NOT EXISTS kueater.user_preferences (
    id BIGSERIAL PRIMARY KEY,
    diets kueater.diet ARRAY,
    allergies kueater.allergen ARRAY,
    cuisines TEXT ARRAY,
    disliked_ingredients TEXT ARRAY,
    favorite_dishes TEXT ARRAY
);

CREATE TABLE IF NOT EXISTS kueater.userprofile (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v7(),
    name VARCHAR(30) NOT NULL,
    email TEXT,
    gender kueater.gender DEFAULT 'Prefer not to say',
    role kueater.role DEFAULT 'Guest'
);

CREATE TABLE IF NOT EXISTS kueater.user_profile_preferences (
    user_id UUID REFERENCES kueater.userprofile ON DELETE CASCADE PRIMARY KEY,
    preferences_id BIGINT REFERENCES kueater.user_preferences ON DELETE SET NULL
);

-- Relationship profiles and items/stalls

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

CREATE TABLE IF NOT EXISTS kueater.saved_item (
    user_id UUID REFERENCES kueater.userprofile ON DELETE CASCADE,
    menu_id UUID REFERENCES kueater.menuitem ON DELETE CASCADE,
    PRIMARY KEY (user_id, menu_id)
);

CREATE TABLE IF NOT EXISTS kueater.saved_stall (
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