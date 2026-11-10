[//]: # ( ---------------------------------------------------------------------- )
[//]: # (+ Authors: 	Ran# <ran.hash@proton.me> )
[//]: # (+ Created: 	2026/11/01 21:11:24.661079 )
[//]: # (+ Revised: 	2026/11/09 21:53:59.337500 )
[//]: # ( ---------------------------------------------------------------------- )

# avalio-lite user guide

Everything the app does, screen by screen. To download and install it, see the [README](../README.md#download-and-install).

**Contents:** [Getting started](#getting-started) · [Using it](#using-it) · [Your data](#your-data) · [Licence](#licence)

## Getting started

1. **Choose your folders.** On first launch the app asks where to keep the database and the logs (both default to `Documents\avalio-lite`; see [Your data](#your-data)). The window opens centred on the screen you started it from, sized to fit; on small screens it opens maximised.
2. **Enter your licence key.** Type the `BRN-…` key you received by email and press **Activate** (see [Licence](#licence)).
3. **Add a class.** Type a name in **+ New class** in the sidebar and press Enter.
4. **Add pupils.** Type a name in the last row of the sheet and press Enter.
5. **Add columns.** Use **+ Add column**, for example two tests and a final grade, then click a header to set its weight and maximum.
6. **Type marks.** The final grade and the pass/fail count under the class name update as you type.
7. *(Optional)* **Set a PIN** in **Settings → PIN and encryption** to encrypt the file.

## Using it

### Classes and pupils

- **Classes** are listed on the left. Type a name in **+ New class** and press Enter.
- **Pupils** are the rows of a class's sheet. Type a name in the last row (or click **+ Add pupil**) and press Enter. A pupil can be in several classes (**⋯ → Add from another class**). Renaming a pupil renames them everywhere. Removing them from one class (× next to the name) deletes only their marks in that class.
- Under the class name you see the number of pupils, the class average of the final grade, and how many pass and fail.
- Classes and pupils are sorted alphabetically, ignoring accents and case (Álvaro goes with the A's; Ñ comes after N).

### Columns

Add columns with **+ Add column**. Click a column's header to open its settings on the right: title, weight, maximum, bonus, rounding, move left / right, and delete. Esc closes the panel. ‹ and › at the top of the panel (or Alt+← and Alt+→) open the settings of the column to the left or right, and Ctrl+← and Ctrl+→ move the column while its panel is open. The header shows the title and small tags such as `50%` and `/10`. **⋯ → Hide weights & scales** hides those tags, and the app remembers the choice.

Drag the right edge of a column's header to change its width (double-click the edge to reset it, or Shift+← / Shift+→ on a focused header); the **Pupil** column too. Each class keeps its widths, also in a duplicate or a backup.

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

Everything saves as you type.

- Scores can be typed as `7.5` or `7,5`: digits with one decimal point or comma (`1e1` and `+5` aren't accepted). A score that isn't a number in the test's range is shaded red and left out of the calculations.
- Press Enter to go to the same column in the next pupil's row, with its mark selected so typing replaces it; after the last pupil Enter leaves the sheet.
- Calculated marks are shown to two decimals, rounded half up. They show as green pills when the shown mark is half the maximum or more (5 out of 10), and as rose pills below that. Failing test scores are shown in rose too.

### Excel

- **⋯ → Export to Excel…** saves the sheet as an .xlsx file, with the marks as numbers and a second tab listing each column's settings.
- **⋯ → Import from Excel…** adds the pupils listed in column A of a workbook (.xlsx, .xls or .ods) and fills in the marks under columns with the same titles. It asks before replacing marks that are already there.

To back up all classes at once, use a [backup](#backups) instead.

### Appearance and language

- **Light and dark:** use the moon / sun button at the bottom of the sidebar, or **Settings → Appearance**. The app follows the Windows setting until you pick one.
- **Narrow sidebar:** the « button at the top of the sidebar (or Ctrl+B) folds it to a strip of icons, giving the sheet more room; » opens it again. The app remembers which you chose.
- **Language:** the app comes in **Galician** and **English**. It follows the Windows language until you pick one in **Settings → Language**. In Galician, calculated marks use a decimal comma (8,5).

### Keyboard

Tab moves between fields and buttons, and Enter or Space presses the focused button. Enter in a mark or note goes to the cell below; Enter in a pupil's name, the class name or a column setting ends the editing. Shift+← / Shift+→ on a focused column header narrows or widens the column.

Shortcuts work anywhere on the main screen, even while you're typing in a cell, but not on the lock or first-run screens. Settings lists them:

| Shortcut | Does | Name in `config.toml` |
|---|---|---|
| Ctrl+L | Lock | `lock` |
| Ctrl+Shift+D | Switch between light and dark | `toggle-theme` |
| Ctrl+H | Show or hide weights and scales | `toggle-column-settings` |
| Ctrl+B | Collapse or expand the sidebar | `toggle-sidebar` |
| Ctrl+, | Open Settings (again: back to the sheet) | `open-settings` |
| Ctrl+PageDown | Next class | `next-class` |
| Ctrl+PageUp | Previous class | `previous-class` |

In a column's panel, Ctrl+←/→ moves the column and Alt+←/→ opens the next column's settings; these are fixed.

To change a shortcut, press **Change** next to it in **Settings → Keyboard shortcuts**, then press the new keys (Esc cancels). The change is saved at once. **Reset** restores one shortcut and **Reset all** restores every one. Keys that another shortcut already uses are refused, not taken over. You can also close the app and add the shortcut to `config.toml` under `[shortcuts]`, e.g. `lock = "Ctrl+K"`.

A shortcut is Ctrl, Shift and/or Alt plus A–Z, 0–9, `,`, `.`, PageUp, PageDown, Home or End; F1–F12 also work on their own. These aren't allowed: plain letters (they type into the cell), Ctrl and Alt together (that is AltGr on many keyboards), and Ctrl+A, C, V, X, Y and Z, which edit text. If a `config.toml` entry isn't valid, or two actions share the same keys, the app says so when it opens and uses the default.

## Your data

### Where it lives

On first launch the app asks where to keep:

- **the database:** `avalio-lite.db`, everything you enter
- **the logs:** `avalio-lite.log`, for diagnosing problems. Once it passes 5 MB, the app sets it aside as `avalio-lite.log.1` when it starts, replacing any older one

Both default to `Documents\avalio-lite`. Either can be moved later from **Settings**: the app copies the file, checks the copy, switches to it, and only then deletes the original. **Open another database…** switches to an existing database file (one from another PC, say). To bring a backup's data into the app, use [Backups](#backups) instead.

The only file in a fixed place is `%APPDATA%\avalio-lite\config.toml`, which holds those two locations. If that file can't be read, the app keeps it as `config.toml.bad`, asks for the locations again, and writes a new one. If a database file is empty, cut short or not an avalio-lite database, the app reports it and never sets up a new database over it. Uninstalling never deletes your database or logs.

### Backups

**Settings → Backup → Export everything…** saves the whole database to one `.avalio` file while the app stays open. The file is encrypted with your PIN if you have set one.

- **Restore from a backup…** replaces everything in the app with the backup. It asks first, and keeps your current PIN.
- **Add classes from a backup…** copies a backup's classes, pupils and marks into the app without changing what's already there. Pupils with the same name are joined.

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

avalio-lite needs a licence key, which looks like `BRN-7KPX-4M9D-QW2R-8NFC` and comes by email. The app checks it with Clavren, Breren's licence server, at `clavren.breren.com`. Only the key and a code for this computer are sent; your marks never leave your computer.

- **Activating:** after choosing your folders (and typing your PIN, if the database has one), the app shows **Enter your licence key**. Type the key (capitals, spaces and dashes don't matter) and press **Activate** or Enter. This needs the internet.
- **Offline:** once activated, the app opens straight away, with or without internet, for up to 30 days after its last check.
- **Daily check:** while the computer is online, the app confirms the licence in the background once a day, at start or while it runs, without slowing anything down. If the licence was cancelled or has ended, the licence screen appears over the sheet (everything you typed is already saved); if it was extended, it goes away again by itself.
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

## Updates

- **Looking for a new version:** once a day, at start or while it runs, the app asks GitHub for the latest release. Nothing about you or your marks is sent. When a newer version is out, a bar above the sheet says *avalio-lite X.Y.Z is available*.
- **What's new** opens the release page, with the list of changes, in your browser.
- **Hide** (×) hides the bar for that version; it comes back for the next one.
- **Update now** downloads the new version and checks its signature. Each release is signed by the developer, and a file that doesn't match is deleted, never run. Then:
  - an **installed** copy closes and runs the new installer, which shows its progress and opens the app again;
  - a **portable** copy replaces its own exe and opens again. Its folder must be writable; if it isn't (a read-only USB stick, for example), the app says so before downloading.

  Everything you typed is already saved. The next start says *Updated to X.Y.Z*.
- **Settings → Updates:** **Check for updates automatically** turns the daily check on or off (it starts on). **Check now** looks straight away, even for a version you hid, and says *avalio-lite X.Y.Z is up to date* when there is nothing newer.
- If the check or the download fails (no internet, GitHub asking to wait, a file missing from the release), the app says why and stays as it was. Try again later, or download the new version by hand from the release page.
