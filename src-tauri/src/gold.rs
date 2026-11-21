//! The team gold strip: while Tab is held in a game (the game's own scoreboard key), each team's
//! item value and the gap show at the top of the game. The numbers come from the game's own live
//! data (every player's items, as the scoreboard lists them) and are added up by the strip's page.
//!
//! Tab is read by asking the system whether it is down, a few dozen times a second while a game
//! runs: no hook and nothing kept from the game. The strip shows only while the game has the
//! keyboard (Tab in another app is not the scoreboard).

use crate::core::Core;
use crate::os;
use std::sync::Arc;
use std::time::Duration;

/// How often Tab is looked at during a game (a press shows the strip within this).
const EVERY: Duration = Duration::from_millis(30);
/// Between games.
const IDLE: Duration = Duration::from_millis(500);

pub fn spawn(core: Arc<Core>) {
    std::thread::Builder::new()
        .name("gold".into())
        .spawn(move || {
            // Tab went down while the game did not have the keyboard: wait for its release.
            let (mut shown, mut ignored) = (false, false);
            loop {
                let active = core.gold_diff_on() && core.ingame.lock().is_some();
                if !active {
                    if shown {
                        core.hide_gold();
                    }
                    (shown, ignored) = (false, false);
                    std::thread::sleep(IDLE);
                    continue;
                }
                let down = os::tab_down();
                if !down {
                    if shown {
                        core.hide_gold();
                    }
                    (shown, ignored) = (false, false);
                } else if !shown && !ignored {
                    if os::game_window().is_some_and(os::has_focus) {
                        core.show_gold();
                        shown = true;
                    } else {
                        ignored = true;
                    }
                }
                std::thread::sleep(EVERY);
            }
        })
        .expect("gold thread");
}
