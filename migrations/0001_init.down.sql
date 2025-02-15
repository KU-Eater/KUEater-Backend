-- Drop tables with foreign key dependencies first
DROP TABLE IF EXISTS kueater.review_context;
DROP TABLE IF EXISTS kueater.review;

-- Drop relationship tables
DROP TABLE IF EXISTS kueater.favorite_stall;
DROP TABLE IF EXISTS kueater.favorite_item;
DROP TABLE IF EXISTS kueater.disliked_item;
DROP TABLE IF EXISTS kueater.liked_stall;
DROP TABLE IF EXISTS kueater.liked_item;

-- Drop user-related tables
DROP TABLE IF EXISTS kueater.user_dietary_restraint;
DROP TABLE IF EXISTS kueater.userprofile;
DROP TABLE IF EXISTS kueater.dietary_restriction;

-- Drop menu and stall relationship tables
DROP TABLE IF EXISTS kueater.stall_menu;
DROP TABLE IF EXISTS kueater.menu_ingredient;

-- Drop main tables
DROP TABLE IF EXISTS kueater.stall;
DROP TABLE IF EXISTS kueater.menuitem;
DROP TABLE IF EXISTS kueater.ingredient;

-- Drop enum type after all tables using it are dropped
DROP TYPE IF EXISTS kueater.restraint_type CASCADE;

-- Finally drop the schema
DROP SCHEMA IF EXISTS kueater;