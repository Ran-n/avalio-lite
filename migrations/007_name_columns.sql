-- First name and surname become columns of the sheet like any other, so they take part in the
-- column order (moved with the panel's buttons, Ctrl+arrows or a drag) and keep their width in
-- their own row. The names themselves stay in `pupils`: a pupil is shared by every class they are
-- in, so their name is one fact, not one per class. A name column holds no settings and no cells.
--
-- SQLite can't change a CHECK constraint in place, so `sheet_columns` is rebuilt the way SQLite
-- documents (create the new table, copy, drop the old, rename). The migration runner turns foreign
-- keys off around it, or dropping the old table would delete every cell through ON DELETE CASCADE,
-- and checks them with `foreign_key_check` before committing.

-- Triggers on other tables whose bodies name sheet_columns would fail the rename; those on
-- sheet_columns itself would go with the old table. All are made again below, unchanged.
DROP TRIGGER remove_cells_when_pupil_leaves_class;
DROP TRIGGER cells_need_a_pupil_of_the_class;
DROP TRIGGER cells_keep_a_pupil_of_the_class;
DROP TRIGGER round_source_on_insert;
DROP TRIGGER round_source_on_update;
DROP TRIGGER cells_only_in_typed_columns_on_insert;
DROP TRIGGER cells_only_in_typed_columns_on_update;

-- One column of a class's sheet. Which settings apply depends on the kind:
--   first_name, surname  the pupil's names, from `pupils`; no title (it follows the interface
--                        language), no settings, never cells; exactly one of each per class
--   info      free text, no settings
--   test      a score out of max_score, counting weight % towards the final grade
--   weighted  final grade: sum of weight% x (score / test max), shown out of max_score
--   average   mean of the graded tests' score / max, shown out of max_score
--   round     source_column_id's value rounded to `decimals` places
-- Computed kinds (weighted, average, round) never store cells.
CREATE TABLE sheet_columns_new (
    id                INTEGER PRIMARY KEY,
    class_id          INTEGER NOT NULL REFERENCES classes (id) ON DELETE CASCADE,
    position          INTEGER NOT NULL,
    title             TEXT NOT NULL,
    kind              TEXT NOT NULL
        CHECK (kind IN ('first_name', 'surname', 'info', 'test', 'weighted', 'average', 'round')),
    weight            REAL,
    max_score         REAL,
    source_column_id  INTEGER REFERENCES sheet_columns (id) ON DELETE SET NULL,
    decimals          INTEGER,
    bonus             INTEGER NOT NULL DEFAULT 0 CHECK (bonus IN (0, 1) AND (bonus = 0 OR kind = 'test')),
    width             INTEGER CHECK (width IS NULL OR width BETWEEN 72 AND 640),
    CHECK (CASE kind
        WHEN 'first_name' THEN title = '' AND weight IS NULL AND max_score IS NULL
            AND source_column_id IS NULL AND decimals IS NULL
        WHEN 'surname' THEN title = '' AND weight IS NULL AND max_score IS NULL
            AND source_column_id IS NULL AND decimals IS NULL
        WHEN 'info' THEN weight IS NULL AND max_score IS NULL
            AND source_column_id IS NULL AND decimals IS NULL
        WHEN 'test' THEN weight IS NOT NULL AND weight >= 0
            AND max_score IS NOT NULL AND max_score > 0
            AND source_column_id IS NULL AND decimals IS NULL
        WHEN 'round' THEN weight IS NULL AND max_score IS NULL
            AND decimals IS NOT NULL AND decimals BETWEEN 0 AND 3
        ELSE weight IS NULL AND max_score IS NOT NULL AND max_score > 0
            AND source_column_id IS NULL AND decimals IS NULL
    END)
);

-- The mark columns move two places right, behind the names, which is where the sheet showed them.
INSERT INTO sheet_columns_new
    (id, class_id, position, title, kind, weight, max_score, source_column_id, decimals, bonus, width)
SELECT id, class_id, position + 2, title, kind, weight, max_score, source_column_id, decimals, bonus, width
FROM sheet_columns;

-- Each class's name columns, with the widths it kept for them (the first-name column could be as
-- narrow as 120 px; now every column can go down to 72 px).
INSERT INTO sheet_columns_new (class_id, position, title, kind, width)
SELECT id, 1, '', 'first_name', name_width FROM classes;
INSERT INTO sheet_columns_new (class_id, position, title, kind, width)
SELECT id, 2, '', 'surname', surname_width FROM classes;

DROP TABLE sheet_columns;
ALTER TABLE sheet_columns_new RENAME TO sheet_columns;

CREATE INDEX idx_sheet_columns_class ON sheet_columns (class_id, position);
CREATE INDEX idx_sheet_columns_source ON sheet_columns (source_column_id);
CREATE UNIQUE INDEX one_name_column_of_each_kind ON sheet_columns (class_id, kind)
    WHERE kind IN ('first_name', 'surname');

-- A class can never lack its name columns: they come with it.
CREATE TRIGGER classes_get_their_name_columns AFTER INSERT ON classes
BEGIN
    INSERT INTO sheet_columns (class_id, position, title, kind) VALUES (NEW.id, 1, '', 'first_name');
    INSERT INTO sheet_columns (class_id, position, title, kind) VALUES (NEW.id, 2, '', 'surname');
END;

-- The widths now sit in the name columns' rows.
ALTER TABLE classes DROP COLUMN name_width;
ALTER TABLE classes DROP COLUMN surname_width;

-- The triggers dropped above, as migrations 001, 003 and 004 made them.
CREATE TRIGGER remove_cells_when_pupil_leaves_class AFTER DELETE ON class_pupils
BEGIN
    DELETE FROM cells
    WHERE pupil_id = OLD.pupil_id
      AND column_id IN (SELECT id FROM sheet_columns WHERE class_id = OLD.class_id);
END;

CREATE TRIGGER cells_need_a_pupil_of_the_class BEFORE INSERT ON cells
WHEN NOT EXISTS (
    SELECT 1 FROM class_pupils cp JOIN sheet_columns sc ON sc.class_id = cp.class_id
    WHERE sc.id = NEW.column_id AND cp.pupil_id = NEW.pupil_id
)
BEGIN
    SELECT RAISE(ABORT, 'a cell must belong to a pupil of its column''s class');
END;

CREATE TRIGGER cells_keep_a_pupil_of_the_class BEFORE UPDATE OF column_id, pupil_id ON cells
WHEN NOT EXISTS (
    SELECT 1 FROM class_pupils cp JOIN sheet_columns sc ON sc.class_id = cp.class_id
    WHERE sc.id = NEW.column_id AND cp.pupil_id = NEW.pupil_id
)
BEGIN
    SELECT RAISE(ABORT, 'a cell must belong to a pupil of its column''s class');
END;

CREATE TRIGGER round_source_on_insert BEFORE INSERT ON sheet_columns
WHEN NEW.source_column_id IS NOT NULL AND NOT EXISTS (
    SELECT 1 FROM sheet_columns s
    WHERE s.id = NEW.source_column_id AND s.class_id = NEW.class_id AND s.kind IN ('test', 'weighted', 'average')
)
BEGIN
    SELECT RAISE(ABORT, 'a rounded column must round a test, final grade or average of its own class');
END;

CREATE TRIGGER round_source_on_update BEFORE UPDATE OF source_column_id, class_id ON sheet_columns
WHEN NEW.source_column_id IS NOT NULL AND NOT EXISTS (
    SELECT 1 FROM sheet_columns s
    WHERE s.id = NEW.source_column_id AND s.class_id = NEW.class_id AND s.kind IN ('test', 'weighted', 'average')
)
BEGIN
    SELECT RAISE(ABORT, 'a rounded column must round a test, final grade or average of its own class');
END;

CREATE TRIGGER cells_only_in_typed_columns_on_insert BEFORE INSERT ON cells
WHEN NOT EXISTS (SELECT 1 FROM sheet_columns WHERE id = NEW.column_id AND kind IN ('info', 'test'))
BEGIN
    SELECT RAISE(ABORT, 'only info and test columns hold typed cells');
END;

CREATE TRIGGER cells_only_in_typed_columns_on_update BEFORE UPDATE OF column_id ON cells
WHEN NOT EXISTS (SELECT 1 FROM sheet_columns WHERE id = NEW.column_id AND kind IN ('info', 'test'))
BEGIN
    SELECT RAISE(ABORT, 'only info and test columns hold typed cells');
END;
