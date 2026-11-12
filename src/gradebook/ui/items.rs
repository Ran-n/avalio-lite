// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/03 11:13:31.148480
//+ Revised: 	2026/11/11 17:10:59.465131
// -----------------------------------------------------------------------------

//! Turns the gradebook's values into the structs the Slint sheet shows, and the sheet's choices back.

use super::view::{ui_id, ui_width};
use crate::gradebook::compute::{Tone, depends_on_weights, evaluate_row, format_number, weights_add_up};
use crate::gradebook::model::{Column, ColumnKind, ColumnType, DuplicateContents, SheetRow};
use crate::i18n;
use crate::ui::{self, CellItem, ColumnItem, RowItem};
use crate::window::view::model;

const NO_SOURCE: i32 = -1;

/// Columns a rounded column may round, in sheet order; the round-source picker's indexes refer to this.
pub fn round_sources(columns: &[Column]) -> impl Iterator<Item = &Column> {
    columns.iter().filter(|column| column.kind.can_be_rounded())
}

pub fn column_item(column: &Column, columns: &[Column]) -> ColumnItem {
    let (weight, max_score, source_index, decimals) = match column.kind {
        ColumnKind::FirstName | ColumnKind::Surname | ColumnKind::Info => {
            (String::new(), String::new(), NO_SOURCE, String::new())
        }
        ColumnKind::Test { weight, max_score, .. } => (number(weight), number(max_score), NO_SOURCE, String::new()),
        ColumnKind::Weighted { max_score } | ColumnKind::Average { max_score } => {
            (String::new(), number(max_score), NO_SOURCE, String::new())
        }
        ColumnKind::Round { source, decimals } => {
            let index = round_sources(columns).position(|candidate| Some(candidate.id) == source);
            (String::new(), String::new(), index.map_or(NO_SOURCE, |index| index as i32), decimals.to_string())
        }
    };
    ColumnItem {
        id: ui_id(column.id),
        kind: to_ui_type(column.kind.column_type()),
        title: column_title(column).into(),
        weight: weight.into(),
        max_score: max_score.into(),
        source_index,
        decimals: decimals.into(),
        bonus: matches!(column.kind, ColumnKind::Test { bonus: true, .. }),
        width: ui_width(column.width),
    }
}

/// A column's title as the sheet shows it: a name column's follows the interface language.
pub fn column_title(column: &Column) -> String {
    i18n::name_column_title(i18n::current(), column.kind.column_type())
        .map_or_else(|| column.title.clone(), str::to_owned)
}

pub fn row_item(row: &SheetRow, columns: &[Column]) -> RowItem {
    RowItem {
        pupil_id: ui_id(row.pupil.id),
        first_name: row.pupil.first_name.as_str().into(),
        surname: row.pupil.surname.as_str().into(),
        full_name: row.pupil.full_name().into(),
        cells: model(cell_items(row, columns)),
    }
}

pub fn cell_items(row: &SheetRow, columns: &[Column]) -> Vec<CellItem> {
    let language = i18n::current();
    let weights_ok = weights_add_up(columns);
    evaluate_row(columns, &row.values)
        .into_iter()
        .zip(columns)
        .map(|(cell, column)| {
            // typed cells keep exactly what was typed; calculated ones use the language's decimal separator
            let text = if column.kind.is_computed() { i18n::localize_number(language, &cell.text) } else { cell.text };
            CellItem {
                text: text.into(),
                invalid: cell.is_invalid,
                tone: to_ui_tone(cell.tone),
                provisional: !weights_ok && depends_on_weights(columns, column),
            }
        })
        .collect()
}

/// A setting or total formatted for display in the current language.
pub fn number(value: f64) -> String {
    i18n::localize_number(i18n::current(), &format_number(value))
}

pub fn to_ui_tone(tone: Tone) -> ui::Tone {
    match tone {
        Tone::Neutral => ui::Tone::Neutral,
        Tone::Pass => ui::Tone::Pass,
        Tone::Fail => ui::Tone::Fail,
    }
}

pub fn to_ui_type(column_type: ColumnType) -> ui::ColumnType {
    match column_type {
        ColumnType::FirstName => ui::ColumnType::FirstName,
        ColumnType::Surname => ui::ColumnType::Surname,
        ColumnType::Info => ui::ColumnType::Info,
        ColumnType::Test => ui::ColumnType::Test,
        ColumnType::Weighted => ui::ColumnType::Weighted,
        ColumnType::Average => ui::ColumnType::Average,
        ColumnType::Round => ui::ColumnType::Round,
    }
}

/// What the folded sidebar shows of a class: its first 3 characters (not bytes: "º" is two),
/// trimmed, since class names here are short codes ("7A", "1º B") that stay recognisable so.
pub fn short_class_name(name: &str) -> String {
    name.trim().chars().take(3).collect::<String>().trim().to_owned()
}

pub fn from_ui_type(column_type: ui::ColumnType) -> ColumnType {
    match column_type {
        ui::ColumnType::FirstName => ColumnType::FirstName,
        ui::ColumnType::Surname => ColumnType::Surname,
        ui::ColumnType::Info => ColumnType::Info,
        ui::ColumnType::Test => ColumnType::Test,
        ui::ColumnType::Weighted => ColumnType::Weighted,
        ui::ColumnType::Average => ColumnType::Average,
        ui::ColumnType::Round => ColumnType::Round,
    }
}

pub fn from_ui_contents(contents: ui::DuplicateContents) -> DuplicateContents {
    match contents {
        ui::DuplicateContents::Columns => DuplicateContents::Columns,
        ui::DuplicateContents::ColumnsAndPupils => DuplicateContents::ColumnsAndPupils,
        ui::DuplicateContents::Everything => DuplicateContents::Everything,
    }
}

#[cfg(test)]
mod tests {
    use super::short_class_name;

    #[test]
    fn short_class_names_are_the_first_three_characters_trimmed() {
        assert_eq!(short_class_name("7A"), "7A");
        assert_eq!(short_class_name("1º ESO A"), "1º", "characters, not bytes, and the space is trimmed");
        assert_eq!(short_class_name("  Matemáticas"), "Mat");
        assert_eq!(short_class_name(""), "");
        assert_eq!(short_class_name("🍎🍐🍊🍋"), "🍎🍐🍊", "multi-byte characters don't panic");
    }
}
