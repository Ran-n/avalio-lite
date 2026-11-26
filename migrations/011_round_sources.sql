-- A rounded column's source used to be a column of `sheet_columns` (`source_column_id`), and the
-- triggers of 003 and 007 forced it into the same class. So for a rounded row `source_column_id`
-- decided its `class_id`: a transitive dependency, and `sheet_columns` was not in BCNF (D11 in
-- docs/design-decisions.md). The link now has a table of its own, keyed by the rounded column. The
-- class is reached only through the two columns the link joins, and nothing is copied.
--
-- An old file can hold a link the new rules refuse. The old triggers checked a rounded column when
-- it moved, but not its source: `UPDATE sheet_columns SET class_id` on the source was allowed, and
-- so was a change of the source's kind if the row still passed its CHECK. Such a link is dropped,
-- not moved and not refused: a refused migration would keep the whole file from opening. The
-- rounded column keeps its decimals and shows no value until it is linked again. Each dropped link
-- is written to `temp.migration_notes`, which the migration runner logs as a warning.
--
-- SQLite can't change a CHECK in place, so `sheet_columns` is rebuilt the way 007 rebuilt it (create
-- the new table, copy, drop the old, rename). The runner turns foreign keys off around this file and
-- checks the references with `foreign_key_check` before committing.

-- Triggers on other tables that name sheet_columns would fail the rename, and the ones on
-- sheet_columns go with the old table. All of them are made again below, unchanged, except the two
-- rounding triggers, which are replaced by the ones on round_sources.
DROP TRIGGER cells_keep_a_pupil_of_the_class;
DROP TRIGGER cells_need_a_pupil_of_the_class;
DROP TRIGGER cells_only_in_typed_columns_on_insert;
DROP TRIGGER cells_only_in_typed_columns_on_update;
DROP TRIGGER remove_cells_when_pupil_leaves_class;
DROP TRIGGER classes_get_their_name_columns;
DROP TRIGGER round_source_on_insert;
DROP TRIGGER round_source_on_update;

-- The links the new rules refuse, written down before they are left out.
INSERT INTO temp.migration_notes (message)
SELECT 'rounded column ' || r.id || ' lost its link to column ' || r.source_column_id
    || ': it must round a test, final grade or average of its own class'
FROM sheet_columns r
WHERE r.source_column_id IS NOT NULL
  AND NOT (r.kind = 'round' AND EXISTS (
      SELECT 1 FROM sheet_columns s
      WHERE s.id = r.source_column_id AND s.class_id = r.class_id AND s.kind IN ('test', 'weighted', 'average')
  ));

-- One link per rounded column (its primary key), to a column of the same class. Deleting either
-- column deletes the link: the rounded column then shows no value, as ON DELETE SET NULL did.
CREATE TABLE round_sources (
    column_id         INTEGER PRIMARY KEY REFERENCES sheet_columns (id) ON DELETE CASCADE,
    source_column_id  INTEGER NOT NULL REFERENCES sheet_columns (id) ON DELETE CASCADE
);

CREATE INDEX idx_round_sources_source ON round_sources (source_column_id);

INSERT INTO round_sources (column_id, source_column_id)
SELECT r.id, r.source_column_id
FROM sheet_columns r
WHERE r.kind = 'round' AND r.source_column_id IS NOT NULL AND EXISTS (
    SELECT 1 FROM sheet_columns s
    WHERE s.id = r.source_column_id AND s.class_id = r.class_id AND s.kind IN ('test', 'weighted', 'average')
);

-- One column of a class's sheet, without the rounding source, which is in round_sources now. The
-- settings that apply depend on the kind, as before:
--   first_name, surname  the pupil's names, from `pupils`; no title (it follows the interface
--                        language), no settings, never cells; exactly one of each per class
--   info      free text, no settings
--   test      a score out of max_score, counting weight % towards the final grade
--   weighted  final grade: sum of weight% x (score / test max), shown out of max_score
--   average   mean of the graded tests' score / max, shown out of max_score
--   round     the source's value rounded to `decimals` places (round_sources)
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
    decimals          INTEGER,
    bonus             INTEGER NOT NULL DEFAULT 0 CHECK (bonus IN (0, 1) AND (bonus = 0 OR kind = 'test')),
    width             INTEGER CHECK (width IS NULL OR width BETWEEN 72 AND 640),
    CHECK (CASE kind
        WHEN 'first_name' THEN title = '' AND weight IS NULL AND max_score IS NULL
            AND decimals IS NULL
        WHEN 'surname' THEN title = '' AND weight IS NULL AND max_score IS NULL
            AND decimals IS NULL
        WHEN 'info' THEN weight IS NULL AND max_score IS NULL
            AND decimals IS NULL
        WHEN 'test' THEN weight IS NOT NULL AND weight >= 0
            AND max_score IS NOT NULL AND max_score > 0
            AND decimals IS NULL
        WHEN 'round' THEN weight IS NULL AND max_score IS NULL
            AND decimals IS NOT NULL AND decimals BETWEEN 0 AND 3
        ELSE weight IS NULL AND max_score IS NOT NULL AND max_score > 0
            AND decimals IS NULL
    END)
);

INSERT INTO sheet_columns_new (id, class_id, position, title, kind, weight, max_score, decimals, bonus, width)
SELECT id, class_id, position, title, kind, weight, max_score, decimals, bonus, width
FROM sheet_columns;

DROP TABLE sheet_columns;
ALTER TABLE sheet_columns_new RENAME TO sheet_columns;

CREATE INDEX idx_sheet_columns_class ON sheet_columns (class_id, position);
CREATE UNIQUE INDEX one_name_column_of_each_kind ON sheet_columns (class_id, kind)
    WHERE kind IN ('first_name', 'surname');

-- The triggers of 009 that were on sheet_columns, as they were.
CREATE TRIGGER column_limits_on_insert BEFORE INSERT ON sheet_columns
WHEN (NEW.weight IS NOT NULL AND NEW.weight > 1000)
  OR (NEW.max_score IS NOT NULL AND NEW.max_score > 10000)
BEGIN
    SELECT RAISE(ABORT, 'a weight is at most 1000 and a maximum score at most 10000');
END;

CREATE TRIGGER column_limits_on_update BEFORE UPDATE OF weight, max_score ON sheet_columns
WHEN (NEW.weight IS NOT NULL AND NEW.weight > 1000)
  OR (NEW.max_score IS NOT NULL AND NEW.max_score > 10000)
BEGIN
    SELECT RAISE(ABORT, 'a weight is at most 1000 and a maximum score at most 10000');
END;

CREATE TRIGGER columns_have_titles_on_insert BEFORE INSERT ON sheet_columns
WHEN NEW.kind NOT IN ('first_name', 'surname') AND trim(NEW.title) = ''
BEGIN
    SELECT RAISE(ABORT, 'a column other than the names needs a title');
END;

CREATE TRIGGER columns_have_titles_on_update BEFORE UPDATE OF title, kind ON sheet_columns
WHEN NEW.kind NOT IN ('first_name', 'surname') AND trim(NEW.title) = ''
BEGIN
    SELECT RAISE(ABORT, 'a column other than the names needs a title');
END;

CREATE TRIGGER name_columns_stay BEFORE DELETE ON sheet_columns
WHEN OLD.kind IN ('first_name', 'surname') AND EXISTS (SELECT 1 FROM classes WHERE id = OLD.class_id)
BEGIN
    SELECT RAISE(ABORT, 'a class keeps its first-name and surname columns');
END;

CREATE TRIGGER name_columns_keep_their_kind BEFORE UPDATE OF kind ON sheet_columns
WHEN NEW.kind <> OLD.kind AND (OLD.kind IN ('first_name', 'surname') OR NEW.kind IN ('first_name', 'surname'))
BEGIN
    SELECT RAISE(ABORT, 'a name column stays a name column, and no other column becomes one');
END;

-- The triggers of 007 that name sheet_columns, as they were.
CREATE TRIGGER classes_get_their_name_columns AFTER INSERT ON classes
BEGIN
    INSERT INTO sheet_columns (class_id, position, title, kind) VALUES (NEW.id, 1, '', 'first_name');
    INSERT INTO sheet_columns (class_id, position, title, kind) VALUES (NEW.id, 2, '', 'surname');
END;

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

-- A link joins a rounded column to a test, final grade or average of its own class. The file
-- refuses any other, however it is written.
CREATE TRIGGER round_sources_on_insert BEFORE INSERT ON round_sources
WHEN NOT EXISTS (
    SELECT 1 FROM sheet_columns r JOIN sheet_columns s ON s.class_id = r.class_id
    WHERE r.id = NEW.column_id AND r.kind = 'round'
      AND s.id = NEW.source_column_id AND s.kind IN ('test', 'weighted', 'average')
)
BEGIN
    SELECT RAISE(ABORT, 'a rounded column must round a test, final grade or average of its own class');
END;

CREATE TRIGGER round_sources_on_update BEFORE UPDATE OF column_id, source_column_id ON round_sources
WHEN NOT EXISTS (
    SELECT 1 FROM sheet_columns r JOIN sheet_columns s ON s.class_id = r.class_id
    WHERE r.id = NEW.column_id AND r.kind = 'round'
      AND s.id = NEW.source_column_id AND s.kind IN ('test', 'weighted', 'average')
)
BEGIN
    SELECT RAISE(ABORT, 'a rounded column must round a test, final grade or average of its own class');
END;

-- A column in a link stays in its class and keeps the kind the link needs. A rounded column must
-- stay rounded; a source must stay a test, final grade or average. Changes that break nothing pass,
-- including a source moving between those three kinds.
CREATE TRIGGER round_sources_keep_their_columns BEFORE UPDATE OF class_id, kind ON sheet_columns
WHEN (NEW.class_id <> OLD.class_id OR NEW.kind <> OLD.kind) AND (
    EXISTS (SELECT 1 FROM round_sources WHERE column_id = OLD.id)
    OR (EXISTS (SELECT 1 FROM round_sources WHERE source_column_id = OLD.id)
        AND (NEW.class_id <> OLD.class_id OR NEW.kind NOT IN ('test', 'weighted', 'average')))
)
BEGIN
    SELECT RAISE(ABORT, 'a rounded column and its source stay in their class, and keep the kinds their link needs');
END;
