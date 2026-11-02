// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/28 09:31:17.615582
//+ Revised: 	2026/11/02 14:00:09.243596
// -----------------------------------------------------------------------------

use std::io;

use super::Language;
use super::shortcuts::{action_label, display};
use crate::error::{AppError, Folder, NumberField};
use crate::shortcuts::{ShortcutError, ShortcutProblem};

/// The error as shown to the user, in their language. Paths are passed through as they are; the
/// system's own error text never is (it is English and technical): errors from files, the
/// database and the settings are worded by what went wrong, and their details go to the log.
pub fn error_message(language: Language, error: &AppError) -> String {
    match language {
        Language::English => english(error),
        Language::Galician => galician(error),
    }
}

fn english(error: &AppError) -> String {
    match error {
        AppError::Io { path, source } => {
            let path = path.display();
            match IoProblem::of(source) {
                IoProblem::NotFound => format!("Couldn't find {path}."),
                IoProblem::Denied => {
                    format!("avalio-lite isn't allowed to use {path}. Choose another place, or check its permissions.")
                }
                IoProblem::InUse => format!("{path} is being used by another program. Close it and try again."),
                IoProblem::Full => {
                    format!("There isn't enough free space for {path}. Free some space or choose another place.")
                }
                IoProblem::Other => {
                    format!(
                        "Couldn't access {path}. Check that it is available and try again; the log has the details."
                    )
                }
            }
        }
        AppError::Database(source) => match DatabaseProblem::of(source) {
            DatabaseProblem::Busy => {
                "The database is being used by another program. Close it and try again.".to_owned()
            }
            DatabaseProblem::Damaged => "The database file is damaged, or isn't an avalio-lite database.".to_owned(),
            DatabaseProblem::Full => {
                "There isn't enough free space for the database. Free some space and try again.".to_owned()
            }
            DatabaseProblem::Other => {
                "Something went wrong with the database. Try again; the log has the details.".to_owned()
            }
        },
        AppError::ConfigParse { path, .. } => {
            format!("The settings file {} is damaged and can't be read.", path.display())
        }
        AppError::ConfigSerialize(_) => "Couldn't save your settings. Try again; the log has the details.".to_owned(),
        AppError::NoConfigDir => "Couldn't find your settings folder.".to_owned(),
        AppError::EmptyPath(Folder::Database) => "Choose a folder for the database.".to_owned(),
        AppError::EmptyPath(Folder::Logs) => "Choose a folder for the logs.".to_owned(),
        AppError::NotFound(path) => format!("{} doesn't exist.", path.display()),
        AppError::AlreadyExists(path) => format!("{} already exists.", path.display()),
        AppError::NewerSchema { found, supported } => format!(
            "This database was made by a newer version of avalio-lite (schema {found}; this version reads up to {supported}). Update the app to open it."
        ),
        AppError::IntegrityCheckFailed(path) => format!("The database at {} is damaged.", path.display()),
        AppError::NotADatabase(path) => format!("{} isn't an avalio-lite backup or database.", path.display()),
        AppError::KeptAside { kept, path } => {
            let (kept, path) = (kept.display(), path.display());
            format!(
                "That didn't work, and your data couldn't be put back in place. It is safe in {kept}. With the app closed, move away anything at {path}, then give the safe copy that name to use it again."
            )
        }
        AppError::NoDatabaseOpen => "No database is open.".to_owned(),
        AppError::EmptyName => "A name can't be empty.".to_owned(),
        AppError::DuplicateClassName(name) => format!("There's already a class called \"{name}\"."),
        AppError::InvalidNumber { field: NumberField::Weight, value } => {
            format!("\"{value}\" isn't a valid weight. Use a number from 0 to 1000.")
        }
        AppError::InvalidNumber { field: NumberField::MaxScore, value } => {
            format!("\"{value}\" isn't a valid maximum. Use a number above 0 and up to 10000.")
        }
        AppError::InvalidNumber { field: NumberField::Decimals, value } => {
            format!("\"{value}\" isn't a valid number of decimals. Use 0, 1, 2 or 3.")
        }
        AppError::NotEditable => "That value can't be edited.".to_owned(),
        AppError::InvalidRoundSource => {
            "A rounded column can only round a test, final grade or average of the same class.".to_owned()
        }
        AppError::InvalidPin => "A PIN is 4 to 128 characters: digits, letters, spaces or symbols.".to_owned(),
        AppError::PinMismatch => "The two PINs don't match. Type the same PIN twice.".to_owned(),
        AppError::WrongPin => "Wrong PIN.".to_owned(),
        AppError::PinWait { seconds } => format!("Too many wrong PINs. Try again in {seconds} s."),
        AppError::InvalidShortcuts(problems) => format!(
            "Some keyboard shortcuts in config.toml weren't valid and use their defaults: {}.",
            entries(problems)
        ),
        AppError::Shortcut(ShortcutError::UnknownKey) => "That key can't be used for a shortcut.".to_owned(),
        AppError::Shortcut(ShortcutError::NeedsCtrlOrAlt) => {
            "Add Ctrl or Alt: a plain letter would type into the cell.".to_owned()
        }
        AppError::Shortcut(ShortcutError::CtrlAlt) => {
            "Ctrl+Alt is the AltGr key on many keyboards, so it can't be used.".to_owned()
        }
        AppError::Shortcut(ShortcutError::EditingKey(letter)) => format!("Ctrl+{letter} is used for editing text."),
        AppError::Shortcut(ShortcutError::Clash { keys, taken_by }) => format!(
            "{} is already used by {}.",
            display(Language::English, keys),
            action_label(Language::English, *taken_by)
        ),
        AppError::SpreadsheetWrite { path, .. } => {
            format!("Couldn't save {}. Check that it isn't open in Excel and that you can save there.", path.display())
        }
        AppError::SpreadsheetRead { path, .. } => format!(
            "{} isn't a workbook that can be read. Check that it is an Excel .xlsx file and isn't damaged.",
            path.display()
        ),
        AppError::NoPupilsInFile(path) => {
            format!("{} has no pupils: names go in column A, from row 2.", path.display())
        }
        AppError::DuplicatePupilInFile(name) => {
            format!("The file has \"{name}\" twice; each pupil can only be in it once.")
        }
    }
}

/// The ignored entries as written in `config.toml`, the same in every language.
fn entries(problems: &[ShortcutProblem]) -> String {
    problems.iter().map(|problem| problem.entry.as_str()).collect::<Vec<_>>().join(", ")
}

fn galician(error: &AppError) -> String {
    match error {
        AppError::Io { path, source } => {
            let path = path.display();
            match IoProblem::of(source) {
                IoProblem::NotFound => format!("Non se atopou {path}."),
                IoProblem::Denied => format!(
                    "avalio-lite non ten permiso para usar {path}. Escolle outro sitio ou revisa os seus permisos."
                ),
                IoProblem::InUse => format!("Outro programa está a usar {path}. Péchao e proba de novo."),
                IoProblem::Full => {
                    format!("Non hai espazo libre abondo para {path}. Libera espazo ou escolle outro sitio.")
                }
                IoProblem::Other => format!(
                    "Non se puido acceder a {path}. Comproba que está dispoñible e proba de novo; o rexistro ten os detalles."
                ),
            }
        }
        AppError::Database(source) => match DatabaseProblem::of(source) {
            DatabaseProblem::Busy => "Outro programa está a usar a base de datos. Péchao e proba de novo.".to_owned(),
            DatabaseProblem::Damaged => {
                "O ficheiro da base de datos está danado ou non é unha base de datos de avalio-lite.".to_owned()
            }
            DatabaseProblem::Full => {
                "Non hai espazo libre abondo para a base de datos. Libera espazo e proba de novo.".to_owned()
            }
            DatabaseProblem::Other => {
                "Algo fallou coa base de datos. Proba de novo; o rexistro ten os detalles.".to_owned()
            }
        },
        AppError::ConfigParse { path, .. } => {
            format!("O ficheiro de axustes {} está danado e non se pode ler.", path.display())
        }
        AppError::ConfigSerialize(_) => {
            "Non se puideron gardar os teus axustes. Proba de novo; o rexistro ten os detalles.".to_owned()
        }
        AppError::NoConfigDir => "Non se atopou o teu cartafol de axustes.".to_owned(),
        AppError::EmptyPath(Folder::Database) => "Escolle un cartafol para a base de datos.".to_owned(),
        AppError::EmptyPath(Folder::Logs) => "Escolle un cartafol para os rexistros.".to_owned(),
        AppError::NotFound(path) => format!("{} non existe.", path.display()),
        AppError::AlreadyExists(path) => format!("{} xa existe.", path.display()),
        AppError::NewerSchema { found, supported } => format!(
            "Esta base de datos creouse cunha versión máis nova de avalio-lite (esquema {found}; esta versión le ata o {supported}). Actualiza a aplicación para abrila."
        ),
        AppError::IntegrityCheckFailed(path) => format!("A base de datos de {} está danada.", path.display()),
        AppError::NotADatabase(path) => {
            format!("{} non é unha copia nin unha base de datos de avalio-lite.", path.display())
        }
        AppError::KeptAside { kept, path } => {
            let (kept, path) = (kept.display(), path.display());
            format!(
                "Non funcionou, e os teus datos non se puideron devolver ao seu sitio. Están a salvo en {kept}. Coa aplicación pechada, quita o que haxa en {path} e despois dálle ese nome á copia gardada para volver usala."
            )
        }
        AppError::NoDatabaseOpen => "Non hai ningunha base de datos aberta.".to_owned(),
        AppError::EmptyName => "O nome non pode quedar baleiro.".to_owned(),
        AppError::DuplicateClassName(name) => format!("Xa hai unha clase chamada «{name}»."),
        AppError::InvalidNumber { field: NumberField::Weight, value } => {
            format!("«{value}» non é un peso válido. Usa un número de 0 a 1000.")
        }
        AppError::InvalidNumber { field: NumberField::MaxScore, value } => {
            format!("«{value}» non é un máximo válido. Usa un número maior ca 0 e ata 10000.")
        }
        AppError::InvalidNumber { field: NumberField::Decimals, value } => {
            format!("«{value}» non é un número de decimais válido. Usa 0, 1, 2 ou 3.")
        }
        AppError::NotEditable => "Ese valor non se pode editar.".to_owned(),
        AppError::InvalidRoundSource => {
            "Unha columna de redondeo só pode redondear unha proba, unha nota final ou unha media da mesma clase."
                .to_owned()
        }
        AppError::InvalidPin => "O PIN ten de 4 a 128 caracteres: números, letras, espazos ou símbolos.".to_owned(),
        AppError::PinMismatch => "Os dous PIN non coinciden. Escribe o mesmo PIN dúas veces.".to_owned(),
        AppError::WrongPin => "PIN incorrecto.".to_owned(),
        AppError::PinWait { seconds } => format!("Demasiados PIN incorrectos. Proba de novo en {seconds} s."),
        AppError::InvalidShortcuts(problems) => format!(
            "Algúns atallos de teclado de config.toml non eran válidos e usan os predeterminados: {}.",
            entries(problems)
        ),
        AppError::Shortcut(ShortcutError::UnknownKey) => "Esa tecla non se pode usar nun atallo.".to_owned(),
        AppError::Shortcut(ShortcutError::NeedsCtrlOrAlt) => {
            "Engade Ctrl ou Alt: unha letra soa escribiríase na cela.".to_owned()
        }
        AppError::Shortcut(ShortcutError::CtrlAlt) => {
            "Ctrl+Alt é a tecla AltGr en moitos teclados, así que non se pode usar.".to_owned()
        }
        AppError::Shortcut(ShortcutError::EditingKey(letter)) => format!("Ctrl+{letter} úsase para editar texto."),
        AppError::Shortcut(ShortcutError::Clash { keys, taken_by }) => format!(
            "{} xa se usa para {}.",
            display(Language::Galician, keys),
            action_label(Language::Galician, *taken_by)
        ),
        AppError::SpreadsheetWrite { path, .. } => format!(
            "Non se puido gardar {}. Comproba que non está aberto en Excel e que podes gardar alí.",
            path.display()
        ),
        AppError::SpreadsheetRead { path, .. } => format!(
            "{} non é un libro que se poida ler. Comproba que é un ficheiro .xlsx de Excel e que non está danado.",
            path.display()
        ),
        AppError::NoPupilsInFile(path) => {
            format!("{} non ten alumnos: os nomes van na columna A, desde a fila 2.", path.display())
        }
        AppError::DuplicatePupilInFile(name) => {
            format!("O ficheiro ten «{name}» dúas veces; cada alumno só pode estar nel unha vez.")
        }
    }
}

/// What went wrong with a file, as far as the user can act on it.
enum IoProblem {
    NotFound,
    Denied,
    InUse,
    Full,
    Other,
}

impl IoProblem {
    fn of(error: &io::Error) -> Self {
        // Windows: 32 and 33 are sharing and lock violations, 39 and 112 a full disk
        match (error.kind(), error.raw_os_error()) {
            (io::ErrorKind::NotFound, _) => Self::NotFound,
            (_, Some(32 | 33)) => Self::InUse,
            (io::ErrorKind::PermissionDenied, _) => Self::Denied,
            (io::ErrorKind::StorageFull, _) | (_, Some(39 | 112)) => Self::Full,
            _ => Self::Other,
        }
    }
}

/// What went wrong with the database, as far as the user can act on it.
enum DatabaseProblem {
    Busy,
    Damaged,
    Full,
    Other,
}

impl DatabaseProblem {
    fn of(error: &rusqlite::Error) -> Self {
        use rusqlite::ErrorCode;
        match error.sqlite_error_code() {
            Some(ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked) => Self::Busy,
            Some(ErrorCode::NotADatabase | ErrorCode::DatabaseCorrupt) => Self::Damaged,
            Some(ErrorCode::DiskFull) => Self::Full,
            _ => Self::Other,
        }
    }
}
