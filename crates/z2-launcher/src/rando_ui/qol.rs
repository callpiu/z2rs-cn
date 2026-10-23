//! Quality-of-life options (never change the generated game, the seed or the
//! hash code).

use egui::Ui;
use crate::i18n::Lang;
use z2_rando::flags::QolFlags;

use super::{check, choice, grid};

/// Widgets for [`QolFlags`].
pub fn ui(ui: &mut Ui, lang: Lang, f: &mut QolFlags) {
    grid(ui, "rando-qol", |ui| {
        check(ui, lang,
            "Fast text",
            "Dialog boxes open and print without the letter-by-letter delay.",
            &mut f.fast_text,
        );
        check(ui, lang,
            "Bug fixes",
            "Fix small original bugs: the 300-point enemies really give 300 \
             (not 301), and defeating Horsehead no longer clears an item held in \
             another enemy slot.",
            &mut f.bug_fixes,
        );
        check(ui, lang,
            "Remove flashing",
            "The screen does not flash when Link dies or on the game-over screen; \
             one steady colour is shown instead.",
            &mut f.remove_flashing,
        );
        check(ui, lang,
            "Dark Thunderbird room",
            "No Thunder screen flash while Thunderbird is in the room (helps with \
             photosensitivity).",
            &mut f.darken_thunderbird,
        );
        check(ui, lang,
            "Updated HUD",
            "The status bar shows lives, keys and crystals. Not available yet: \
             the original status bar is kept.",
            &mut f.updated_hud,
        );
        check(ui, lang,
            "Steady HUD on lag",
            "The status bar does not flash on lag frames. Not available yet.",
            &mut f.disable_hud_lag,
        );
        check(ui, lang,
            "Fast spell casting",
            "Select casts the selected spell again without reopening the pause \
             menu.",
            &mut f.fast_spell_casting,
        );
        check(ui, lang,
            "Up+Select on controller 1",
            "Holding Up+Select on the first controller opens the save prompt, like \
             Up+A on the second controller (which still works).",
            &mut f.up_a_on_controller_1,
        );
        choice(ui, lang,
            "Low-health beep at",
            "How low the life bar must be before the warning beep starts.",
            &mut f.beep_threshold,
        );
        choice(ui, lang,
            "Low-health beep speed",
            "How often the warning beeps, or Off for no beep at all.",
            &mut f.beep_frequency,
        );
    });
}
