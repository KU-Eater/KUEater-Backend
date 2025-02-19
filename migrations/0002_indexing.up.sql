-- Create indexing for existing tables

CREATE INDEX IF NOT EXISTS ingredient_idx ON kueater.ingredient USING btree(id);
CREATE INDEX IF NOT EXISTS menuitem_idx ON kueater.menuitem USING btree(id);
CREATE INDEX IF NOT EXISTS stall_idx ON kueater.stall USING btree(id);

-- Demo migration
CREATE INDEX IF NOT EXISTS menuitem_name_idx ON kueater.menuitem (LOWER(name));