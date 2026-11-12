-- Pupils get a first name and a surname as separate fields, so a sheet can be read by surname and
-- matched against school files that keep the two apart. Names typed before this keep their whole
-- text as the first name, with no surname: guessing where a name splits would get some wrong, so
-- teachers fix the old ones by hand. The surname column's width is a fact about one class, like
-- the pupil column's `name_width` beside it, so it sits in the class's row.
ALTER TABLE pupils RENAME COLUMN name TO first_name;
ALTER TABLE pupils ADD COLUMN surname TEXT NOT NULL DEFAULT '';
ALTER TABLE classes ADD COLUMN surname_width INTEGER CHECK (surname_width IS NULL OR surname_width BETWEEN 72 AND 640);
