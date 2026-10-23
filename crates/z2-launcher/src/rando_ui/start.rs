//! Start options: starting items, spells, containers, techniques, lives,
//! levels and the random-switch rate.

use egui::Ui;
use crate::i18n::Lang;
use z2_rando::flags::StartFlags;

use super::{check, choice, grid, heading, min_max, range};

/// Widgets for [`StartFlags`].
pub fn ui(ui: &mut Ui, lang: Lang, f: &mut StartFlags) {
    grid(ui, "rando-start", |ui| {
        heading(ui, lang, "Items");
        check(ui, lang,
            "Shuffle starting items",
            "Each item not ticked below gets a 25% chance to be in the starting inventory.",
            &mut f.shuffle_starting_items,
        );
        check(ui, lang,
            "Candle",
            "Start with the candle.",
            &mut f.start_with_candle,
        );
        check(ui, lang,
            "Glove",
            "Start with the glove.",
            &mut f.start_with_glove,
        );
        check(ui, lang, "Raft", "Start with the raft.", &mut f.start_with_raft);
        check(ui, lang,
            "Boots",
            "Start with the boots.",
            &mut f.start_with_boots,
        );
        check(ui, lang,
            "Flute",
            "Start with the flute.",
            &mut f.start_with_flute,
        );
        check(ui, lang,
            "Cross",
            "Start with the cross.",
            &mut f.start_with_cross,
        );
        check(ui, lang,
            "Hammer",
            "Start with the hammer.",
            &mut f.start_with_hammer,
        );
        check(ui, lang,
            "Magic key",
            "Start with the magic key.",
            &mut f.start_with_magic_key,
        );
        choice(ui, lang,
            "Starting item limit",
            "Most items the start may hold, ticked ones included (picked at random when more are ticked).",
            &mut f.start_items_limit,
        );
        heading(ui, lang, "Spells");
        check(ui, lang,
            "Shuffle starting spells",
            "Each spell not ticked below gets a 25% chance to be known at the start.",
            &mut f.shuffle_starting_spells,
        );
        check(ui, lang,
            "Shield",
            "Start knowing Shield.",
            &mut f.start_with_shield,
        );
        check(ui, lang, "Jump", "Start knowing Jump.", &mut f.start_with_jump);
        check(ui, lang, "Life", "Start knowing Life.", &mut f.start_with_life);
        check(ui, lang, "Fairy", "Start knowing Fairy.", &mut f.start_with_fairy);
        check(ui, lang,
            "Fire",
            "Start knowing Fire (or Dash when Fire is replaced).",
            &mut f.start_with_fire,
        );
        check(ui, lang,
            "Reflect",
            "Start knowing Reflect.",
            &mut f.start_with_reflect,
        );
        check(ui, lang, "Spell", "Start knowing Spell.", &mut f.start_with_spell);
        check(ui, lang,
            "Thunder",
            "Start knowing Thunder.",
            &mut f.start_with_thunder,
        );
        choice(ui, lang,
            "Starting spell limit",
            "Most spells the start may hold.",
            &mut f.start_spells_limit,
        );
        heading(ui, lang, "Containers and techniques");
        min_max(ui, lang,
            "Starting heart containers",
            "Link starts with a number of heart containers in this range (vanilla 4).",
            &mut f.heart_containers_min,
            &mut f.heart_containers_max,
            1,
            8,
        );
        min_max(ui, lang,
            "Starting magic containers",
            "Link starts with a number of magic containers in this range (vanilla 4).",
            &mut f.magic_containers_min,
            &mut f.magic_containers_max,
            1,
            8,
        );
        choice(ui, lang,
            "Total heart containers",
            "How many heart containers exist. Fewer than the start plus four turns the \
             spare ones into small items; more turns small items into hearts.",
            &mut f.max_heart_containers,
        );
        choice(ui, lang,
            "Starting sword techniques",
            "Downstab and/or upstab known from the start.",
            &mut f.starting_techs,
        );
        choice(ui, lang,
            "Starting lives",
            "Lives at the start and after a game over (vanilla 3).",
            &mut f.starting_lives,
        );
        heading(ui, lang, "Starting levels");
        range(ui, lang,
            "Attack level",
            "Starting attack level (vanilla 1).",
            &mut f.attack_level,
            1,
            8,
        );
        range(ui, lang,
            "Magic level",
            "Starting magic level (vanilla 1).",
            &mut f.magic_level,
            1,
            8,
        );
        range(ui, lang,
            "Life level",
            "Starting life level (vanilla 1).",
            &mut f.life_level,
            1,
            8,
        );
        heading(ui, lang, "Random switches");
        choice(ui, lang,
            "Random switch rate",
            "How likely an option set to Random comes out on.",
            &mut f.random_flag_rate,
        );
        check(ui, lang,
            "Allow difficulty levels",
            "Racers share one base seed while picking different difficulty-only options. Recorded in the flags; z2rs does not yet split the seed by difficulty.",
            &mut f.share_seed_across_difficulty,
        );
    });
}
