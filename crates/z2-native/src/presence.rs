//! Discord Rich Presence: a line of game state ("West Hyrule", "Attack 3 · Magic 2 · Life 4 ·
//! 3 lives") shown on the player's Discord profile.
//!
//! [`from_ram`] reads the state from work RAM (offsets in `ram-map.toml`);
//! [`start`] spawns a thread that owns the IPC client and pushes the latest
//! value to the local Discord app. With the `discord` feature off (or on
//! Android) `start` returns `None` and nothing is sent.

use std::sync::mpsc::Sender;

/// Discord application ("z2rs") whose name and art assets the profile shows.
#[cfg(all(feature = "discord", not(target_os = "android")))]
const APP_ID: &str = "1554881636485701702";

/// The one art asset key, uploaded to the application above. It must be the
/// project's own logo (the Android launcher icon is original vector art);
/// never a screenshot, sprite or map from the game (see LEGAL.md).
#[cfg(all(feature = "discord", not(target_os = "android")))]
const LARGE_IMAGE: &str = "z2rs";

/// What the profile shows. Compared by the window loop so only changes are sent.
#[derive(Debug, Clone, PartialEq)]
pub struct Presence {
    details: String,
    state: String,
    party: (i32, i32),
}

/// Presence for the current RAM. `coop` = two Links are playing.
pub fn from_ram(ram: &[u8; 0x800], coop: bool) -> Presence {
    let details = match ram[0x0707] {
        0 => ["West Hyrule", "Death Mountain", "East Hyrule"][usize::from(ram[0x0706].min(2))],
        1 | 2 => "In a town",
        3 | 4 => "Exploring a palace",
        5 => "The Great Palace",
        _ => "Zelda awakens",
    };
    Presence {
        details: details.to_string(),
        state: format!(
            "Attack {} · Magic {} · Life {} · {} lives",
            ram[0x0777], ram[0x0778], ram[0x0779], ram[0x0700]
        ),
        party: if coop { (2, 2) } else { (1, 1) },
    }
}

/// Start the presence thread; `None` when Discord support is compiled out.
#[cfg(not(all(feature = "discord", not(target_os = "android"))))]
pub fn start() -> Option<Sender<Presence>> {
    None
}

/// Start the presence thread. It exits when the returned sender is dropped.
#[cfg(all(feature = "discord", not(target_os = "android")))]
pub fn start() -> Option<Sender<Presence>> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("discord-presence".into())
        .spawn(move || worker(rx))
        .ok()?;
    Some(tx)
}

#[cfg(all(feature = "discord", not(target_os = "android")))]
fn worker(rx: std::sync::mpsc::Receiver<Presence>) {
    use std::sync::mpsc::TryRecvError;

    // Discord drops activity updates faster than one per 15 s; the same
    // interval doubles as the reconnect backoff while Discord is closed.
    // ponytail: fixed 15 s retry forever; add exponential backoff if a
    // closed Discord ever shows up in profiles.
    const INTERVAL: std::time::Duration = std::time::Duration::from_secs(15);

    let started = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    let mut client = None;
    let mut want: Option<Presence> = None;
    let mut sent: Option<Presence> = None;
    let mut warned = false;
    loop {
        // Nothing pending: block until the loop sends something (or hangs up).
        if want == sent {
            match rx.recv() {
                Ok(p) => want = Some(p),
                Err(_) => return,
            }
        }
        // Coalesce: only the newest value matters.
        loop {
            match rx.try_recv() {
                Ok(p) => want = Some(p),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return,
            }
        }
        if let Some(p) = want.as_ref().filter(|p| sent.as_ref() != Some(*p)) {
            match push(&mut client, p, started) {
                Ok(()) => {
                    sent = want.clone();
                    warned = false;
                }
                Err(e) => {
                    if !warned {
                        eprintln!("discord presence: {e} (retrying every 15 s)");
                        warned = true;
                    }
                    client = None;
                }
            }
        }
        std::thread::sleep(INTERVAL);
    }
}

/// Connect if needed, then set the activity.
#[cfg(all(feature = "discord", not(target_os = "android")))]
fn push(
    client: &mut Option<discord_rich_presence::DiscordIpcClient>,
    p: &Presence,
    started: i64,
) -> Result<(), discord_rich_presence::error::Error> {
    use discord_rich_presence::activity::{Activity, Assets, Party, Timestamps};
    use discord_rich_presence::{DiscordIpc, DiscordIpcClient};

    let c = match client {
        Some(c) => c,
        None => {
            let mut c = DiscordIpcClient::new(APP_ID);
            c.connect()?;
            client.insert(c)
        }
    };
    let mut activity = Activity::new()
        .details(p.details.as_str())
        .state(p.state.as_str())
        .assets(Assets::new().large_image(LARGE_IMAGE))
        .timestamps(Timestamps::new().start(started));
    if p.party.1 > 1 {
        activity = activity.party(Party::new().size([p.party.0, p.party.1]));
    }
    c.set_activity(activity)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presence_from_ram() {
        let mut ram = [0u8; 0x800];
        ram[0x0700] = 3;
        ram[0x0777] = 2;
        ram[0x0778] = 1;
        ram[0x0779] = 4;
        ram[0x0706] = 1;
        let p = from_ram(&ram, false);
        assert_eq!(p.details, "Death Mountain");
        assert_eq!(p.state, "Attack 2 · Magic 1 · Life 4 · 3 lives");
        assert_eq!(p.party, (1, 1));

        ram[0x0707] = 4;
        let p = from_ram(&ram, true);
        assert_eq!(
            (p.details.as_str(), p.party),
            ("Exploring a palace", (2, 2))
        );
        ram[0x0707] = 5;
        assert_eq!(from_ram(&ram, false).details, "The Great Palace");
    }
}
