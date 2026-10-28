-- The file itself now enforces two rules the app already kept:
--   a cell belongs to a pupil of its column's class, and
--   a rounded column rounds a test, final grade or average of its own class.
-- Cells that broke the first rule were never shown (the sheet only lists the class's pupils)
-- and would have resurfaced if the pupil joined the class later, so they are removed.
DELETE FROM cells
WHERE NOT EXISTS (
    SELECT 1 FROM class_pupils cp JOIN sheet_columns sc ON sc.class_id = cp.class_id
    WHERE sc.id = cells.column_id AND cp.pupil_id = cells.pupil_id
);

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
