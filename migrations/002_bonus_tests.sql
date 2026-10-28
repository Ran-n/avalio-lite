-- Bonus tests: extra credit that counts on top of the normal tests' 100 %.
-- Only test columns can be bonus tests.
ALTER TABLE sheet_columns ADD COLUMN bonus INTEGER NOT NULL DEFAULT 0
    CHECK (bonus IN (0, 1) AND (bonus = 0 OR kind = 'test'));
