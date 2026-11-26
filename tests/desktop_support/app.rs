// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 14:38:04.410584
//+ Revised: 	2026/11/25 16:21:08.445165
// -----------------------------------------------------------------------------

//! The real app in its own process, with its window, and the replies its driver sends back. A test
//! starts one (`start` and the variants for other settings), sends it commands (`send`, `state`,
//! `wait_for`) and quits it (`quit`, `quit_timed`, `wait_closed`); the turn lock in `binaries` keeps
//! two tests' windows from showing at once. Clicks, setup and captures are in the sibling modules.

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use avalio_lite::platform::config::CONFIG_DIR_ENV;
use serde_json::{Value, json};

use super::binaries::driver_exe;
use super::window::AppWindow;

const REPLY_TIMEOUT: Duration = Duration::from_secs(20);
/// How long the app may take to show its window, and then to give its first answer, which waits for
/// the event loop. A debug build that starts on a busy machine has taken over 20 s for each.
const STARTUP_TIMEOUT: Duration = Duration::from_secs(45);

/// The real app in its own process, with its window.
pub struct App {
    child: Child,
    stdin: ChildStdin,
    replies: Receiver<Value>,
    pub window: AppWindow,
    /// Physical pixels per logical pixel.
    pub scale: f64,
}

/// Passes the driver's replies (its `@@`-marked output lines) to the test, until the driver ends.
fn forward_replies(stdout: ChildStdout, sender: &mpsc::Sender<Value>) {
    let replies = BufReader::new(stdout).lines().map_while(Result::ok);
    for reply in replies.filter_map(|line| line.strip_prefix("@@").map(str::to_owned)) {
        // the test may have stopped listening; then nobody needs the reply
        let _ = sender.send(serde_json::from_str(&reply).expect("replies are JSON"));
    }
}

impl App {
    /// Starts the app on the settings in `config_dir`, on a Windows set to `language`.
    pub fn start(config_dir: &Path, language: &str) -> Self {
        Self::start_with(config_dir, language, &[])
    }

    /// Like `start`, on a computer whose licence key was never activated: the licence screen.
    pub fn start_unlicensed(config_dir: &Path, language: &str) -> Self {
        Self::start_with(config_dir, language, &["unlicensed"])
    }

    /// Like `start`, on a computer whose licence needs the internet within days: the sheet warns.
    pub fn start_offline_soon(config_dir: &Path, language: &str) -> Self {
        Self::start_with(config_dir, language, &["offline-soon"])
    }

    /// Like `start`, as a build from source: no update controls.
    pub fn start_from_source(config_dir: &Path, language: &str) -> Self {
        Self::start_with(config_dir, language, &["licensed", "source"])
    }

    fn start_with(config_dir: &Path, language: &str, extra: &[&str]) -> Self {
        let log_path = config_dir.with_extension("driver.log");
        let log = std::fs::File::create(&log_path).unwrap();
        let mut child = Command::new(driver_exe())
            .arg(config_dir)
            .arg(language)
            .args(extra)
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
        std::thread::spawn(move || forward_replies(stdout, &sender));
        let Some(window) = AppWindow::of_process(child.id(), STARTUP_TIMEOUT) else {
            // no App owns the child yet, so its Drop can't close it: a window that shows up late
            // would stay open after the run, holding the test's output pipe open with it
            let _ = child.kill();
            let _ = child.wait();
            panic!("the app showed no window within {STARTUP_TIMEOUT:?}; see {}", log_path.display());
        };
        let mut app = Self { child, stdin, replies, window, scale: 1.0 };
        // the run stays off the user's screen unless it asks to see the window
        let shown = crate::common::win32::window_shown();
        assert_eq!(
            app.window.is_hidden(),
            !shown,
            "the window is hidden unless {} is 1",
            crate::common::win32::SHOW_WINDOW_ENV
        );
        app.scale = app.send_within(json!({ "cmd": "ping" }), STARTUP_TIMEOUT)["scale"].as_f64().unwrap();
        // a click before the first frames are drawn (the SVG logo is rasterized then) can be lost,
        // so tests only start once the window shows a settled picture
        app.capture();
        app
    }

    /// Sends one command and waits for its answer; a failed command fails the test.
    pub fn send(&mut self, command: Value) -> Value {
        self.send_within(command, REPLY_TIMEOUT)
    }

    fn send_within(&mut self, command: Value, limit: Duration) -> Value {
        writeln!(self.stdin, "{command}").expect("the driver is running");
        self.stdin.flush().unwrap();
        let reply = self.replies.recv_timeout(limit).unwrap_or_else(|_| panic!("no answer to {command}"));
        assert_eq!(reply["ok"], json!(true), "{command} failed: {reply}");
        reply
    }

    pub fn state(&mut self) -> Value {
        self.send(json!({ "cmd": "state" }))
    }

    /// Polls the driver's state until `done` holds; fails after 20 s.
    pub fn wait_for(&mut self, what: &str, done: impl Fn(&Value) -> bool) -> Value {
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            let state = self.state();
            if done(&state) {
                return state;
            }
            assert!(Instant::now() < deadline, "{what} never happened: {state}");
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    /// Waits for the app to close by itself (an update restarting it) within `limit`, and returns
    /// its goodbye: `{"quit": true, "restart": <program or null>}`.
    pub fn wait_closed(mut self, limit: Duration) -> Value {
        let goodbye = self.replies.recv_timeout(limit).expect("the app closed by itself");
        assert_eq!(goodbye["quit"], json!(true), "{goodbye}");
        let status = self.child.wait().unwrap();
        assert!(status.success(), "the app exited with {status}");
        goodbye
    }

    /// Quits and returns how long the app took to exit after being asked.
    pub fn quit_timed(mut self) -> Duration {
        let asked = Instant::now();
        self.send(json!({ "cmd": "quit" }));
        let goodbye = self.replies.recv_timeout(REPLY_TIMEOUT).expect("the app says it shut down");
        assert_eq!(goodbye["quit"], json!(true));
        let status = self.child.wait().unwrap();
        assert!(status.success(), "the app exited with {status}");
        asked.elapsed()
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
