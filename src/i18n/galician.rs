// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/03 11:13:31.011467
//+ Revised: 	2026/11/08 04:36:59.586820
// -----------------------------------------------------------------------------

//! The Rust-side texts and error messages in Galician.

use super::Language;
use super::errors::{DatabaseProblem, IoProblem, entries};
use super::messages::{Message, counted, plural, renamings};
use super::shortcuts::{action_label, display};
use crate::error::{AppError, Folder, NumberField};
use crate::shortcuts::ShortcutError;

/// A message in Galician.
pub(super) fn message(message: &Message) -> String {
    match message {
        Message::ConfirmDeleteClass { class } => {
            format!(
                "Eliminar a clase «{class}» con todas as súas columnas e notas?\n\nOs seus alumnos seguen nas outras clases."
            )
        }
        Message::ConfirmRemovePupil { pupil, class } => {
            format!("Quitar a {pupil} de {class}?\n\nBórranse as súas notas nesta clase. Segue nas outras clases.")
        }
        Message::ConfirmDeleteColumn { column } => {
            format!("Eliminar a columna «{column}» e todo o que se escribiu nela?")
        }
        Message::ConfirmDeletePupil { pupil } => {
            format!("Eliminar a {pupil}? Non está en ningunha clase; non se pode desfacer.")
        }
        Message::DatabaseMoved { path } => format!("Base de datos movida a {path}"),
        Message::LogsMoved { path } => format!("Rexistros movidos a {path}"),
        Message::DatabaseFileType => "Base de datos de avalio-lite".to_owned(),
        Message::BackupFileType => "Copia de seguranza de avalio-lite".to_owned(),
        Message::BackupExported { path } => format!("Copia gardada en {path}"),
        Message::ConfirmRestore { file, classes, pupils } => format!(
            "Substituír todo o que hai na aplicación pola copia {file} ({}, {})?\n\nSubstitúense todas as clases, alumnos e notas que hai agora. Non se pode desfacer. Exporta antes unha copia se as podes necesitar.",
            counted(*classes, "clase", "clases"),
            counted(*pupils, "alumno", "alumnos"),
        ),
        Message::BackupRestored { file } => format!("Restaurouse a copia {file}."),
        Message::BackupClassesAdded { classes, pupils, existing, file } => {
            let already = match existing {
                0 => String::new(),
                1 => " (1 xa estaba)".to_owned(),
                _ => format!(" ({existing} xa estaban)"),
            };
            let verb = plural(*classes, "Engadiuse", "Engadíronse");
            format!(
                "{verb} {} e {}{already} desde {file}.",
                counted(*classes, "clase", "clases"),
                counted(*pupils, "alumno", "alumnos")
            )
        }
        Message::BackupClassesRenamed { pairs } => {
            format!("Renomeadas porque o nome xa existía: {}.", renamings(pairs))
        }
        Message::BackupHasNoClasses => "A copia non ten clases; non se engadiu nada.".to_owned(),
        Message::WindowFailed => {
            "Non se puido iniciar avalio-lite. Péchao e ábreo de novo; se segue pasando, o rexistro ten os detalles."
                .to_owned()
        }
        Message::LockNeedsPin => "Define un PIN en Axustes para bloquear a aplicación.".to_owned(),
        Message::PinTurnedOn => "A base de datos xa está cifrada co teu PIN.".to_owned(),
        Message::PinChanged => "PIN cambiado; a base de datos cifrouse de novo con el.".to_owned(),
        Message::PinTurnedOff => "PIN eliminado; a base de datos xa non está cifrada.".to_owned(),
        Message::ShortcutChanged { action, keys } => format!("{action} agora é {keys}."),
        Message::ShortcutsReset => "Todos os atallos de teclado volven ser os de orixe.".to_owned(),
        Message::SheetExported { class, path } => format!("Gardouse {class} en {path}"),
        Message::SpreadsheetFileType => "Libro de Excel".to_owned(),
        Message::ExportWeightsWarning { total } => {
            format!("Os pesos das probas suman {total} %, non 100 %: as notas finais son provisionais.")
        }
        Message::ConfirmImportOverwrite { count, class } => {
            let (verb, what) = if *count == 1 {
                ("substitúese", "nota ou observación")
            } else {
                ("substitúense", "notas ou observacións")
            };
            format!("Ao importar {verb} {count} {what} que xa hai en {class}. Continuar?")
        }
        Message::ImportDone { pupils, new_pupils, marks, class } => {
            let added = if *new_pupils > 0 {
                format!(" ({new_pupils} {})", plural(*new_pupils, "novo", "novos"))
            } else {
                String::new()
            };
            let verb = plural(*pupils, "Importouse", "Importáronse");
            format!(
                "{verb} {pupils} {}{added} e {marks} {} en {class}.",
                plural(*pupils, "alumno", "alumnos"),
                plural(*marks, "nota ou observación", "notas ou observacións")
            )
        }
        Message::ImportLeftOut { headers, count } => {
            format!("{} fóra: {headers}.", plural(*count, "Quedou", "Quedaron"))
        }
        Message::ImportErrorCells { count } => {
            if *count == 1 {
                "Saltouse 1 cela cun erro de Excel.".to_owned()
            } else {
                format!("Saltáronse {count} celas con erros de Excel.")
            }
        }
        Message::UntitledColumn { letter } => format!("columna {letter} (sen título)"),
        Message::LicenceAskForKey => {
            "avalio-lite necesita unha clave de licenza para funcionar. Recibíchela por correo; ten esta forma:
BRN-XXXX-XXXX-XXXX-XXXX"
                .to_owned()
        }
        Message::LicenceInvalidKey => "Esa clave non é válida para avalio-lite. Revísaa e téntao de novo.".to_owned(),
        Message::LicenceInUseElsewhere { email } => {
            format!("Esta clave xa se está a usar noutro ordenador. Para pasala a este, escribe a {email}.")
        }
        Message::LicenceRevoked { email } => {
            format!("Esta licenza foi cancelada. Escribe a {email} se cres que é un erro.")
        }
        Message::LicenceExpired { date: Some(date), email } => {
            format!("A túa licenza rematou o {date}. Escribe a {email} para renovala.")
        }
        Message::LicenceExpired { date: None, email } => {
            format!("A túa licenza rematou. Escribe a {email} para renovala.")
        }
        Message::LicenceWithdrawn { email } => {
            format!("Agora mesmo non se pode licenciar avalio-lite. Escribe a {email}.")
        }
        Message::LicenceNeedsInternet => "Conéctate a internet para que avalio-lite poida confirmar a túa licenza. Funciona sen conexión ata 30 días seguidos.".to_owned(),
        Message::LicenceWait { minutes } => format!("Demasiados intentos. Téntao de novo en {minutes} min."),
        Message::LicenceUnreachable => {
            "Non se pode contactar co servidor de licenzas. Comproba a conexión a internet e téntao de novo.".to_owned()
        }
        Message::LicenceNotSaved { error } => format!("Non se puido gardar a licenza neste ordenador: {error}"),
        Message::BackgroundFailed { error } => format!("Non se puido iniciar a comprobación: {error}"),
        Message::LicenceConnectBy { date } => {
            format!("Conéctate a internet antes do {date} para seguir usando avalio-lite.")
        }
        Message::LicenceEndsOn { date, email } => {
            format!("A túa licenza remata o {date}. Escribe a {email} para renovala.")
        }
        Message::LicencePermanent => "Permanente".to_owned(),
        Message::LicenceUntil { date } => format!("Ata o {date}"),
        Message::LicenceOfflineUntil { date } => format!("Funciona sen conexión ata o {date}"),
        Message::LicenceConfirmed => "Licenza confirmada.".to_owned(),
        Message::UpdateAvailable { version } => format!("Xa está dispoñible avalio-lite {version}."),
        Message::UpToDate { version } => format!("avalio-lite está ao día ({version})."),
        Message::UpdateCheckFailed { reason } => format!("Non se puideron buscar actualizacións: {reason}"),
        Message::LinkFailed { error } => format!("Non se puido abrir a páxina: {error}"),
        Message::UpdateNotVerified => {
            "Non se puido verificar a actualización, así que non se instalou.".to_owned()
        }
        Message::UpdateDownloadFailed { reason } => format!("Non se puido descargar a actualización: {reason}"),
        Message::UpdateNotPlaced { reason } => format!("Non se puido colocar a actualización: {reason}"),
        Message::UpdateFolderReadOnly => {
            "Non se pode escribir neste cartafol. Descarga a nova versión desde a páxina da versión.".to_owned()
        }
        Message::UpdateFileMissing { file } => {
            format!("A versión non ten {file}. Descarga a nova versión desde a páxina da versión.")
        }
        Message::UpdatedTo { version } => format!("Actualizado a avalio-lite {version}."),
        Message::UpdateNotStarted { reason } => {
            format!("Non se puido iniciar a actualización: {reason}. Descárgaa desde a páxina da versión.")
        }
    }
}

/// An error in Galician, as `error_message` describes.
pub(super) fn error(error: &AppError) -> String {
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
