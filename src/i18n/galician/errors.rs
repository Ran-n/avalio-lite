// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 00:26:08.673886
//+ Revised: 	2026/11/25 11:43:06.810933
// -----------------------------------------------------------------------------

//! The error texts in Galician, grouped by what went wrong: the database, files and folders, the
//! fields and PIN, and keyboard shortcuts.

use crate::error::{AppError, Folder, NumberField};
use crate::i18n::Language;
use crate::i18n::errors::{DatabaseProblem, IoProblem, entries};
use crate::i18n::shortcuts::{action_label, display};
use crate::shortcuts::ShortcutError;

/// An error in Galician, as `error_message` describes. Each arm is sent to its group's function;
/// the groups match only their own errors, which this routing guarantees.
pub(in crate::i18n) fn error(error: &AppError) -> String {
    match error {
        AppError::Database(_)
        | AppError::NewerSchema { .. }
        | AppError::IntegrityCheckFailed(_)
        | AppError::NotADatabase(_)
        | AppError::OpenDatabaseAsBackup(_)
        | AppError::KeptAside { .. }
        | AppError::NoDatabaseOpen => database(error),
        AppError::Io { .. }
        | AppError::ConfigParse { .. }
        | AppError::ConfigSerialize(_)
        | AppError::NoConfigDir
        | AppError::EmptyPath(_)
        | AppError::RelativePath(_)
        | AppError::NotFound(_)
        | AppError::AlreadyExists(_)
        | AppError::SpreadsheetWrite { .. }
        | AppError::SpreadsheetRead { .. }
        | AppError::PictureRead { .. }
        | AppError::NoPupilsInFile(_)
        | AppError::DuplicatePupilInFile(_) => files(error),
        AppError::EmptyName
        | AppError::DuplicateClassName(_)
        | AppError::InvalidNumber { .. }
        | AppError::NotEditable
        | AppError::InvalidRoundSource
        | AppError::InvalidPin
        | AppError::PinMismatch
        | AppError::WrongPin
        | AppError::PinWait { .. } => fields(error),
        AppError::InvalidShortcuts(_) | AppError::Shortcut(_) => shortcuts(error),
    }
}

/// The database errors.
fn database(error: &AppError) -> String {
    match error {
        AppError::Database(source) => database_problem(source),
        AppError::NewerSchema { found, supported } => format!(
            "Esta base de datos creouse cunha versión máis nova de avalio lite (esquema {found}; esta versión le ata o {supported}). Actualiza a aplicación para abrila."
        ),
        AppError::IntegrityCheckFailed(path) => format!("A base de datos de {} está danada.", path.display()),
        AppError::NotADatabase(path) => {
            format!("{} non é unha copia nin unha base de datos de avalio lite.", path.display())
        }
        AppError::OpenDatabaseAsBackup(path) => format!(
            "{} é a base de datos que a aplicación está a usar agora, non unha copia. Escolle un ficheiro de copia.",
            path.display()
        ),
        AppError::KeptAside { kept, path } => {
            let (kept, path) = (kept.display(), path.display());
            format!(
                "Non funcionou, e os teus datos non se puideron devolver ao seu sitio. Están a salvo en {kept}. Coa aplicación pechada, quita o que haxa en {path} e despois dálle ese nome á copia gardada para volver usala."
            )
        }
        AppError::NoDatabaseOpen => "Non hai ningunha base de datos aberta.".to_owned(),
        _ => unreachable!("error() sends only database errors to database()"),
    }
}

/// What went wrong with the database, in words the user can act on.
fn database_problem(source: &rusqlite::Error) -> String {
    match DatabaseProblem::of(source) {
        DatabaseProblem::Busy => "Outro programa está a usar a base de datos. Péchao e proba de novo.".to_owned(),
        DatabaseProblem::Damaged => {
            "O ficheiro da base de datos está danado ou non é unha base de datos de avalio lite.".to_owned()
        }
        DatabaseProblem::Full => {
            "Non hai espazo libre abondo para a base de datos. Libera espazo e proba de novo.".to_owned()
        }
        DatabaseProblem::Other => {
            "Algo fallou coa base de datos. Proba de novo; o rexistro ten os detalles.".to_owned()
        }
    }
}

/// The errors about files, folders, the settings file and spreadsheets.
fn files(error: &AppError) -> String {
    match error {
        AppError::Io { path, source } => io_problem(path, source),
        AppError::ConfigParse { path, .. } => {
            format!("O ficheiro de axustes {} está danado e non se pode ler.", path.display())
        }
        AppError::ConfigSerialize(_) => {
            "Non se puideron gardar os teus axustes. Proba de novo; o rexistro ten os detalles.".to_owned()
        }
        AppError::NoConfigDir => "Non se atopou o teu cartafol de axustes.".to_owned(),
        AppError::EmptyPath(Folder::Database) => "Escolle un cartafol para a base de datos.".to_owned(),
        AppError::EmptyPath(Folder::Logs) => "Escolle un cartafol para os rexistros.".to_owned(),
        AppError::RelativePath(Folder::Database) => {
            r"Indica o cartafol da base de datos completo, coa súa unidade, como D:\Notas.".to_owned()
        }
        AppError::RelativePath(Folder::Logs) => {
            r"Indica o cartafol dos rexistros completo, coa súa unidade, como D:\Notas.".to_owned()
        }
        AppError::NotFound(path) => format!("{} non existe.", path.display()),
        AppError::AlreadyExists(path) => format!("{} xa existe.", path.display()),
        AppError::SpreadsheetWrite { path, .. } => format!(
            "Non se puido gardar {}. Comproba que non está aberto en Excel e que podes gardar alí.",
            path.display()
        ),
        AppError::SpreadsheetRead { path, .. } => format!(
            "{} non é un libro que se poida ler. Comproba que é un ficheiro .xlsx de Excel e que non está danado.",
            path.display()
        ),
        AppError::PictureRead { path, .. } => format!(
            "{} non é unha imaxe que se poida ler. Escolle un ficheiro JPEG, PNG ou WebP que non estea danado.",
            path.display()
        ),
        AppError::NoPupilsInFile(path) => format!(
            "{} non ten alumnos: a súa primeira fila precisa unha cabeceira Nome (ou os nomes na súa primeira columna), cun alumno en cada fila de debaixo.",
            path.display()
        ),
        AppError::DuplicatePupilInFile(name) => {
            format!("O ficheiro ten «{name}» dúas veces; cada alumno só pode estar nel unha vez.")
        }
        _ => unreachable!("error() sends only file errors to files()"),
    }
}

/// What went wrong with a file, in words the user can act on.
fn io_problem(path: &std::path::Path, source: &std::io::Error) -> String {
    let path = path.display();
    match IoProblem::of(source) {
        IoProblem::NotFound => format!("Non se atopou {path}."),
        IoProblem::Denied => {
            format!("avalio lite non ten permiso para usar {path}. Escolle outro sitio ou revisa os seus permisos.")
        }
        IoProblem::InUse => format!("Outro programa está a usar {path}. Péchao e proba de novo."),
        IoProblem::Full => {
            format!("Non hai espazo libre abondo para {path}. Libera espazo ou escolle outro sitio.")
        }
        IoProblem::Other => format!(
            "Non se puido acceder a {path}. Comproba que está dispoñible e proba de novo; o rexistro ten os detalles."
        ),
    }
}

/// The errors about a value typed into a field: a name, a number, a rounding source or a PIN.
fn fields(error: &AppError) -> String {
    match error {
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
        _ => unreachable!("error() sends only field errors to fields()"),
    }
}

/// The errors about keyboard shortcuts: a bad entry in the settings file, or a key that can't be used.
fn shortcuts(error: &AppError) -> String {
    match error {
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
        _ => unreachable!("error() sends only shortcut errors to shortcuts()"),
    }
}
