//! Forever's ratings (addon/Polestar/Gear/Gear.lua, Weights.lua, PawnStats.lua, #1529): an item's ratings read at the
//! client's points per 1%, and haste, expertise and armor penetration weighed in the bands (docs/gear-weights.md).

use forge_addontest::{emitted, Addon, RunAddon};
use serde_json::{json, Value};

fn run(script: &str) -> Vec<Value> {
	emitted(&RunAddon {
		stubs: vec!["stub_world.lua", "stub_gear.lua"],
		before: r#"w = stub.world; g = stub.gear
w.class, w.className, w.classID, w.level = "WARRIOR", "Warrior", 1, 30
w.race, w.raceName, w.raceID, w.faction = "Troll", "Troll", 8, "Horde""#
			.into(),
		addons: vec![Addon::files("Polestar", &["Core/Core.xml", "Gear/Gear.xml"])],
		login: true,
		script: ["G = Polestar.Gear\n", script].concat(),
		..Default::default()
	})
}

#[test]
fn an_item_s_ratings_count_at_the_client_s_points_per_1_percent_and_plain_hit_and_crit_count_for_spells_too() {
	let out = run(r#"
		g.defineItem("item:9101", { equipLoc = "INVTYPE_CHEST", classID = 4, subclassID = 4, minLevel = 20, stats = {
			ITEM_MOD_STRENGTH_SHORT = 10, ITEM_MOD_CRIT_RATING_SHORT = 14, ITEM_MOD_HIT_RATING_SHORT = 10, ITEM_MOD_CRIT_SPELL_RATING_SHORT = 7,
			ITEM_MOD_HASTE_RATING_SHORT = 20, ITEM_MOD_EXPERTISE_RATING_SHORT = 5, ITEM_MOD_ARMOR_PENETRATION_RATING_SHORT = 10,
			ITEM_MOD_DODGE_RATING_SHORT = 12, ITEM_MOD_PARRY_RATING_SHORT = 15, ITEM_MOD_BLOCK_RATING_SHORT = 5,
			ITEM_MOD_DEFENSE_SKILL_RATING_SHORT = 3, ITEM_MOD_HIT_MELEE_RATING_SHORT = 20 } })
		emit(G.GetItemInfo("item:9101").stats)
	"#);
	assert_eq!(
		out[0],
		json!({ "str": 10, "crit": 1, "spellCrit": 1.5, "hit": 3, "spellHit": 1, "haste": 2, "expertise": 0.5, "armorPen": 1,
			"dodge": 1, "parry": 1, "block": 1, "defense": 3 }),
		"14 crit rating is 1% (melee and spell), 10 hit 1%, 20 haste 2%; defense rating is defense skill"
	);
}

#[test]
fn every_band_weighs_haste_weapon_specs_expertise_and_physical_damage_armor_penetration() {
	let out = run(r#"
		local out = {}
		for class, meta in pairs(G.CLASSES) do
			for _, s in ipairs(meta.specs) do
				local w = G.WeightsFor(class, s.key, 60)
				out[class .. "." .. s.key] = { haste = w.haste or 0, expertise = w.expertise or 0, armorPen = w.armorPen or 0, crit = w.crit or 0 }
			end
		end
		local stat = {}
		for _, k in ipairs(G.STAT_KEYS) do stat[k] = true end
		emit({ specs = out, keys = { stat.haste or false, stat.expertise or false, stat.armorPen or false } })
	"#);
	let specs = out[0]["specs"].as_object().unwrap();
	assert_eq!(specs.len(), 27);
	assert_eq!(out[0]["keys"], json!([true, true, true]));
	let weapon = ["WARRIOR.arms", "WARRIOR.fury", "WARRIOR.protection", "PALADIN.retribution", "ROGUE.assassination", "ROGUE.combat", "ROGUE.subtlety", "SHAMAN.enhancement", "DRUID.feral"];
	for (name, w) in specs {
		let hunter = name.starts_with("HUNTER.");
		let melee = weapon.contains(&name.as_str());
		// Forever Leveling BiS gives a Protection Paladin no haste and no expertise.
		assert_eq!(w["haste"].as_f64().unwrap() > 0.0, name != "PALADIN.protection", "{name}: haste");
		assert_eq!(w["expertise"].as_f64().unwrap() > 0.0, melee || hunter, "{name}: expertise");
		assert_eq!(w["armorPen"].as_f64().unwrap() > 0.0, melee || hunter || name == "PALADIN.protection", "{name}: armor penetration");
		if melee && name != "WARRIOR.protection" {
			assert!(w["haste"].as_f64().unwrap() < w["crit"].as_f64().unwrap(), "{name}: haste is worth less than crit");
		}
	}
}

#[test]
fn pawn_s_haste_expertise_and_armor_penetration_map_to_the_new_keys() {
	let out = run(r#"
		local p = assert(G.Pawn.Parse('( Pawn: v1: "R": HasteRating=3, ExpertiseRating=2, ArmorPenetration=1, CritRating=4 )'))
		local w, mapped, ignored = G.Pawn.Weights(p)
		emit({ w = w, mapped = mapped, ignored = ignored })
	"#);
	assert_eq!(out[0], json!({ "w": { "haste": 30, "expertise": 20, "armorPen": 10, "crit": 56 }, "mapped": 4, "ignored": {} }), "per rating point in Pawn, per 1% in Polestar");
}
