-- A pupil's picture, one at most, shared like the pupil by every class they are in. It is a small
-- square JPEG the app makes (src/gradebook/pictures.rs), kept in the database so it is encrypted and
-- backed up with the marks. A table of its own, so reading names never reads pictures; its key is
-- the pupil, the only determinant, so it is in BCNF (D11 in docs/design-decisions.md).
CREATE TABLE pupil_pictures (
    pupil_id  INTEGER PRIMARY KEY REFERENCES pupils (id) ON DELETE CASCADE,
    image     BLOB NOT NULL CHECK (typeof(image) = 'blob' AND length(image) BETWEEN 1 AND 262144)
);
