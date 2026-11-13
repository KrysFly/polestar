//! Gear advice's stat weights (addon/Polestar/Gear/Weights.lua, WeightBands.lua, docs/gear-weights.md): stat
//! weights from Forever Leveling BiS by Thunder, used with the author's permission (2026-11-13), read into
//! Polestar's keys by the rules docs/gear-weights.md gives, with Polestar's own weights for the stats they leave
//! out; known upgrades scoring the right way; an off hand weighed with the spec's own off-hand weight.

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

// The rules of docs/gear-weights.md ("How Polestar reads a band"), written out again here, against every band of
// every spec.
#[test]
fn every_spec_has_twelve_bands_and_each_band_is_read_by_the_documented_rules() {
	let out = run(r#"
		local NAMES = { str = "str", agi = "agi", sta = "sta", int = "int", spi = "spi", armor = "armor", atkpwr = "ap",
			rgdatkpwr = "rap", arcsplpwr = "spArcane", firsplpwr = "spFire", frosplpwr = "spFrost", holsplpwr = "spHoly",
			natsplpwr = "spNature", shasplpwr = "spShadow", manargn = "mp5", dps = "dps", ohDps = "ohDps", rangedDps = "rdps",
			defrtng = "defense", dodgePct = "dodge", parryPct = "parry", blockPct = "block", block = "blockValue" }
		local bad, specs, bands, owns = {}, 0, 0, {}
		local function no(s) bad[#bad + 1] = s end
		for class, meta in pairs(G.CLASSES) do
			for _, s in ipairs(meta.specs) do
				specs = specs + 1
				local spec = G.WEIGHT_BANDS[class][s.key]
				local tops = {}
				for i = 2, #spec do tops[#tops + 1] = spec[i][1] end
				if table.concat(tops, " ") ~= "9 14 19 24 29 34 39 44 49 54 59 60" then no(class .. "." .. s.key .. " bands " .. table.concat(tops, " ")) end
				local names = {}
				for x in spec[1]:gmatch("%S+") do names[#names + 1] = x end
				local main = G.MAIN_STAT[class][s.key]
				local caster = main == "sp" or main == "heal"
				local from = 1
				for i = 2, #spec do
					bands = bands + 1
					local band, r, want, ours = spec[i], {}, {}, {}
					for j, x in ipairs(names) do r[x] = band[j + 1] end
					if #band ~= #names + 1 then no(class .. "." .. s.key .. "@" .. band[1] .. " has " .. #band .. " numbers") end
					for x, v in pairs(r) do if NAMES[x] and v ~= 0 then want[NAMES[x]] = v end end
					local sp = r.spldmg or math.max(0, (r.splpwr or 0) - (r.splheal or 0))
					local heal = r.splheal or math.max(0, (r.splpwr or 0) - (r.spldmg or 0))
					if sp > 0 then want.sp = sp end
					if heal > 0 then want.heal = heal end
					local crit, hit = r.critPct or 0, r.hitPct or 0
					if crit > 0 then want[caster and "spellCrit" or "crit"] = crit end
					if hit > 0 then want[caster and "spellHit" or "hit"] = hit end
					local per = (r.critstrkrtng or 0) > 0 and crit / (14 * r.critstrkrtng) or 1
					if (r.hastertng or 0) > 0 then want.haste = r.hastertng * 10 * per end
					if (r.exprtng or 0) > 0 then want.expertise = r.exprtng * 10 * per end
					if class == "SHAMAN" and s.key == "enhancement" and from >= 15 then want.int = math.max(0, want.int - 1.5 * want.ap) end
					local level = math.floor((from + band[1]) / 2 + 0.5)
					local function own(k, v) if v > 0 then want[k], ours[k] = v, true end end
					own("health", (want.sta or 0) / 10)
					local perCrit = G.IntPerSpellCrit(class, level)
					if perCrit then own("mana", math.max(0, (want.int or 0) - (want.spellCrit or 0) / perCrit) / 15) end
					own("hp5", (want.health or 0) * 5)
					if not caster then own("armorPen", (want.crit or 0) * G.ArmorShare(level)) end
					local m = want[main]
					for k, v in pairs(want) do want[k] = v / m end
					for _, level in ipairs({ from, band[1] }) do
						local got, info = G.BandFor(class, s.key, level)
						local where = class .. "." .. s.key .. "@" .. level
						if got[main] ~= 1 then no(where .. " main " .. main .. " " .. tostring(got[main])) end
						if info.source ~= "flbis" or info.from ~= from or info.upTo ~= band[1] then no(where .. " info") end
						for k, v in pairs(want) do
							if not got[k] or math.abs(got[k] - v) > 1e-9 then no(where .. " " .. k .. " " .. tostring(got[k]) .. " want " .. v) end
						end
						for k in pairs(got) do if want[k] == nil then no(where .. " extra " .. k) end end
						for k in pairs(ours) do if not info.ours[k] then no(where .. " not marked ours " .. k) end end
						for k in pairs(info.ours) do if not ours[k] then no(where .. " marked ours " .. k) end owns[k] = true end
					end
					from = band[1] + 1
				end
			end
		end
		local ownList = {}
		for k in pairs(owns) do ownList[#ownList + 1] = k end
		table.sort(ownList)
		emit({ specs = specs, bands = bands, n = #bad, first = { bad[1] or false, bad[2] or false, bad[3] or false }, ours = ownList,
			past = { G.BandFor("WARRIOR", "arms", 70) == G.BandFor("WARRIOR", "arms", 60), select(2, G.BandFor("WARRIOR", "arms", 70)).from } })
	"#);
	let o = &out[0];
	assert_eq!(o["n"], 0, "first differences: {}", o["first"]);
	assert_eq!(o["specs"], 27);
	assert_eq!(o["bands"], 27 * 12);
	assert_eq!(o["ours"], json!(["armorPen", "health", "hp5", "mana"]), "Polestar's own weights, and only those, are marked");
	assert_eq!(o["past"], json!([true, 60]), "past the last band: the last band");
}

// The game's numbers behind Polestar's own weights, as docs/gear-weights.md gives them.
#[test]
fn polestar_s_own_weights_come_from_the_documented_game_numbers() {
	let out = run(r#"
		local mage1, mage60, pal30 = G.IntPerSpellCrit("MAGE", 1), G.IntPerSpellCrit("MAGE", 60), G.IntPerSpellCrit("PALADIN", 30)
		local frost = G.WeightsFor("MAGE", "frost", 60)
		local arms = G.WeightsFor("WARRIOR", "arms", 60)
		emit({ int = { mage1, mage60, pal30, G.IntPerSpellCrit("WARRIOR", 60) or false },
			armor = { G.ArmorShare(60), G.ArmorShare(12) },
			frost = { mana = frost.mana, int = frost.int, crit = frost.spellCrit, pen = frost.armorPen or false },
			arms = { health = arms.health, sta = arms.sta, hp5 = arms.hp5, pen = arms.armorPen, crit = arms.crit } })
	"#);
	let o = &out[0];
	let f = |v: &Value| v.as_f64().unwrap();
	let close = |a: f64, b: f64| (a - b).abs() < 1e-9;
	assert!(close(f(&o["int"][0]), 59.5 * 0.2) && close(f(&o["int"][1]), 59.5), "Intellect per 1% spell crit, Mage: {}", o["int"]);
	assert!(close(f(&o["int"][2]), 54.0 * (0.2 + 0.8 * 29.0 / 59.0)), "Paladin at 30: {}", o["int"]);
	assert_eq!(o["int"][3], false, "no spell crit from Intellect for a Warrior");
	assert!(close(f(&o["armor"][0]), 3466.0 / (3466.0 + 400.0 + 85.0 * 60.0)), "a level 60 mob's armor share: {}", o["armor"]);
	assert!(close(f(&o["armor"][1]), 552.0 / (552.0 + 400.0 + 85.0 * 12.0)), "a level 12 mob's armor share: {}", o["armor"]);
	let fr = &o["frost"];
	assert!(close(f(&fr["mana"]), (f(&fr["int"]) - f(&fr["crit"]) / 59.5) / 15.0), "Frost's mana: Intellect less its spell crit, over 15: {fr}");
	assert_eq!(fr["pen"], false, "spells ignore armor: no armor penetration for a caster");
	let a = &o["arms"];
	assert!(close(f(&a["health"]), f(&a["sta"]) / 10.0) && close(f(&a["hp5"]), f(&a["health"]) * 5.0), "health and health per 5 s: {a}");
	assert!(close(f(&a["pen"]), f(&a["crit"]) * 3466.0 / (3466.0 + 400.0 + 85.0 * 60.0)), "armor penetration: {a}");
}

#[test]
fn known_upgrades_score_the_right_way() {
	let out = run(r#"
		local function score(class, key, level, inv, stats, off)
			return G.ScoreWith({ inv = inv, class = 4, subclass = 0, stats = stats }, G.WeightsFor(class, key, level), off)
		end
		emit({
			-- A two-hander 5 DPS better beats one with 5 more Strength (weapon DPS about 6.4 Strength at 30).
			arms = score("WARRIOR", "arms", 30, 17, { dps = 30 }) > score("WARRIOR", "arms", 30, 17, { dps = 25, str = 5 }),
			-- Spell damage beats Intellect for a Frost Mage; Frost damage counts almost in full, Fire hardly.
			mage = { score("MAGE", "frost", 60, 5, { sp = 20 }) > score("MAGE", "frost", 60, 5, { int = 20 }),
				score("MAGE", "frost", 60, 5, { spFrost = 20 }) > score("MAGE", "frost", 60, 5, { spFire = 20 }) * 10 },
			-- Healing beats "+spell power" of fewer points for a Holy Priest, which counts as healing; spell damage alone is nothing.
			priest = { score("PRIEST", "holy", 60, 5, { heal = 22 }) > score("PRIEST", "holy", 60, 5, { sp = 12, heal = 12 }),
				score("PRIEST", "holy", 60, 5, { sp = 12, heal = 12 }) == score("PRIEST", "holy", 60, 5, { heal = 12 }),
				score("PRIEST", "holy", 60, 5, { sp = 20 }) },
			-- A tank's Stamina beats Strength; a Rogue's Agility beats Strength.
			prot = score("WARRIOR", "protection", 60, 5, { sta = 10 }) > score("WARRIOR", "protection", 60, 5, { str = 10 }),
			rogue = score("ROGUE", "combat", 40, 5, { agi = 10 }) > score("ROGUE", "combat", 40, 5, { str = 10 }),
			-- A Hunter's bow DPS outweighs melee weapon DPS many times over.
			hunter = score("HUNTER", "beastmastery", 50, 15, { dps = 5 }) > 10 * score("HUNTER", "beastmastery", 50, 17, { dps = 5 }),
			-- Feral weighs the weapon's DPS (Forever's forms hit with it), about 6 Strength per DPS at 30.
			feral = score("DRUID", "feral", 30, 17, { dps = 5 }) > score("DRUID", "feral", 30, 5, { str = 25 }),
			-- Crit is worth more at 60 than at 10, against Strength.
			crit = { G.WeightsFor("WARRIOR", "arms", 10).crit, G.WeightsFor("WARRIOR", "arms", 60).crit },
		})
	"#);
	let o = &out[0];
	assert_eq!(o["arms"], true);
	assert_eq!(o["mage"], json!([true, true]));
	assert_eq!(o["priest"], json!([true, true, 0]));
	assert_eq!(o["prot"], true);
	assert_eq!(o["rogue"], true);
	assert_eq!(o["hunter"], true);
	assert_eq!(o["feral"], true);
	assert!(o["crit"][0].as_f64().unwrap() * 5.0 < o["crit"][1].as_f64().unwrap(), "{}", o["crit"]);
}

#[test]
fn an_off_hand_weapon_counts_at_the_spec_s_own_off_hand_weight_else_half_of_the_main_hand_s() {
	let out = run(r#"
		local dagger = { inv = 13, class = 2, subclass = 15, stats = { dps = 10 } }
		local combat, arms = G.WeightsFor("ROGUE", "combat", 40), G.WeightsFor("WARRIOR", "arms", 30)
		local pawn = G.Pawn.Weights(assert(G.Pawn.Parse('( Pawn: v1: "O": OffHandDps=2, MeleeDps=3 )')))
		emit({ combat = { G.ScoreWith(dagger, combat, true), combat.ohDps, G.ScoreWith(dagger, combat), combat.dps },
			arms = { G.ScoreWith(dagger, arms, true), arms.dps, arms.ohDps or false }, pawn = pawn })
	"#);
	let o = &out[0];
	let f = |v: &Value| v.as_f64().unwrap();
	assert!((f(&o["combat"][0]) - 10.0 * f(&o["combat"][1])).abs() < 1e-9, "Combat's own off-hand weight: {}", o["combat"]);
	assert!((f(&o["combat"][2]) - 10.0 * f(&o["combat"][3])).abs() < 1e-9, "in the main hand, the main hand's: {}", o["combat"]);
	assert_eq!(o["arms"][2], false, "Arms has no off-hand weight");
	assert!((f(&o["arms"][0]) - 5.0 * f(&o["arms"][1])).abs() < 1e-9, "half of the main hand's: {}", o["arms"]);
	assert_eq!(o["pawn"], json!({ "ohDps": 2, "dps": 3 }), "Pawn's OffHandDps is the off hand's weight");
}
