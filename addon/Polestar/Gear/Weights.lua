-- Stat weights per class, talent spec and level band, and what each class can
-- wear.
--
-- Stat weights from Forever Leveling BiS by Thunder, used with the author's
-- permission (2026-11-13): its numbers per spec and band are in WeightBands.lua.
-- Gear.BandFor below turns a band into Polestar's stat keys with the spec's
-- main stat at 1 (Gear.MAIN_STAT) and fills in the stats Forever Leveling BiS
-- leaves out from the game's formulas: health, mana, health per 5 seconds and
-- armor penetration are Polestar's own (each band lists them in `ours`).
-- docs/gear-weights.md explains every step and gives the weights at levels 10
-- to 60. A player changes them for themselves with Adjust weights (Adjust.lua).
--
-- Units: one point of the main stat is 1 and every other weight is "worth this
-- many points of it". Percent stats (crit, hit, spellCrit, spellHit, haste,
-- expertise, armorPen, dodge, parry, block) are per 1%: an item's ratings are
-- read at the client's rating per 1% (Gear.RATINGS, Gear.lua). mp5 and hp5 are
-- per point per 5 seconds; dps is the main hand's (or two-hander's) weapon DPS,
-- ohDps the off hand's (a spec without it counts the off hand at half of dps,
-- its damage penalty), rdps the ranged slot's (bows, guns, crossbows, thrown,
-- wands); sp is spell damage of every school, spFire and the like one
-- school only, heal is healing. A +spell power item carries both sp and heal,
-- so damage specs weigh sp and healers heal.

local ADDON, ns = ...

local Gear = ns.Gear or {}
ns.Gear = Gear

local type, pairs, ipairs, max, floor = type, pairs, ipairs, math.max, math.floor

-- Stat keys a weight table may use (also the canonical item stat keys).
Gear.STAT_KEYS = {
	"str", "agi", "sta", "int", "spi", "armor", "health", "mana",
	"ap", "rap", "sp", "spArcane", "spFire", "spFrost", "spHoly", "spNature", "spShadow", "heal",
	"crit", "hit", "spellCrit", "spellHit", "mp5", "hp5", "defense", "dodge", "parry", "block", "blockValue",
	"haste", "expertise", "armorPen",
	"dps", "ohDps", "rdps",
}

-- Specs in talent order: Forever's talent columns left to right, the same as
-- the talent-tab (C_SpecializationInfo) index. `names` are the enUS column
-- names, matched first; the position is the fallback (other languages).
-- `default` is the usual levelling spec, used before any talent point is spent.
Gear.CLASSES = {
	WARRIOR = { bit = 1, default = "arms", specs = {
		{ key = "arms", label = "Arms", names = { "arms" } },
		{ key = "fury", label = "Fury", names = { "fury" } },
		{ key = "protection", label = "Protection (tank)", names = { "protection" } },
	} },
	PALADIN = { bit = 2, default = "retribution", specs = {
		{ key = "holy", label = "Holy (healer)", names = { "holy" } },
		{ key = "protection", label = "Protection (tank)", names = { "protection" } },
		{ key = "retribution", label = "Retribution", names = { "retribution" } },
	} },
	HUNTER = { bit = 4, default = "beastmastery", specs = {
		{ key = "beastmastery", label = "Beast Mastery", names = { "beastmastery" } },
		{ key = "marksmanship", label = "Marksmanship", names = { "marksmanship" } },
		{ key = "survival", label = "Survival", names = { "survival" } },
	} },
	ROGUE = { bit = 8, default = "combat", specs = {
		{ key = "assassination", label = "Assassination", names = { "assassination" } },
		{ key = "combat", label = "Combat", names = { "combat" } },
		{ key = "subtlety", label = "Subtlety", names = { "subtlety" } },
	} },
	PRIEST = { bit = 16, default = "shadow", specs = {
		{ key = "discipline", label = "Discipline", names = { "discipline" } },
		{ key = "holy", label = "Holy (healer)", names = { "holy" } },
		{ key = "shadow", label = "Shadow", names = { "shadow" } },
	} },
	SHAMAN = { bit = 64, default = "enhancement", specs = {
		{ key = "elemental", label = "Elemental", names = { "elemental" } },
		{ key = "enhancement", label = "Enhancement", names = { "enhancement" } },
		{ key = "restoration", label = "Restoration (healer)", names = { "restoration" } },
	} },
	MAGE = { bit = 128, default = "frost", specs = {
		{ key = "arcane", label = "Arcane", names = { "arcane" } },
		{ key = "fire", label = "Fire", names = { "fire" } },
		{ key = "frost", label = "Frost", names = { "frost" } },
	} },
	WARLOCK = { bit = 256, default = "affliction", specs = {
		{ key = "affliction", label = "Affliction", names = { "affliction" } },
		{ key = "demonology", label = "Demonology", names = { "demonology" } },
		{ key = "destruction", label = "Destruction", names = { "destruction" } },
	} },
	DRUID = { bit = 1024, default = "feral", specs = {
		{ key = "balance", label = "Balance", names = { "balance" } },
		{ key = "feral", label = "Feral", names = { "feral", "feralcombat" } },
		{ key = "restoration", label = "Restoration (healer)", names = { "restoration" } },
	} },
}

-- The stat each spec's weights hold at 1: the attribute its attack power
-- comes from for weapon specs, Stamina for tanks, spell damage for casters and
-- healing for healers. A spec with sp or heal here takes its crit and hit as
-- spell crit and spell hit; every other spec as melee and ranged crit and hit.
Gear.MAIN_STAT = {
	WARRIOR = { arms = "str", fury = "str", protection = "sta" },
	PALADIN = { holy = "heal", protection = "sta", retribution = "str" },
	HUNTER = { beastmastery = "agi", marksmanship = "agi", survival = "agi" },
	ROGUE = { assassination = "agi", combat = "agi", subtlety = "agi" },
	PRIEST = { discipline = "heal", holy = "heal", shadow = "sp" },
	SHAMAN = { elemental = "sp", enhancement = "str", restoration = "heal" },
	MAGE = { arcane = "sp", fire = "sp", frost = "sp" },
	WARLOCK = { affliction = "sp", demonology = "sp", destruction = "sp" },
	DRUID = { balance = "sp", feral = "str", restoration = "heal" },
}

-- Talents that turn one stat into another (Forever's talent tree): by class,
-- the talent's spell, its ranks and the share of `from` that counts as `into`
-- at each rank. Gear.lua adds `into`'s weight times the share for the ranks
-- bought to `from`, on the built-in weights in use (a custom scale is used as
-- it is). `source`: the band numbers already count the talent for this spec
-- from that level at that share; Gear.BandFor takes it out first, so the ranks
-- the player has count instead.
--   Mental Dexterity (Shaman, 3 ranks, the Enhancement column's second row):
--   33/66/100% of Intellect as attack power since 12 November 2026 (the
--   client's hotfix data, talent curve 85272; #2090), 50/100/150% before.
--   Forever Leveling BiS v1.2.7's numbers were made on 11 November: its
--   Enhancement Intellect counts the talent at 3/3 and 150% from the 15-19 band
--   on (5 points in the tree reach the second row at level 15).
Gear.TALENT_STATS = {
	SHAMAN = { { spell = 415140, ranks = 3, from = "int", into = "ap", shares = { 0.33, 0.66, 1 },
		source = { spec = "enhancement", level = 15, share = 1.5 } } },
}

-- Forever Leveling BiS's stat names -> Polestar's. crit and hit go to
-- spellCrit and spellHit for casters and healers (Gear.MAIN_STAT); sp and heal
-- come from its damage and healing numbers; haste and expertise from its
-- rating numbers (below).
local NAMES = {
	str = "str", agi = "agi", sta = "sta", int = "int", spi = "spi", armor = "armor",
	atkpwr = "ap", rgdatkpwr = "rap",
	arcsplpwr = "spArcane", firsplpwr = "spFire", frosplpwr = "spFrost", holsplpwr = "spHoly", natsplpwr = "spNature", shasplpwr = "spShadow",
	manargn = "mp5", dps = "dps", ohDps = "ohDps", rangedDps = "rdps",
	defrtng = "defense", dodgePct = "dodge", parryPct = "parry", blockPct = "block", block = "blockValue",
}

-- The game's own numbers the stats Forever Leveling BiS leaves out are made from:
local HEALTH_PER_STA = 10 -- gametables/hppersta.txt, every level (1.60.1.70888)
local MANA_PER_INT = 15 -- each point of Intellect past the first 20
local HP5_HEALTH = 5 -- 1 health per 5 seconds restores 5 over a 25-second fight
-- Intellect per 1% spell crit at level 60; at level L it takes
-- (0.2 + 0.8 (L - 1) / 59) of that (Classic's formula, as the community
-- TrinityCore server for Forever has it).
local INT_PER_SPELL_CRIT = { MAGE = 59.5, PRIEST = 59.2, WARLOCK = 60.6, DRUID = 60, SHAMAN = 59.5, PALADIN = 54, HUNTER = 60 }
-- A same-level mob's armor at levels 1 to 60 (the Forever TrinityCore world
-- database's Classic creature stats, melee mobs). It takes
-- armor / (armor + 400 + 85 x attacker level) off a physical hit.
local MOB_ARMOR = { 0, 0, 25, 60, 103, 156, 221, 297, 386, 492, 518, 552, 585, 619, 653, 688, 722, 755, 789, 822,
	858, 892, 925, 959, 992, 1026, 1061, 1093, 1127, 1160, 1194, 1229, 1261, 1296, 1328, 1434, 1546, 1661, 1785, 1914,
	2049, 2193, 2343, 2502, 2668, 2723, 2776, 2828, 2883, 2936, 2988, 3043, 3096, 3148, 3201, 3256, 3308, 3361, 3414, 3466 }

-- The share of a physical hit a same-level mob's armor takes at `level`.
function Gear.ArmorShare(level)
	local a = MOB_ARMOR[level] or MOB_ARMOR[#MOB_ARMOR]
	return a / (a + 400 + 85 * level)
end

-- Intellect per 1% spell crit for `class` at `level` (nil: no spell crit from Intellect).
function Gear.IntPerSpellCrit(class, level)
	local r = INT_PER_SPELL_CRIT[class]
	return r and r * (0.2 + 0.8 * (level - 1) / 59) or nil
end

-- Whether Polestar has weights for the class (and spec).
function Gear.HasWeights(class, key)
	local c = type(class) == "string" and Gear.WEIGHT_BANDS and Gear.WEIGHT_BANDS[class]
	if not c then return false end
	return key == nil or c[key] ~= nil
end

local names = {} -- a spec's name list (its first entry) -> its names, split once

local function namesOf(spec)
	local list = names[spec]
	if not list then
		list = {}
		for n in spec[1]:gmatch("%S+") do list[#list + 1] = n end
		names[spec] = list
	end
	return list
end

-- One band in Polestar's keys, with the spec's main stat at 1, for the levels
-- `from` to band[1]. -> weights, info { source, from, upTo, ours }
local function readBand(class, key, spec, band, from)
	local r = {}
	for i, n in ipairs(namesOf(spec)) do r[n] = band[i + 1] or 0 end
	local main = Gear.MAIN_STAT[class][key]
	local caster = main == "sp" or main == "heal"
	local w = {}
	for n, v in pairs(r) do
		local k = NAMES[n]
		if k and v ~= 0 then w[k] = v end
	end
	-- Spell damage and healing: "+spell power" (damage and healing) is worth
	-- the two together, so each takes its own number.
	local splpwr, spldmg, splheal = r.splpwr or 0, r.spldmg, r.splheal
	local sp = spldmg or max(0, splpwr - (splheal or 0))
	local heal = splheal or max(0, splpwr - (spldmg or 0))
	if sp > 0 then w.sp = sp end
	if heal > 0 then w.heal = heal end
	local crit, hit = r.critPct or 0, r.hitPct or 0
	if crit > 0 then w[caster and "spellCrit" or "crit"] = crit end
	if hit > 0 then w[caster and "spellHit" or "hit"] = hit end
	-- Haste and expertise come per rating point, at the rating per 1% the
	-- numbers were made with, which the crit numbers give (1% crit against
	-- crit rating); the client's table takes 14 crit, 10 haste and 10
	-- expertise rating per 1% at every level (Gear.RATING_RATE).
	local rate = Gear.RATING_RATE or {}
	local critRating = r.critstrkrtng or 0
	local per = critRating > 0 and crit / ((rate.crit or 14) * critRating) or 1
	if (r.hastertng or 0) > 0 then w.haste = r.hastertng * (rate.haste or 10) * per end
	if (r.exprtng or 0) > 0 then w.expertise = r.exprtng * (rate.expertise or 10) * per end
	-- A talent the numbers count at a share the game has changed since.
	for _, t in ipairs(Gear.TALENT_STATS[class] or {}) do
		local src = t.source
		if src and src.spec == key and from >= src.level and w[t.into] then
			w[t.from] = max(0, (w[t.from] or 0) - src.share * w[t.into])
		end
	end
	-- Polestar's own, from the game's formulas, at the band's middle level.
	local level = floor((from + band[1]) / 2 + 0.5)
	local ours = {}
	local function own(k, v)
		if v > 0 then
			w[k] = v
			ours[k] = true
		end
	end
	own("health", (w.sta or 0) / HEALTH_PER_STA)
	local perCrit = Gear.IntPerSpellCrit(class, level)
	if perCrit then own("mana", max(0, (w.int or 0) - (w.spellCrit or 0) / perCrit) / MANA_PER_INT) end
	own("hp5", (w.health or 0) * HP5_HEALTH)
	if not caster then own("armorPen", (w.crit or 0) * Gear.ArmorShare(level)) end
	local m = w[main]
	if m and m > 0 then
		for k, v in pairs(w) do w[k] = v / m end
	end
	return w, { source = "flbis", from = from, upTo = band[1], ours = ours }
end

local bandCache = setmetatable({}, { __mode = "k" }) -- band -> { weights, info }

-- The spec's weights at `level`: its first band whose highest level reaches
-- it (the last band past the end), in Polestar's keys.
-- -> weights, info { source = "flbis", from, upTo, ours = { [key] = true } } | nil
function Gear.BandFor(class, key, level)
	local spec = Gear.WEIGHT_BANDS[class] and Gear.WEIGHT_BANDS[class][key]
	if not spec or #spec < 2 then return nil end
	local i, from = #spec, 1
	for j = 2, #spec do
		if level <= spec[j][1] then
			i = j
			break
		end
		from = spec[j][1] + 1
	end
	if i == #spec and level > spec[i][1] then from = (spec[i - 1] and type(spec[i - 1]) == "table" and spec[i - 1][1] + 1) or 1 end
	local band = spec[i]
	local c = bandCache[band]
	if not c then
		local w, info = readBand(class, key, spec, band, from)
		c = { w, info }
		bandCache[band] = c
	end
	return c[1], c[2]
end

-- Armor and weapon proficiencies (Forever trainers; weapon skills a class
-- can train count as usable). Armor subclasses: 1 cloth, 2 leather, 3 mail,
-- 4 plate; `armorAt40` is the heavier type learned at level 40. Relics:
-- 7 libram, 8 idol, 9 totem. Weapon subclasses: 0/1 axe 1H/2H, 2 bow, 3 gun,
-- 4/5 mace 1H/2H, 6 polearm, 7/8 sword 1H/2H, 10 staff, 13 fist, 15 dagger,
-- 16 thrown, 18 crossbow, 19 wand. `dualWield` = the level Dual Wield is
-- trained (Spell 674 known also counts).
local function set(...)
	local t = {}
	for i = 1, select("#", ...) do t[select(i, ...)] = true end
	return t
end

Gear.PROFICIENCY = {
	WARRIOR = { armor = 3, armorAt40 = 4, shield = true, dualWield = 20, weapons = set(0, 1, 2, 3, 4, 5, 6, 7, 8, 10, 13, 15, 16, 18) },
	PALADIN = { armor = 3, armorAt40 = 4, shield = true, relic = 7, weapons = set(0, 1, 4, 5, 6, 7, 8) },
	HUNTER = { armor = 2, armorAt40 = 3, dualWield = 20, weapons = set(0, 1, 2, 3, 6, 7, 8, 10, 13, 15, 16, 18) },
	ROGUE = { armor = 2, dualWield = 10, weapons = set(0, 2, 3, 4, 7, 13, 15, 16, 18) },
	PRIEST = { armor = 1, weapons = set(4, 10, 15, 19) },
	SHAMAN = { armor = 2, armorAt40 = 3, shield = true, relic = 9, weapons = set(0, 1, 4, 5, 10, 13, 15) },
	MAGE = { armor = 1, weapons = set(7, 10, 15, 19) },
	WARLOCK = { armor = 1, weapons = set(7, 10, 15, 19) },
	DRUID = { armor = 2, relic = 8, weapons = set(4, 5, 10, 13, 15) },
}
