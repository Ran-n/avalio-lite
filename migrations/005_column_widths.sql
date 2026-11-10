-- Column widths the user set by dragging a column's edge. NULL means "the default for the kind"
-- (the pupil column, a test or calculated column, a text column), so a column never resized
-- follows a later change of the default. Each width is a fact about one column or one class, so
-- it sits in that row.
ALTER TABLE sheet_columns ADD COLUMN width INTEGER CHECK (width IS NULL OR width BETWEEN 72 AND 640);
ALTER TABLE classes ADD COLUMN name_width INTEGER CHECK (name_width IS NULL OR name_width BETWEEN 120 AND 640);
