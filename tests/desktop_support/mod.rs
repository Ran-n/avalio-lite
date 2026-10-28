// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/28 15:53:07.485818
//+ Revised: 	2026/10/28 22:55:05.200796
// -----------------------------------------------------------------------------

//! The desktop test harness: the real app started through `examples/app_driver.rs`, its window
//! driven through Win32 (`window`), and its pixels checked against theme colours and reviewed
//! baselines (`pixels`).

pub mod pixels;
pub mod window;

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

use avalio_lite::config::CONFIG_DIR_ENV;
use serde_json::{Value, json};

use pixels::{Image, Rect, Rgb};
use window::AppWindow;

const REPLY_TIMEOUT: Duration = Duration::from_secs(20);
const UPDATE_ENV: &str = "AVALIO_UPDATE_BASELINES";
/// How long typed text may take to show up in a field, and how often a field is typed into.
const TYPING_WAIT: Duration = Duration::from_secs(3);
const TYPING_ATTEMPTS: u32 = 2;
/// Pixels a blinking text caret changes between two captures (about 2 x 24, with some to spare).
const CARET_PIXELS: usize = 64;
/// Per channel: anti-aliasing may wobble by a step or two between runs.
const PIXEL_TOLERANCE: u8 = 12;
/// Share of the screen allowed to differ before a snapshot fails (a blinking caret, say).
const CHANGED_SHARE: f64 = 0.001;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// One window on screen at a time: the tests take turns.
pub fn turn() -> MutexGuard<'static, ()> {
    static TURN: Mutex<()> = Mutex::new(());
    TURN.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Builds the driver once per test run, with the same toolchain and profile as the tests.
fn driver_exe() -> PathBuf {
    static BUILT: OnceLock<PathBuf> = OnceLock::new();
    BUILT
        .get_or_init(|| {
            let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
            let status = Command::new(cargo)
                .args(["build", "--quiet", "--example", "app_driver", "--manifest-path"])
                .arg(manifest_dir().join("Cargo.toml"))
                .status()
                .expect("cargo runs");
            assert!(status.success(), "building the app driver failed");
            // target/<profile>/deps/desktop-<hash>.exe -> target/<profile>/examples/app_driver.exe
            let deps = std::env::current_exe().unwrap().parent().unwrap().to_path_buf();
            deps.parent().unwrap().join("examples").join("app_driver.exe")
        })
        .clone()
}

/// The real app in its own process, with its window.
pub struct App {
    child: Child,
    stdin: ChildStdin,
    replies: Receiver<Value>,
    pub window: AppWindow,
    /// Physical pixels per logical pixel.
    pub scale: f64,
}

impl App {
    /// Starts the app on the settings in `config_dir`, on a Windows set to `language`.
    pub fn start(config_dir: &Path, language: &str) -> Self {
        let log = std::fs::File::create(config_dir.with_extension("driver.log")).unwrap();
        let mut child = Command::new(driver_exe())
            .arg(config_dir)
            .arg(language)
            // whatever the app suggests or creates stays in the scratch folder, which is also where
            // the relative paths of the test settings start (so screens don't show a random path)
            .env(CONFIG_DIR_ENV, config_dir)
            .current_dir(config_dir.parent().unwrap())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(log)
            .spawn()
            .expect("the app driver starts");
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (sender, replies) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if let Some(reply) = line.strip_prefix("@@") {
                    let _ = sender.send(serde_json::from_str(reply).expect("replies are JSON"));
                }
            }
        });
        let window = AppWindow::of_process(child.id(), Duration::from_secs(20));
        let mut app = Self { child, stdin, replies, window, scale: 1.0 };
        app.scale = app.send(json!({ "cmd": "ping" }))["scale"].as_f64().unwrap();
        app
    }

    /// Sends one command and waits for its answer; a failed command fails the test.
    pub fn send(&mut self, command: Value) -> Value {
        writeln!(self.stdin, "{command}").expect("the driver is running");
        self.stdin.flush().unwrap();
        let reply = self.replies.recv_timeout(REPLY_TIMEOUT).unwrap_or_else(|_| panic!("no answer to {command}"));
        assert_eq!(reply["ok"], json!(true), "{command} failed: {reply}");
        reply
    }

    pub fn state(&mut self) -> Value {
        self.send(json!({ "cmd": "state" }))
    }

    /// Logical size, as the user would set it by resizing.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.send(json!({ "cmd": "size", "width": width, "height": height }));
        self.settle();
    }

    /// Light or dark, without saving it: only what the screen looks like.
    pub fn theme(&mut self, dark: bool) {
        self.send(json!({ "cmd": "theme", "dark": dark }));
        self.settle();
    }

    /// Answers for the next folder pickers, file pickers and Yes/No questions.
    pub fn answer_dialogs(&mut self, folders: &[&Path], files: &[&Path], answers: &[bool]) {
        self.send(json!({ "cmd": "dialogs", "folders": folders, "files": files, "answers": answers }));
    }

    /// Where the controls labelled `label` are, in physical pixels.
    pub fn locate(&mut self, label: &str) -> Vec<Rect> {
        self.locate_from(label, false)
    }

    /// Where the options labelled `label` of the open popup menu are, in physical pixels. Every menu
    /// option has a hint (its accessible description), which tells it apart from other controls.
    pub fn locate_in_menu(&mut self, label: &str) -> Vec<Rect> {
        self.locate_from(label, true)
    }

    fn locate_from(&mut self, label: &str, in_menu: bool) -> Vec<Rect> {
        let reply = self.send(json!({ "cmd": "locate", "label": label }));
        // a popup's controls are placed relative to the popup
        let (left, top) = if in_menu {
            let popup = &reply["popup"];
            assert!(popup.is_object(), "no popup menu is open");
            (popup["x"].as_f64().unwrap(), popup["y"].as_f64().unwrap())
        } else {
            (0.0, 0.0)
        };
        let scale = self.scale;
        let physical = |value: f64| (value * scale).round() as u32;
        reply["found"]
            .as_array()
            .unwrap()
            .iter()
            // menu options carry their hint as a description; a header with the same title doesn't
            .filter(|found| !in_menu || found["description"].as_str().is_some_and(|hint| !hint.is_empty()))
            .map(|found| Rect {
                x: physical(left + found["x"].as_f64().unwrap()),
                y: physical(top + found["y"].as_f64().unwrap()),
                width: physical(found["width"].as_f64().unwrap()),
                height: physical(found["height"].as_f64().unwrap()),
            })
            .collect()
    }

    /// A real click on the option labelled `label` of the open popup menu.
    pub fn click_in_menu(&mut self, label: &str) {
        let found = self.locate_in_menu(label);
        assert_eq!(found.len(), 1, "expected one menu option {label:?}, found {found:?}");
        self.click_at(found[0]);
    }

    fn click_at(&mut self, target: Rect) {
        self.window.click((target.x + target.width / 2) as i32, (target.y + target.height / 2) as i32);
        self.settle();
    }

    fn the(&mut self, label: &str, index: usize) -> Rect {
        let found = self.locate(label);
        *found.get(index).unwrap_or_else(|| panic!("no control {index} labelled {label:?} on screen: {found:?}"))
    }

    /// A real click in the middle of the `index`-th control labelled `label`.
    pub fn click_nth(&mut self, label: &str, index: usize) {
        let target = self.the(label, index);
        self.click_at(target);
    }

    pub fn click(&mut self, label: &str) {
        let found = self.locate(label);
        assert_eq!(found.len(), 1, "expected one control labelled {label:?}, found {found:?}");
        self.click_nth(label, 0);
    }

    /// Clicks into the field labelled `label`, clears it and types `text`, then waits until the
    /// field holds exactly that. The keys are posted, so the app may still be working through
    /// them; and a key that lands before the click has focused the field is lost, so a field
    /// that doesn't end up with `text` is typed into once more before the test fails.
    pub fn type_into(&mut self, label: &str, text: &str) {
        for attempt in 1..=TYPING_ATTEMPTS {
            self.clear_and_type(label, text);
            if self.wait_for_text(label, text) {
                return;
            }
            eprintln!("typing {text:?} into {label:?} didn't take (attempt {attempt}); typing again");
        }
        panic!("the field {label:?} never held {text:?}; it holds {:?}", self.text_of(label));
    }

    fn clear_and_type(&mut self, label: &str, text: &str) {
        const VK_END: u16 = 0x23;
        const VK_BACK: u16 = 0x08;
        self.click(label);
        self.window.press(VK_END);
        for _ in 0..80 {
            self.window.press(VK_BACK);
        }
        self.window.type_text(text);
        self.settle();
    }

    /// Polls the field until it holds `text`, for up to `TYPING_WAIT`.
    fn wait_for_text(&mut self, label: &str, text: &str) -> bool {
        let deadline = Instant::now() + TYPING_WAIT;
        while Instant::now() < deadline {
            if self.text_of(label) == text {
                return true;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        false
    }

    /// The text of the one field labelled `label`.
    fn text_of(&mut self, label: &str) -> String {
        let reply = self.send(json!({ "cmd": "locate", "label": label }));
        let found = reply["found"].as_array().cloned().unwrap_or_default();
        assert_eq!(found.len(), 1, "expected one field labelled {label:?}, found {found:?}");
        found[0]["value"].as_str().unwrap_or_default().to_owned()
    }

    pub fn press_enter(&mut self) {
        const VK_RETURN: u16 = 0x0D;
        self.window.press(VK_RETURN);
        self.settle();
    }

    pub fn press_tab(&mut self) {
        const VK_TAB: u16 = 0x09;
        self.window.press(VK_TAB);
        self.settle();
    }

    pub fn scroll(&mut self, label: &str, notches: i32) {
        let target = self.the(label, 0);
        self.window.scroll((target.x + target.width / 2) as i32, (target.y + target.height / 2) as i32, notches);
        self.settle();
    }

    /// Waits until the event loop has handled everything sent so far and the frame is drawn.
    pub fn settle(&mut self) {
        self.send(json!({ "cmd": "ping" }));
        std::thread::sleep(Duration::from_millis(150));
    }

    /// The window as it is now, once two captures in a row agree apart from a blinking caret
    /// (animations have finished).
    pub fn capture(&mut self) -> Image {
        self.window.mouse_away();
        self.settle();
        let mut previous = self.window.capture();
        for _ in 0..20 {
            std::thread::sleep(Duration::from_millis(120));
            let current = self.window.capture();
            // a focused field's caret blinks on and off for as long as it has focus, so a
            // caret's worth of change still counts as settled
            if current.difference(&previous, 0).0 <= CARET_PIXELS {
                return current;
            }
            previous = current;
        }
        let failures = manifest_dir().join("target").join("desktop-failures");
        let current = self.window.capture();
        let (changed, marked) = current.difference(&previous, 0);
        previous.save(&failures.join("unsettled-previous.png"));
        current.save(&failures.join("unsettled-current.png"));
        marked.save(&failures.join("unsettled-diff.png"));
        panic!("the window never stopped changing ({changed} pixels still changing); see {}", failures.display());
    }

    /// Compares the window with `baselines/<name>@<scale>.png`. On a difference, the capture and a
    /// marked diff are written next to the target folder for review. With `AVALIO_UPDATE_BASELINES=1`
    /// the baseline is rewritten instead (open and review each one before committing it).
    pub fn snapshot(&mut self, name: &str) -> Image {
        let image = self.capture();
        let file = format!("{name}@{}.png", (self.scale * 100.0).round() as u32);
        let baseline = manifest_dir().join("tests").join("desktop").join("baselines").join(&file);
        if std::env::var_os(UPDATE_ENV).is_some_and(|value| value == "1") {
            image.save(&baseline);
            return image;
        }
        let expected = Image::load(&baseline)
            .unwrap_or_else(|| panic!("no baseline {file}; create it with {UPDATE_ENV}=1 and review it"));
        let failures = manifest_dir().join("target").join("desktop-failures");
        if (expected.width, expected.height) != (image.width, image.height) {
            image.save(&failures.join(file.replace(".png", "-actual.png")));
            panic!(
                "{file}: the screen is {}x{}, the baseline {}x{}",
                image.width, image.height, expected.width, expected.height
            );
        }
        let (changed, marked) = image.difference(&expected, PIXEL_TOLERANCE);
        let allowed = (f64::from(image.width * image.height) * CHANGED_SHARE) as usize;
        if changed > allowed {
            image.save(&failures.join(file.replace(".png", "-actual.png")));
            marked.save(&failures.join(file.replace(".png", "-diff.png")));
            panic!("{file}: {changed} pixels changed (allowed {allowed}); see {}", failures.display());
        }
        image
    }

    /// Closes the window the way the user would and waits for the app to shut down.
    pub fn quit(mut self) {
        self.send(json!({ "cmd": "quit" }));
        let goodbye = self.replies.recv_timeout(REPLY_TIMEOUT).expect("the app says it shut down");
        assert_eq!(goodbye["quit"], json!(true));
        let status = self.child.wait().unwrap();
        assert!(status.success(), "the app exited with {status}");
    }
}

impl Drop for App {
    fn drop(&mut self) {
        // a failed test still mustn't leave a window behind
        if matches!(self.child.try_wait(), Ok(None)) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

/// The colour tokens of `ui/theme.slint`, light and dark, read from the file itself.
pub struct ThemeColours {
    tokens: HashMap<String, (Rgb, Rgb)>,
}

impl ThemeColours {
    pub fn load() -> Self {
        let source = std::fs::read_to_string(manifest_dir().join("ui").join("theme.slint")).unwrap();
        let mut tokens = HashMap::new();
        for line in source.lines() {
            let Some(rest) = line.trim().strip_prefix("out property <color> ") else { continue };
            let Some((name, value)) = rest.split_once(": dark ? ") else { continue };
            let Some((dark, light)) = value.trim_end_matches(';').split_once(" : ") else { continue };
            // only opaque colours; shadows have an alpha and never show as themselves
            if dark.len() == 7 && light.len() == 7 {
                tokens.insert(name.to_owned(), (Rgb::hex(dark), Rgb::hex(light)));
            }
        }
        assert!(tokens.len() > 15, "theme.slint's colours weren't found");
        Self { tokens }
    }

    pub fn get(&self, name: &str, dark: bool) -> Rgb {
        let (dark_colour, light_colour) = self.tokens[name];
        if dark { dark_colour } else { light_colour }
    }
}
