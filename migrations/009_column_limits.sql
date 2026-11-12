-- Rules the app checked only in Rust, now kept by the database too, so no path that writes
-- sheet_columns directly (a future feature, a merge, a hand edit) can break them:
--   * a weight is at most 1000 % and a maximum score at most 10 000 (columns.rs MAX_WEIGHT and
--     MAX_SCORE_LIMIT; the CHECKs of 007 only ask for >= 0 and > 0);
--   * every column but the names has a title (the names' title is '' and follows the language);
--   * a class keeps its two name columns: they can't be deleted while their class exists, nor
--     turned into another kind (the partial unique index of 007 only stops a second one).
-- Triggers rather than CHECKs, since SQLite can only add a CHECK by rebuilding the table.

-- The limits came in 0.4.0; a column typed before that may be over them, and would then fail the
-- triggers whenever it is copied (a duplicate, a backup added). It is brought to the limit, the
-- most it could be given since (the migration log says how many rows changed).
UPDATE sheet_columns SET weight = 1000 WHERE weight > 1000;
UPDATE sheet_columns SET max_score = 10000 WHERE max_score > 10000;

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

-- Deleting a class removes its columns through ON DELETE CASCADE after the class row is gone, so
-- the class no longer exists then and its name columns go with it.
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
