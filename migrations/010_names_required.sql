-- Two more rules the app checked only in Rust (model.rs clean_name), now kept by the database too,
-- like the column titles of 009: a class has a name and a pupil a first name, neither empty nor
-- only spaces, so no path that writes these tables directly (a future feature, a merge, a hand
-- edit) can leave a row the sheet shows as blank. A surname may be empty: some pupils have one name.
-- Triggers rather than CHECKs, since SQLite can only add a CHECK by rebuilding the table.
--
-- Rows already there are left as they are: the app has never written an empty name, and one that
-- was edited in by hand still shows, to be renamed in the app.
CREATE TRIGGER classes_have_names_on_insert BEFORE INSERT ON classes
WHEN trim(NEW.name) = ''
BEGIN
    SELECT RAISE(ABORT, 'a class needs a name');
END;

CREATE TRIGGER classes_have_names_on_update BEFORE UPDATE OF name ON classes
WHEN trim(NEW.name) = ''
BEGIN
    SELECT RAISE(ABORT, 'a class needs a name');
END;

CREATE TRIGGER pupils_have_first_names_on_insert BEFORE INSERT ON pupils
WHEN trim(NEW.first_name) = ''
BEGIN
    SELECT RAISE(ABORT, 'a pupil needs a first name');
END;

CREATE TRIGGER pupils_have_first_names_on_update BEFORE UPDATE OF first_name ON pupils
WHEN trim(NEW.first_name) = ''
BEGIN
    SELECT RAISE(ABORT, 'a pupil needs a first name');
END;
