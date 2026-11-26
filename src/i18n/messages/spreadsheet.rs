// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 00:26:07.378811
//+ Revised: 	2026/11/23 16:34:36.012460
// -----------------------------------------------------------------------------

//! The spreadsheet topic of `Message`. Its texts are in `english/spreadsheet.rs` and
//! `galician/spreadsheet.rs`.

/// The texts about Excel: exports and imports, and what an import left out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpreadsheetMessage<'a> {
    SheetExported {
        class: &'a str,
        path: &'a str,
    },
    SpreadsheetFileType,
    /// the note under an exported sheet whose final grades are provisional; `total` is localized
    ExportWeightsWarning {
        total: &'a str,
    },
    ConfirmImportOverwrite {
        count: usize,
        class: &'a str,
    },
    ImportDone {
        pupils: usize,
        new_pupils: usize,
        marks: usize,
        class: &'a str,
    },
    /// the `count` headers an import ignored, already joined
    ImportLeftOut {
        headers: &'a str,
        count: usize,
    },
    ImportErrorCells {
        count: usize,
    },
    /// rows of the file with marks or a surname but no first name, skipped
    ImportUnnamedRows {
        count: usize,
    },
}
