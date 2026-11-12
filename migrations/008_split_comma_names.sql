-- Names typed surname first, the way school lists write them ("Pérez López, Ana", with or without
-- a space after the comma), are split into the surname and first name that migration 006 left
-- together. Unlike the guess 006 refused to make for "Xoán Carlos Pérez", the comma marks the split,
-- so this is no guess. Only pupils with no surname yet and exactly one comma with text on both
-- sides change; anything else (no comma, two commas, a side left empty, a surname already typed)
-- stays as it was. Spaces, tabs, line breaks and no-break spaces around each part are trimmed, as
-- the app trims a typed name. SQLite reads every right-hand side from the old row, so the surname
-- is cut from the unsplit name.
UPDATE pupils
SET surname    = trim(substr(first_name, 1, instr(first_name, ',') - 1), ' ' || char(9, 10, 13, 160)),
    first_name = trim(substr(first_name, instr(first_name, ',') + 1), ' ' || char(9, 10, 13, 160))
WHERE surname = ''
  AND instr(first_name, ',') > 0
  AND instr(substr(first_name, instr(first_name, ',') + 1), ',') = 0
  AND trim(substr(first_name, 1, instr(first_name, ',') - 1), ' ' || char(9, 10, 13, 160)) <> ''
  AND trim(substr(first_name, instr(first_name, ',') + 1), ' ' || char(9, 10, 13, 160)) <> '';
