-- The file itself now enforces one more rule the app already kept: only info and test columns
-- hold typed cells; final grade, average and rounded columns are always calculated.
-- Cells that broke it were never shown (calculated columns ignore stored values), so they go.
DELETE FROM cells
WHERE column_id IN (SELECT id FROM sheet_columns WHERE kind NOT IN ('info', 'test'));

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
