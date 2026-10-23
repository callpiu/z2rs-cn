//! Enemy options: shuffles, hit points, experience and immunities.

use egui::Ui;
use crate::i18n::Lang;
use z2_rando::flags::{EnemyFlags, EnemyLife};

use super::{check, choice, choice_from, grid, tri};

const BOSS_HP: &[EnemyLife] = &[
    EnemyLife::Vanilla,
    EnemyLife::Narrow,
    EnemyLife::Medium,
    EnemyLife::MediumHigh,
    EnemyLife::High,
];

/// Widgets for [`EnemyFlags`].
pub fn ui(ui: &mut Ui, lang: Lang, f: &mut EnemyFlags) {
    grid(ui, "rando-enemies", |ui| {
        tri(ui, lang,
            "Shuffle overworld enemies",
            "Encounter enemies are shuffled within their continent.",
            &mut f.shuffle_overworld_enemies,
        );
        tri(ui, lang,
            "Shuffle palace enemies",
            "Room enemies are shuffled within their palace group.",
            &mut f.shuffle_palace_enemies,
        );
        tri(ui, lang,
            "Mix large and small enemies",
            "Small and large ground enemies may replace each other (flyers stay flyers, generators stay generators).",
            &mut f.mix_large_and_small,
        );
        check(ui, lang,
            "Generators always match",
            "Left and right spawners in a room spawn the same enemy.",
            &mut f.generators_always_match,
        );
        choice(ui, lang,
            "Dripper enemy",
            "What the palace drippers spawn.",
            &mut f.dripper_enemy,
        );
        choice(ui, lang,
            "Enemy HP",
            "Range regular enemy hit points are scaled by (each enemy type gets its own roll, capped at 255).",
            &mut f.enemy_hp,
        );
        choice_from(ui, lang,
            "Boss HP",
            "Range boss hit points are scaled by (Great Palace bosses excluded).",
            &mut f.boss_hp,
            BOSS_HP,
        );
        check(ui, lang,
            "Shuffle experience stealers",
            "Which enemies steal experience when they hit you (the same number per area as in the original).",
            &mut f.shuffle_xp_stealers,
        );
        check(ui, lang,
            "Shuffle stolen amount",
            "Stolen experience is 50-150% of the original.",
            &mut f.shuffle_xp_stolen_amount,
        );
        choice(ui, lang,
            "Sword immunity",
            "Which enemies can only be hurt with Fire. Shuffle keeps how many there are; the conditional choice removes them all when Fire is replaced by Dash or tied to Fairy.",
            &mut f.sword_immunity,
        );
        choice(ui, lang,
            "Experience drops",
            "How much experience enemies give.",
            &mut f.xp_drops,
        );
        check(ui, lang,
            "Randomize knockback",
            "Chaotic knockback when Link is hit.",
            &mut f.randomize_knockback,
        );
    });
}
