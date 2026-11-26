[//]: # ( ---------------------------------------------------------------------- )
[//]: # (+ Authors: 	Ran# <ran.hash@proton.me> )
[//]: # (+ Created: 	2026/11/01 21:11:24.661079 )
[//]: # (+ Revised: 	2026/11/25 11:43:01.059580 )
[//]: # ( ---------------------------------------------------------------------- )

# avalio lite user guide

[README](../README.md) · **User guide** · [Development guide](development.md) · [Design decisions](design-decisions.md)

Everything the app does, screen by screen. To download and install it, see the [README](../README.md#download-and-install). There are two downloads, both for Windows 64-bit: an installer (the setup) and a portable file that runs from any folder.

**Contents:** [Getting started](#getting-started) · [Using it](#using-it) · [Your data](#your-data) · [Licence](#licence) · [Updates](#updates) · [Troubleshooting](#troubleshooting) · [Help](#help)

## Getting started

1. **Choose your folders.** On first launch the app asks where to keep the database and the logs (both default to `Documents\avalio-lite`; see [Your data](#your-data)). The window opens centred on the screen you started it from, sized to fit; on small screens it opens maximised.
2. **Enter your licence key.** Type the `BRN-…` key you received by email and press **Activate** (see [Licence](#licence)).
3. **Add a class.** Type a name in **+ New class** in the sidebar and press Enter.
4. **Add pupils.** In the last row of the sheet, type a first name, press Enter, type the surname and press Enter again.
5. **Add columns.** Use **+ Add column**, for example two tests and a final grade, then click a header to set its weight and maximum.
6. **Type marks.** The final grade and the pass/fail count under the class name update as you type.
7. *(Optional)* **Set a PIN** in **Settings → PIN and encryption** to encrypt the file.

## Using it

### Classes and pupils

- **Classes** are listed on the left. Type a name in **+ New class** and press Enter.
- **Pupils** are the rows of a class's sheet. Each has a **first name** and a **surname**, in two columns of their own. To add one, type the first name in the last row (or click **+ Add pupil**), press Enter to go to the surname, and press Enter again: the pupil is added and the first-name field is ready for the next one. The surname may be left empty; the first name can't. Pupils from before surnames existed that were typed surname first, as in "Pérez López, Ana", are split into the two columns when you update (also in older backups you restore or add from); the others keep their whole name as the first name, to fix by hand. A pupil can be in several classes (**⋯ → Add from another class**, which lists pupils as "Surname, First name", sorted by surname). Renaming a pupil renames them everywhere. Removing them from one class (the × at the start of their row) deletes only their marks in that class.
- Under the class name you see the number of pupils, the class average of the final grade, and how many pass and fail.
- Classes, and lists of pupils to pick from, are sorted alphabetically, ignoring accents and case (Álvaro goes with the A's; Ñ comes after N). The rows of a sheet stay in the order the pupils were added. A pupil's **number** is their place in the class by surname, so it changes when someone before them joins or leaves.

### Columns

Add columns with **+ Add column**. Click a column's header to open its settings on the right: title, weight, maximum, bonus, rounding, move left / right, and delete. Esc closes the panel. ‹ and › at the top of the panel (or Alt+← and Alt+→) open the settings of the column to the left or right, and Ctrl+← and Ctrl+→ move the column while its panel is open. The header shows the title and small tags such as `50%` and `/10`. **⋯ → Hide weights & scales** hides those tags, and the app remembers the choice.

Drag a column's header to move it: a line shows where it will land, and it moves when you let go (a click without dragging still opens its settings). The panel's Move buttons and Ctrl+← / Ctrl+→ still move it one place. Dragging near the edge of the sheet scrolls it, so a column can go to a place off screen.

The **First name** and **Surname** columns move and resize like any other, but can't be deleted or renamed: their panel shows only the Move buttons, and their titles follow the interface language.

Drag the right edge of a column's header to change its width (double-click the edge to reset it, or Shift+← / Shift+→ on a focused header); the name columns too. Each class keeps its own column order and widths, also in a duplicate or a backup.

**⋯ → Duplicate class** copies a class's columns and settings; **Duplicate with pupils** brings the same pupils with empty marks (for a new term), and **Duplicate with pupils and marks** makes an exact copy. The copy is called e.g. *1A (2)*; rename it in place.

| Column | Holds |
|---|---|
| Test | A score you type, out of the test's maximum (e.g. 10 or 5), counting a set % towards the final grade. Tick **Bonus** for extra credit: its % counts on top of the normal tests' 100 % |
| Info | Any text, e.g. notes |
| Final grade | The sum of each test's weight × score / maximum (bonus tests included), shown out of its own maximum and never above it. Ungraded tests count as 0. It stays blank until a test has a mark. While the normal tests' weights don't add up to 100 %, a warning shows under the class name and final grades turn grey with a "!" |
| Average | Mean of the graded normal tests (each as score / maximum), shown out of its own maximum; bonus tests are left out |
| Rounded | Another numeric column rounded to 0–3 decimals (half up: 7.45 → 7.5) |

A test's weight can be anything from 0 to 1000 %, and its maximum anything above 0 up to 10 000.

### Marks

Everything saves as you type; there is no Save button. The line under the class name says so: **Saves as you type** at first, then **✓ Saved**, which lights up green for a moment after each change that was written. If a change couldn't be saved, it says **Not saved** in red until the next change that is, and the message above the sheet says why. Pressing Ctrl+S, by habit, answers *Everything is saved*.

- Scores can be typed as `7.5` or `7,5`: digits with one decimal point or comma (`1e1` and `+5` aren't accepted). A score that isn't a number in the test's range is shaded red and left out of the calculations.
- Press Enter to go to the same column in the next pupil's row, with its mark selected so typing replaces it; after the last pupil Enter leaves the sheet.
- Calculated marks are shown to two decimals, rounded half up. They show as green pills when the shown mark is half the maximum or more (5 out of 10), and as rose pills below that. Failing test scores are shown in rose too.

### Pupils' pictures

The round button at the start of each row shows the pupil's picture, or their initials. Click it to open the pupil's panel on the right: **Choose a picture…** takes a JPEG, PNG or WebP photo (up to 50 MB), **Change the picture…** replaces it and **Remove the picture** deletes it, after asking. Esc closes the panel. A pupil has one picture, the same in all their classes.

The app keeps the middle square of the photo, made small, inside the database: your photo file isn't changed, and what it says about where and with what it was taken isn't kept. Pictures are part of your [data](#your-data): encrypted if you set a PIN, included in backups, and deleted with the pupil. When you add a backup's classes, a pupil who already has a picture here keeps it.

### Showing a pupil their grades

**⋯ → Show a pupil their grades…** covers the whole window, so you can let a pupil come to the computer and see their own grades and nobody else's.

1. A list of the class's pupils opens, with each one's picture, number and name and no grades. Numbers go by surname, from 1. Pick the pupil who is coming; they can come in any order.
2. The window shows only that pupil's picture, name, number, class, and their own tests, final grades, averages and roundings, each out of its maximum. Weights show when the sheet shows them. **Info columns are never shown**, since they hold your notes, and neither is a score that isn't a valid mark. The sidebar, the sheet and the update bar are gone, and every keyboard shortcut but Lock is off.
3. **Next pupil** goes on to the next number, and **Back to the list** clears the grades so you can pick anyone.
4. **Back to the sheet**, in the list, leaves the mode.

With a [PIN](#pin-and-encryption), it is asked **before each pupil's grades** (picked or next) and **before going back to the sheet**. It isn't asked to open the list or to go back to it, so at most you type it once per pupil. The page that asks names the pupil it is for, and **Back to the list** gives up. Wrong PINs count together with those on the lock screen.

Without a PIN nothing is asked, so a pupil could open someone else's grades (the list says so). Set one in **Settings → PIN** to prevent it. Locking ends the mode, so unlocking shows the sheet. Closing and reopening the app does too: with a PIN it opens on the lock screen, without one on the sheet. The mode doesn't start while an update is downloading, because the update restarts the app.

### Excel

- **⋯ → Export to Excel…** saves the sheet as an .xlsx file in the sheet's column order, First name and Surname included, with the marks as numbers and a second tab listing each column's settings. When the two name columns come first, they stay in view as you scroll sideways.
- **⋯ → Import from Excel…** adds the pupils of a workbook (.xlsx, .xls or .ods) and fills in the marks under columns with the same titles. It finds the names by their headers anywhere in the first row: **First name** / **Nome** and **Surname** / **Apelidos**. With neither header, the first column holds the whole name (and with only a surname header, the first column is the first name, or the second when the surname is in the first); a name written surname first there, as in "Pérez López, Ana", is split at its comma. The headers are the first row that has anything in it, so a blank row or column before the table doesn't matter. Pupils are matched by their full name, so a file with one name column still finds Ana Pérez. Rows with marks or a surname but no first name are skipped, and the report says how many. It asks before replacing marks that are already there.

To back up all classes at once, use a [backup](#backups) instead.

### Appearance and language

- **Light and dark:** use the moon / sun button at the bottom of the sidebar, or **Settings → Appearance**. The app follows the Windows setting until you pick one.
- **Narrow sidebar:** the arrow button at the top of the sidebar (or Ctrl+B) folds it to a strip of icons, giving the sheet more room; the same button opens it again. The app remembers which you chose.
- **Window:** the app opens where it was when you last closed it, at the same size, and maximised if it was. If that place is no longer on any screen (a second monitor unplugged), it opens in the middle of the screen.
- **Language:** the app comes in **Galician** and **English**. It follows the Windows language until you pick one in **Settings → Language**. In Galician, calculated marks use a decimal comma (8,5).

### Keyboard

Tab moves between fields and buttons, and Enter or Space presses the focused button. Enter in a mark or note goes to the cell below; Enter in a pupil's name, the class name or a column setting ends the editing. Shift+← / Shift+→ on a focused column header narrows or widens the column.

Shortcuts work anywhere on the main screen, even while you're typing in a cell, but not on the lock or first-run screens, and only Lock works while a pupil is shown their grades. Settings lists them:

| Shortcut | Does | Name in `config.toml` |
|---|---|---|
| Ctrl+L | Lock | `lock` |
| Ctrl+Shift+D | Switch between light and dark | `toggle-theme` |
| Ctrl+H | Show or hide weights and scales | `toggle-column-settings` |
| Ctrl+B | Collapse or expand the sidebar | `toggle-sidebar` |
| Ctrl+, | Open Settings (again: back to the sheet) | `open-settings` |
| Ctrl+PageDown | Next class | `next-class` |
| Ctrl+PageUp | Previous class | `previous-class` |

In a column's panel, Ctrl+←/→ moves the column and Alt+←/→ opens the next column's settings; these are fixed. So is Ctrl+S, which only confirms that everything is saved, unless you give those keys to one of the shortcuts above.

To change a shortcut, press **Change** next to it in **Settings → Keyboard shortcuts**, then press the new keys (Esc cancels). The change is saved at once. **Reset** restores one shortcut and **Reset all** restores every one. Keys that another shortcut already uses are refused, not taken over. You can also close the app and add the shortcut to `config.toml` under `[shortcuts]`, e.g. `lock = "Ctrl+K"`.

A shortcut is Ctrl, Shift and/or Alt plus A–Z, 0–9, `,`, `.`, PageUp, PageDown, Home or End; F1–F12 also work on their own. These aren't allowed: plain letters (they type into the cell), Ctrl and Alt together (that is AltGr on many keyboards), and Ctrl+A, C, V, X, Y and Z, which edit text. If a `config.toml` entry isn't valid, or two actions share the same keys, the app says so when it opens and uses the default.

## Your data

### Where it lives

On first launch the app asks where to keep:

- **the database:** `avalio-lite.db`, everything you enter
- **the logs:** `avalio-lite.log`, for diagnosing problems. Once it passes 5 MB, the app sets it aside as `avalio-lite.log.1` when it starts, replacing any older one

Both default to `Documents\avalio-lite`, and a folder must be given in full, with its drive. Either can be moved later from **Settings**: the app copies the file, checks the copy, switches to it, and only then deletes the original (if another program keeps the original from being deleted, the app says where it is, to delete by hand).

The first start after an update that changes how the database is stored keeps a copy of it as it was, `avalio-lite.db.before-update`, next to it (with the same PIN, if it has one); the next such update replaces it, and setting or changing the PIN deletes it. Copies a crash could leave while a PIN was being changed or a backup restored (`.rekey`, `.previous`, `.restore`) are deleted the next time the database opens. **Open another database…** switches to an existing database file (one from another PC, say). To bring a backup's data into the app, use [Backups](#backups) instead.

The only file in a fixed place is `%APPDATA%\Breren\avalio-lite\config.toml`, which holds those two locations (0.10.0 and earlier kept it in `%APPDATA%\avalio-lite`; the first start of a newer version moves it). If that file can't be read, the app keeps it as `config.toml.bad`, asks for the locations again, and writes a new one. If a database file is empty, cut short or not an avalio-lite database, the app reports it and never sets up a new database over it. Uninstalling never deletes your database or logs.

### Backups

**Settings → Backup → Export everything…** saves the whole database to one `.avalio` file while the app stays open. The file is encrypted with your PIN if you have set one.

- **Restore from a backup…** replaces everything in the app with the backup. It asks first, and keeps your current PIN.
- **Add classes from a backup…** copies a backup's classes, pupils and marks into the app without changing what's already there. A backup pupil joins the pupil here with the same full name (first name and surname) when there is exactly one; with none, or several, a new pupil is made, so two people are never merged by guess.

A backup is an `.avalio` file or a copy of an `avalio-lite.db` made while the app was closed. The database the app is using right now isn't one, and choosing it is refused: its latest changes may not be in the file yet.

### PIN and encryption

In **Settings → PIN and encryption** you can set a PIN of 4 to 128 characters: digits, letters, spaces or symbols. The database file is then encrypted with it using [SQLCipher](https://www.zetetic.net/sqlcipher/): every page of the file is encrypted with AES-256 and checked for tampering, so the file can't be read without the PIN.

- **Unlocking:** the app asks for the PIN when it opens. Type it, or click it on the on-screen keys: **ABC** switches them to letters (⇧ for capitals, plus a space bar), **#+=** switches to two pages of symbols (**1/2** flips between them), and **123** goes back to numbers.
- **Locking:** **Ctrl+L** (or your own shortcut) or **Lock** in the sidebar closes the database and shows the lock screen.
- **Wrong PINs:** after five wrong PINs, each further try waits 30 seconds.
- **Removing the PIN** (it asks for the current one) turns the encryption off again.

**How strong a PIN to use is up to you.** The 30-second wait only applies inside the app. Someone with a copy of the file can try guesses as fast as their computer allows; each guess is deliberately slow, but not slow enough to protect short PINs. A 4-digit PIN can be found in minutes, and 8 digits in about a day on a gaming graphics card. A password of 12 or more characters with letters, or four random words, would take far longer than anyone can wait.

While the app is open, the PIN is kept in memory only while the database is unlocked. It is wiped (overwritten with zeros) when the database locks or closes, or when the PIN changes, and SQLCipher wipes its own copies too. The PIN is never written to the settings file or the log.

> [!WARNING]
> **If you forget the PIN, the data can't be recovered, by anyone.** Keep the PIN somewhere safe.

**Forgot the PIN?** on the lock screen takes you to the first-run screen, where you can open another database or start a new one in a different folder. The locked file stays where it is, untouched, and **Back to the lock screen** returns to it (a 30-second wait that has already started keeps running).

## Licence

avalio-lite needs a licence key, which looks like `BRN-XXXX-XXXX-XXXX-XXXX` and comes by email. The app checks it with Clavren, Breren's licence server, at `clavren.breren.com`. Only the key and a code for this computer are sent; your marks never leave your computer.

- **Activating:** after choosing your folders (and typing your PIN, if the database has one), the app shows **Enter your licence key**. Type the key (capitals, spaces and dashes don't matter) and press **Activate** or Enter. This needs the internet.
- **Offline:** once activated, the app opens straight away, with or without internet, for up to 30 days after its last check.
- **Daily check:** while the computer is online, the app confirms the licence in the background once a day, at start or while it runs, without slowing anything down. If the licence was cancelled or has ended, the licence screen takes the sheet's place (everything you typed is already saved); if it was extended, it goes away again by itself.
- **Warnings on the sheet:** five days before the 30 days offline run out, the sheet says *Connect to the internet by …*; two weeks before a licence for a set period ends, it says *Your licence ends on …* and who to write to.
- **Settings → Licence** shows whether the licence is permanent or until when it lasts, and until when the app works offline. **Check now** confirms it straight away.
- **One computer at a time:** a key belongs to one Windows account on one computer. The licence is kept, protected by Windows, in `%APPDATA%\Breren\avalio-lite\licence.dat`; copied to another computer, it doesn't work there. Uninstalling keeps it, so a reinstall needs no new activation.
- **While the app is locked**, **Back up my data** saves everything to a `.avalio` backup, as **Settings → Backup** does, so your marks are never out of reach. **Ctrl+L** still locks the database.

What the licence screen can say:

| Message | What it means | What to do |
|---|---|---|
| That key isn't valid for avalio-lite | The key is mistyped, unknown, or for another product | Check it against the email |
| This key is already in use on another computer | The key is active on another computer or Windows account | Write to support.avalio.lite@breren.com to move it |
| This licence has been cancelled | The licence was revoked | Write to support.avalio.lite@breren.com |
| Your licence ended on … | A licence for a set period has run out | Write to support.avalio.lite@breren.com to renew it, then press **Check again** |
| avalio-lite can't be licensed at the moment | Licensing for avalio-lite is paused | Write to support.avalio.lite@breren.com |
| Connect to the internet so avalio-lite can confirm your licence | 30 days without a check, or the computer's clock was set back | Connect and press **Check again** |
| Too many attempts | The server asks to wait | Try again after the minutes shown |
| The licence server can't be reached | No internet, or the server is down | Check the connection and try again |
| The licence couldn't be saved on this computer | The licence file couldn't be written | Check that there is free space and try again; the log has the details |

## Updates

- **Looking for a new version:** about a minute after the app opens, and then once a day, it asks GitHub and GitLab for the latest release. When a newer version is out, a bar across the top of the window says *avalio-lite X.Y.Z is available.*
- **Update and restart** downloads the new version, checks that it is the developer's (each release is signed in minisign's format; the app checks the signature before it downloads anything, then the file's checksum and the signature over the file before it uses it), puts it in place of the running program and opens it again. Everything you typed is already saved. The next start says *Updated to avalio-lite X.Y.Z.* While it downloads, the bar shows how far it has come, and **Cancel** stops it.
- **Skip this version** hides the bar until a newer version is out. **Hide** (×) hides it until you open the app again.
- **Settings → Updates** shows the version you are running and what the last check found (*avalio-lite is up to date (checked 14:02).*). **Check for updates automatically** turns the daily check on or off (it starts on). **Check now** looks straight away. **Stop skipping X.Y.Z** offers a version you skipped again. **Update and restart** is there too.
- **What it sends:** only the request for the latest release, with the app's name and version. Nothing about you, this computer or your marks. Turning off the automatic check stops it.
- If the check or the download fails, the app says why in plain words (for example *Couldn't check for updates: GitHub: the request timed out; GitLab: the server answered 503.*) and stays as it was; a download that failed or didn't verify is deleted, never run. Try again later, or download the new version by hand from the release page the message names. A check that runs by itself and fails only changes the line in Settings; it tries again an hour later.
- Installed and portable copies update the same way. A portable copy needs a folder it can write to; on a read-only USB stick, download the new version by hand.
- A copy you built yourself from the source code doesn't update itself: Settings says so, and you update it with `git pull` and a new build.

## About

**Settings → About**, at the bottom of Settings, shows the version you are running, who makes avalio lite (Ran#, published by [Breren](https://breren.com)) and its licence, the PayBack License 2.0: free for people, not for profit. Its buttons open the licence, the licences of the open-source code the app is built with, the source code, all versions, what's new, and where to report a problem or write to support (support.avalio.lite@breren.com). If Windows can't open a link, the app shows the address to copy into a browser.

## Troubleshooting

| What you see | What to do |
|---|---|
| The lock screen asks for a PIN you can't remember | Your marks can't be recovered, by anyone. Press **Forgot the PIN?** to open or create another database; the locked file stays exactly as it is. |
| *The database isn't at … any more.* (at start-up) | The app never recreates a missing database by itself. Press **Open another database…** to point it at the file where it is now, or start a new one yourself. |
| *Too many wrong PINs. Try again in 30 s.* | Wait the time shown, then type the PIN again. After five wrong PINs, each further try waits 30 seconds. |
| *This key is already in use on another computer.* | A key works on one computer at a time. Write to support.avalio.lite@breren.com to move it here. |
| *Connect to the internet so avalio lite can confirm your licence.* | The app has gone 30 days without a check, or the computer's clock was set back. Connect to the internet and press **Check again**. |
| Windows shows *"Windows protected your PC"* | The files aren't code-signed for Windows. Click **More info**, then **Run anyway**; or right-click the file, choose **Properties** and tick **Unblock**. |
| *The download of avalio lite … didn't match its checksum* or *isn't signed with avalio lite's release key* | The update was refused and nothing was installed. Download the new version by hand from the release page the message names. |
| *Left out: …* or *… rows with no first name were skipped.* (Excel import) | Give each pupil a first name in the file and import again, or add them by hand. Cells with Excel errors are skipped too; type those marks again. |
| *This backup is encrypted. Type the PIN it was made with.* | A backup of this database opens with the app's PIN. A backup made with another PIN asks for that one; if you no longer have it, the backup can't be opened. |
| The message ends with *the log has the details* | Open **Settings → Logs** to see the log's folder, then send `avalio-lite.log` with your report. It holds file paths, so look it over first. |

## Help

- **Bugs and ideas:** open an issue on [GitHub](https://github.com/Ran-n/avalio-lite/issues), [GitLab](https://gitlab.com/Ran-n/avalio-lite/-/issues) or [Codeberg](https://codeberg.org/ran-n/avalio-lite/issues). [CONTRIBUTING.md](../CONTRIBUTING.md#reporting-bugs) says what to include.
- **Your licence, or anything the issues don't cover:** write to support.avalio.lite@breren.com.
