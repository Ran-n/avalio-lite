// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 00:26:06.881387
//+ Revised: 	2026/11/23 16:34:35.750781
// -----------------------------------------------------------------------------

//! The data topic of `Message`. Its texts are in `english/data.rs` and `galician/data.rs`.

/// The texts about the database, the logs, backups, the PIN and the lock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataMessage<'a> {
    DatabaseMoved {
        path: &'a str,
    },
    /// the database was moved to `path`, but its original at `old` couldn't be deleted
    DatabaseMovedOldKept {
        path: &'a str,
        old: &'a str,
    },
    LogsMoved {
        path: &'a str,
    },
    DatabaseFileType,
    BackupFileType,
    BackupExported {
        path: &'a str,
    },
    ConfirmRestore {
        file: &'a str,
        classes: usize,
        pupils: usize,
    },
    BackupRestored {
        file: &'a str,
    },
    BackupClassesAdded {
        classes: usize,
        pupils: usize,
        existing: usize,
        file: &'a str,
    },
    /// (name in the backup, name given here) of each class whose name was taken
    BackupClassesRenamed {
        pairs: &'a [(String, String)],
    },
    BackupHasNoClasses,
    LockNeedsPin,
    PinTurnedOn,
    PinChanged,
    PinTurnedOff,
}
