// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/29 10:48:30.870291
//+ Revised: 	2026/11/11 17:11:00.872714
// -----------------------------------------------------------------------------

//! The sheet's Excel menu entries: export the open class to a workbook, and import pupils and
//! marks from one.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use slint::ComponentHandle;

use crate::gradebook::ui::run;
use crate::gradebook::ui::view::{Refresh, current_class_id};
use crate::i18n::{self, Language, Message};
use crate::spreadsheet::export;
use crate::spreadsheet::import::{self, ImportSummary, LeftOut};
use crate::ui::{AppWindow, Gradebook};
use crate::window::dialogs::FileKind;
use crate::window::view::show_status;
use crate::window::{SharedDialogs, SharedState, with_window};

pub fn wire(window: &AppWindow, state: &SharedState, dialogs: &SharedDialogs) {
    wire_export(window, state, dialogs);
    wire_import(window, state, dialogs);
}

fn wire_export(window: &AppWindow, state: &SharedState, dialogs: &SharedDialogs) {
    // the folder of the last export in this session, where the next one starts
    let last_folder: Rc<RefCell<Option<PathBuf>>> = Rc::default();
    let (shared, dialogs) = (state.clone(), dialogs.clone());
    window.global::<Gradebook>().on_export_sheet(with_window(window, move |window| {
        let class = window.global::<Gradebook>().get_class_name();
        let start = last_folder.borrow().clone().or_else(dirs::document_dir).unwrap_or_default();
        let proposed = format!("{}.{}", export::file_name(&class), FileKind::Spreadsheet.save_extensions()[0]);
        let Some(path) = dialogs.save_file(FileKind::Spreadsheet, &start, &proposed) else { return };
        let class_id = current_class_id(window);
        let exported = run(window, &shared, "export sheet", |connection| {
            export::export_sheet(connection, class_id, i18n::current(), &path)?;
            Ok(Refresh::Nothing)
        });
        if exported {
            *last_folder.borrow_mut() = path.parent().map(Path::to_path_buf);
            let shown = path.display().to_string();
            show_status(window, i18n::text(i18n::current(), &Message::SheetExported { class: &class, path: &shown }));
        }
    }));
}

/// Reads and plans first, so the question about replacing marks comes before anything changes.
fn wire_import(window: &AppWindow, state: &SharedState, dialogs: &SharedDialogs) {
    let (shared, dialogs) = (state.clone(), dialogs.clone());
    window.global::<Gradebook>().on_import_sheet(with_window(window, move |window| {
        let Some(path) = dialogs.pick_file(FileKind::Spreadsheet) else { return };
        let class = window.global::<Gradebook>().get_class_name();
        let class_id = current_class_id(window);
        let mut plan = None;
        run(window, &shared, "read import", |connection| {
            let (first_name, surname): (Vec<&str>, Vec<&str>) =
                (i18n::first_name_headers().collect(), i18n::surname_headers().collect());
            let names = import::NameHeaders { first_name: &first_name, surname: &surname };
            let table = import::read_table(&path, &names)?;
            plan = Some(import::plan_import(connection, class_id, &table)?);
            Ok(Refresh::Nothing)
        });
        let Some(plan) = plan else { return };
        let language = i18n::current();
        if plan.overwrites > 0 {
            let question = Message::ConfirmImportOverwrite { count: plan.overwrites, class: &class };
            if !dialogs.confirm(&i18n::text(language, &question)) {
                log::info!("import into class {class_id} cancelled before replacing {} values", plan.overwrites);
                return;
            }
        }
        let mut summary = None;
        let imported = run(window, &shared, "import sheet", |connection| {
            summary = Some(import::apply_import(connection, class_id, &plan)?);
            Ok(Refresh::All { select: Some(class_id) })
        });
        if let (true, Some(summary)) = (imported, summary) {
            show_status(window, import_report(language, &class, &summary));
        }
    }));
}

/// What the import did, then what it left out, each sentence only when it has something to say.
fn import_report(language: Language, class: &str, summary: &ImportSummary) -> String {
    let done =
        Message::ImportDone { pupils: summary.pupils, new_pupils: summary.new_pupils, marks: summary.marks, class };
    let mut report = vec![i18n::text(language, &done)];
    if !summary.left_out.is_empty() {
        let headers: Vec<String> = summary.left_out.iter().map(|left_out| left_out_name(language, left_out)).collect();
        let left_out = Message::ImportLeftOut { headers: &headers.join(", "), count: headers.len() };
        report.push(i18n::text(language, &left_out));
    }
    if summary.error_cells > 0 {
        report.push(i18n::text(language, &Message::ImportErrorCells { count: summary.error_cells }));
    }
    report.join(" ")
}

fn left_out_name(language: Language, left_out: &LeftOut) -> String {
    match left_out {
        LeftOut::Header(header) => header.clone(),
        LeftOut::Untitled(letter) => i18n::text(language, &Message::UntitledColumn { letter }),
    }
}
