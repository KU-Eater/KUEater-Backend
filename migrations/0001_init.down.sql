-- Drop tables with foreign key dependencies first
DROP TABLE IF EXISTS kueater.review_context;
DROP TABLE IF EXISTS kueater.review;

-- Drop relationship tables
DROP TABLE IF EXISTS kueater.saved_stall;
DROP TABLE IF EXISTS kueater.saved_item;
DROP TABLE IF EXISTS kueater.disliked_item;
DROP TABLE IF EXISTS kueater.liked_stall;
DROP TABLE IF EXISTS kueater.liked_item;

-- Drop user-related tables
DROP TABLE IF EXISTS kueater.user_profile_preferences;
DROP TABLE IF EXISTS kueater.userprofile;
DROP TABLE IF EXISTS kueater.user_preferences;

-- Drop menu and stall relationship tables
DROP TABLE IF EXISTS kueater.stall_menu;
DROP TABLE IF EXISTS kueater.menu_ingredient;

-- Drop main tables
DROP TABLE IF EXISTS kueater.stall;
DROP TABLE IF EXISTS kueater.menuitem;
DROP TABLE IF EXISTS kueater.ingredient;

-- Drop enum type after all tables using it are dropped
DROP TYPE IF EXISTS kueater.role CASCADE;
DROP TYPE IF EXISTS kueater.gender CASCADE;
DROP TYPE IF EXISTS kueater.allergen CASCADE;
DROP TYPE IF EXISTS kueater.diet CASCADE;

-- Finally drop the schema
DROP SCHEMA IF EXISTS kueater;