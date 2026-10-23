//! Enemy drop options.

use egui::Ui;
use crate::i18n::Lang;
use z2_rando::flags::{DropFlags, DropPool};

use super::{check, grid, heading};

fn pool(ui: &mut Ui, lang: Lang, who: &str, p: &mut DropPool) {
    for (label, v) in p.entries_mut() {
        let tip = lang.t("{who} enemies can drop: {label}.")
            .replace("{who}", lang.t(who)).replace("{label}", lang.t(label));
        check(ui, lang, label, &tip, v);
    }
}

/// Widgets for [`DropFlags`].
pub fn ui(ui: &mut Ui, lang: Lang, f: &mut DropFlags) {
    grid(ui, "rando-drops", |ui| {
        check(ui, lang,
            "Shuffle drop frequency",
            "An item drops every 4 to 8 kills instead of every 6.",
            &mut f.shuffle_drop_frequency,
        );
        check(ui, lang,
            "Randomize drops",
            "Each unticked item below has a 50% chance to join its pool.",
            &mut f.randomize_drops,
        );
        check(ui, lang,
            "Standardize drops",
            "Drops follow a fixed sequence for the seed.",
            &mut f.standardize_drops,
        );
        heading(ui, lang, "Small enemy pool");
        pool(ui, lang, "Small", &mut f.small_pool);
        heading(ui, lang, "Large enemy pool");
        pool(ui, lang, "Large", &mut f.large_pool);
    });
}
