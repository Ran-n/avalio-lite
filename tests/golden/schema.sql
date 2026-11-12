CREATE TABLE cells (
    column_id  INTEGER NOT NULL REFERENCES sheet_columns (id) ON DELETE CASCADE,
    pupil_id   INTEGER NOT NULL REFERENCES pupils (id) ON DELETE CASCADE,
    value      TEXT NOT NULL CHECK (value <> ''),
    PRIMARY KEY (column_id, pupil_id)
);

CREATE TRIGGER cells_keep_a_pupil_of_the_class BEFORE UPDATE OF column_id, pupil_id ON cells
WHEN NOT EXISTS (
    SELECT 1 FROM class_pupils cp JOIN sheet_columns sc ON sc.class_id = cp.class_id
    WHERE sc.id = NEW.column_id AND cp.pupil_id = NEW.pupil_id
)
BEGIN
    SELECT RAISE(ABORT, 'a cell must belong to a pupil of its column''s class');
END;

CREATE TRIGGER cells_need_a_pupil_of_the_class BEFORE INSERT ON cells
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

CREATE TABLE class_pupils (
    class_id  INTEGER NOT NULL REFERENCES classes (id) ON DELETE CASCADE,
    pupil_id  INTEGER NOT NULL REFERENCES pupils (id) ON DELETE CASCADE,
    position  INTEGER NOT NULL,
    PRIMARY KEY (class_id, pupil_id)
);

CREATE TABLE classes (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE TRIGGER classes_get_their_name_columns AFTER INSERT ON classes
BEGIN
    INSERT INTO sheet_columns (class_id, position, title, kind) VALUES (NEW.id, 1, '', 'first_name');
    INSERT INTO sheet_columns (class_id, position, title, kind) VALUES (NEW.id, 2, '', 'surname');
END;

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

CREATE INDEX idx_cells_pupil ON cells (pupil_id);

CREATE INDEX idx_class_pupils_pupil ON class_pupils (pupil_id);

CREATE INDEX idx_sheet_columns_class ON sheet_columns (class_id, position);

CREATE INDEX idx_sheet_columns_source ON sheet_columns (source_column_id);

CREATE TRIGGER name_columns_keep_their_kind BEFORE UPDATE OF kind ON sheet_columns
WHEN NEW.kind <> OLD.kind AND (OLD.kind IN ('first_name', 'surname') OR NEW.kind IN ('first_name', 'surname'))
BEGIN
    SELECT RAISE(ABORT, 'a name column stays a name column, and no other column becomes one');
END;

CREATE TRIGGER name_columns_stay BEFORE DELETE ON sheet_columns
WHEN OLD.kind IN ('first_name', 'surname') AND EXISTS (SELECT 1 FROM classes WHERE id = OLD.class_id)
BEGIN
    SELECT RAISE(ABORT, 'a class keeps its first-name and surname columns');
END;

CREATE UNIQUE INDEX one_name_column_of_each_kind ON sheet_columns (class_id, kind)
    WHERE kind IN ('first_name', 'surname');

CREATE TABLE pupils (
    id          INTEGER PRIMARY KEY,
    first_name        TEXT NOT NULL,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
, surname TEXT NOT NULL DEFAULT '');

CREATE TRIGGER remove_cells_when_pupil_leaves_class AFTER DELETE ON class_pupils
BEGIN
    DELETE FROM cells
    WHERE pupil_id = OLD.pupil_id
      AND column_id IN (SELECT id FROM sheet_columns WHERE class_id = OLD.class_id);
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

CREATE TABLE "sheet_columns" (
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
