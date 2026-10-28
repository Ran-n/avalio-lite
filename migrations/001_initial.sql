-- Initial schema: classes, pupils shared between classes, and one sheet per class.
-- Single local user, so no accounts or ownership columns.

CREATE TABLE classes (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE TABLE pupils (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

-- A pupil can be in several classes; position is the row order in that class's sheet.
CREATE TABLE class_pupils (
    class_id  INTEGER NOT NULL REFERENCES classes (id) ON DELETE CASCADE,
    pupil_id  INTEGER NOT NULL REFERENCES pupils (id) ON DELETE CASCADE,
    position  INTEGER NOT NULL,
    PRIMARY KEY (class_id, pupil_id)
);

-- One column of a class's sheet. Which settings apply depends on the kind:
--   info      free text, no settings
--   test      a score out of max_score, counting weight % towards the final grade
--   weighted  final grade: sum of weight% x (score / test max), shown out of max_score
--   average   mean of the graded tests' score / max, shown out of max_score
--   round     source_column_id's value rounded to `decimals` places
-- Computed kinds (weighted, average, round) never store cells.
CREATE TABLE sheet_columns (
    id                INTEGER PRIMARY KEY,
    class_id          INTEGER NOT NULL REFERENCES classes (id) ON DELETE CASCADE,
    position          INTEGER NOT NULL,
    title             TEXT NOT NULL,
    kind              TEXT NOT NULL CHECK (kind IN ('info', 'test', 'weighted', 'average', 'round')),
    weight            REAL,
    max_score         REAL,
    source_column_id  INTEGER REFERENCES sheet_columns (id) ON DELETE SET NULL,
    decimals          INTEGER,
    CHECK (CASE kind
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

-- Only non-empty values are stored; an empty cell has no row.
CREATE TABLE cells (
    column_id  INTEGER NOT NULL REFERENCES sheet_columns (id) ON DELETE CASCADE,
    pupil_id   INTEGER NOT NULL REFERENCES pupils (id) ON DELETE CASCADE,
    value      TEXT NOT NULL CHECK (value <> ''),
    PRIMARY KEY (column_id, pupil_id)
);

-- Taking a pupil out of a class removes their marks in that class only.
CREATE TRIGGER remove_cells_when_pupil_leaves_class AFTER DELETE ON class_pupils
BEGIN
    DELETE FROM cells
    WHERE pupil_id = OLD.pupil_id
      AND column_id IN (SELECT id FROM sheet_columns WHERE class_id = OLD.class_id);
END;

CREATE INDEX idx_class_pupils_pupil ON class_pupils (pupil_id);
CREATE INDEX idx_sheet_columns_class ON sheet_columns (class_id, position);
CREATE INDEX idx_sheet_columns_source ON sheet_columns (source_column_id);
CREATE INDEX idx_cells_pupil ON cells (pupil_id);
