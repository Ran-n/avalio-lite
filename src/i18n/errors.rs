// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/28 09:31:17.615582
//+ Revised: 	2026/10/28 22:23:16.942256
// -----------------------------------------------------------------------------

use super::Language;
use crate::error::{AppError, Folder, NumberField};

/// The error as shown to the user, in their language. Paths and the system's own
/// error details are passed through as they are.
pub fn error_message(language: Language, error: &AppError) -> String {
    match language {
        Language::English => english(error),
        Language::Galician => galician(error),
    }
}

fn english(error: &AppError) -> String {
    match error {
        AppError::Io { path, source } => format!("Couldn't access {}: {source}", path.display()),
        AppError::Database(source) => format!("Database error: {source}"),
        AppError::ConfigParse { path, source } => {
            format!("Couldn't read the settings file {}: {source}", path.display())
        }
        AppError::ConfigSerialize(source) => format!("Couldn't save the settings file: {source}"),
        AppError::NoConfigDir => "Couldn't find your settings folder.".to_owned(),
        AppError::EmptyPath(Folder::Database) => "Choose a folder for the database.".to_owned(),
        AppError::EmptyPath(Folder::Logs) => "Choose a folder for the logs.".to_owned(),
        AppError::NotFound(path) => format!("{} doesn't exist.", path.display()),
        AppError::AlreadyExists(path) => format!("{} already exists.", path.display()),
        AppError::NewerSchema { found, supported } => format!(
            "This database was made by a newer version of avalio-lite (schema {found}; this version reads up to {supported}). Update the app to open it."
        ),
        AppError::IntegrityCheckFailed(path) => format!("The database at {} is damaged.", path.display()),
        AppError::NoDatabaseOpen => "No database is open.".to_owned(),
        AppError::EmptyName => "A name can't be empty.".to_owned(),
        AppError::DuplicateClassName(name) => format!("There's already a class called \"{name}\"."),
        AppError::InvalidNumber { field: NumberField::Weight, value } => {
            format!("\"{value}\" isn't a valid weight. Use a number of 0 or more.")
        }
        AppError::InvalidNumber { field: NumberField::MaxScore, value } => {
            format!("\"{value}\" isn't a valid maximum. Use a number above 0.")
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
    }
}

fn galician(error: &AppError) -> String {
    match error {
        AppError::Io { path, source } => format!("Non se puido acceder a {}: {source}", path.display()),
        AppError::Database(source) => format!("Erro da base de datos: {source}"),
        AppError::ConfigParse { path, source } => {
            format!("Non se puido ler o ficheiro de axustes {}: {source}", path.display())
        }
        AppError::ConfigSerialize(source) => format!("Non se puido gardar o ficheiro de axustes: {source}"),
        AppError::NoConfigDir => "Non se atopou o teu cartafol de axustes.".to_owned(),
        AppError::EmptyPath(Folder::Database) => "Escolle un cartafol para a base de datos.".to_owned(),
        AppError::EmptyPath(Folder::Logs) => "Escolle un cartafol para os rexistros.".to_owned(),
        AppError::NotFound(path) => format!("{} non existe.", path.display()),
        AppError::AlreadyExists(path) => format!("{} xa existe.", path.display()),
        AppError::NewerSchema { found, supported } => format!(
            "Esta base de datos creouse cunha versión máis nova de avalio-lite (esquema {found}; esta versión le ata o {supported}). Actualiza a aplicación para abrila."
        ),
        AppError::IntegrityCheckFailed(path) => format!("A base de datos de {} está danada.", path.display()),
        AppError::NoDatabaseOpen => "Non hai ningunha base de datos aberta.".to_owned(),
        AppError::EmptyName => "O nome non pode quedar baleiro.".to_owned(),
        AppError::DuplicateClassName(name) => format!("Xa hai unha clase chamada «{name}»."),
        AppError::InvalidNumber { field: NumberField::Weight, value } => {
            format!("«{value}» non é un peso válido. Usa un número igual ou maior ca 0.")
        }
        AppError::InvalidNumber { field: NumberField::MaxScore, value } => {
            format!("«{value}» non é un máximo válido. Usa un número maior ca 0.")
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
    }
}
