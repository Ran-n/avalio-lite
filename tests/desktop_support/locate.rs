// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 14:38:05.162958
//+ Revised: 	2026/11/24 17:54:10.013015
// -----------------------------------------------------------------------------

//! Finding controls on screen, in physical pixels: the controls with a label (`locate`), the plain
//! texts showing a text (`locate_text`), and the controls of an open popup menu (`locate_in_menu`,
//! `locate_in_popup`), placed relative to the popup. Clicking them is in `input`.

use serde_json::{Value, json};

use super::App;
use super::pixels::Rect;

impl App {
    /// Where the controls labelled `label` are, in physical pixels.
    pub fn locate(&mut self, label: &str) -> Vec<Rect> {
        self.locate_from(label, false, false)
    }

    /// Where the plain texts showing exactly `text` are (a status line, a label), in physical pixels.
    pub fn locate_text(&mut self, text: &str) -> Vec<Rect> {
        let reply = self.send(json!({ "cmd": "locate", "label": text, "text": true }));
        self.rects(&reply, (0.0, 0.0), false)
    }

    /// Where the options labelled `label` of the open popup menu are, in physical pixels. Every menu
    /// option has a hint (its accessible description), which tells it apart from other controls.
    pub fn locate_in_menu(&mut self, label: &str) -> Vec<Rect> {
        self.locate_from(label, true, true)
    }

    /// Where the controls labelled `label` of the open popup are, in physical pixels, hint or not:
    /// for a menu whose options have none, such as the sheet's "⋯" menu.
    pub fn locate_in_popup(&mut self, label: &str) -> Vec<Rect> {
        self.locate_from(label, true, false)
    }

    /// Whether a popup (a menu) is open.
    pub fn popup_is_open(&mut self) -> bool {
        self.send(json!({ "cmd": "locate", "label": "" }))["popup"].is_object()
    }

    #[expect(
        clippy::fn_params_excessive_bools,
        reason = "test harness: where to look (an open popup or the window) and whether the options carry hints; each caller names its case"
    )]
    fn locate_from(&mut self, label: &str, in_popup: bool, hinted: bool) -> Vec<Rect> {
        let reply = self.send(json!({ "cmd": "locate", "label": label }));
        // a popup's controls are placed relative to the popup
        let (left, top) = if in_popup {
            let popup = &reply["popup"];
            assert!(popup.is_object(), "no popup menu is open");
            (popup["x"].as_f64().unwrap(), popup["y"].as_f64().unwrap())
        } else {
            (0.0, 0.0)
        };
        self.rects(&reply, (left, top), hinted)
    }

    #[expect(
        clippy::fn_params_excessive_bools,
        reason = "test harness: whether only the controls with a hint count (menu options; not a header of the same title)"
    )]
    fn rects(&self, reply: &Value, (left, top): (f64, f64), hinted: bool) -> Vec<Rect> {
        let physical = |value: f64| self.px(value);
        reply["found"]
            .as_array()
            .unwrap()
            .iter()
            // menu options carry their hint as a description; a header with the same title doesn't
            .filter(|found| !hinted || found["description"].as_str().is_some_and(|hint| !hint.is_empty()))
            .map(|found| Rect {
                x: physical(left + found["x"].as_f64().unwrap()),
                y: physical(top + found["y"].as_f64().unwrap()),
                width: physical(found["width"].as_f64().unwrap()),
                height: physical(found["height"].as_f64().unwrap()),
            })
            .collect()
    }
}
