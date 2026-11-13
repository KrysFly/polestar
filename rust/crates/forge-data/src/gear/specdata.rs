//! addon/Polestar/Gear/SpecData.lua: racial gear traits. The stat weights are in
//! addon/Polestar/Gear/WeightBands.lua (stat weights from Forever Leveling BiS by Thunder, used with
//! the author's permission), which this build does not write.

use forge_guide::js::{json_quote, num_str, str_to_num, Arr, Obj, V};

/// FLB Racials.lua (by race ID) -> Gear.RACIALS.
pub fn racials(flb_racials: &Obj) -> Obj {
	let out = Obj::new();
	for (id, r) in flb_racials.entries() {
		let rec = Obj::new();
		let wc = r.get("weaponCrit");
		if wc.truthy() {
			let w = Obj::new();
			w.set("pct", wc.get("pct"));
			let mut subs: Vec<f64> = wc.get("subclasses").keys().iter().map(|k| str_to_num(k)).collect();
			subs.sort_by(|a, b| (a - b).partial_cmp(&0.0).unwrap_or(std::cmp::Ordering::Equal));
			w.set("subclasses", V::Arr(Arr::from_vec(subs.into_iter().map(V::Num).collect())));
			w.set("name", wc.get("name"));
			rec.set("weaponCrit", V::Obj(w));
		}
		let sm = r.get("statMult");
		if sm.truthy() {
			rec.set("statMult", sm);
		}
		out.set_str(id, V::Obj(rec));
	}
	out
}

fn lua(v: &V) -> String {
	match v {
		V::Null | V::Undef => "nil".into(),
		V::Num(n) => num_str(*n),
		V::Bool(b) => b.to_string(),
		V::Str(s) => {
			let mut q = String::new();
			json_quote(&mut q, s);
			q.replace("\\u00b7", "\\194\\183")
		}
		V::Arr(a) => format!("{{ {} }}", a.to_vec().iter().map(lua).collect::<Vec<_>>().join(", ")),
		V::Obj(o) => {
			let parts: Vec<String> = o
				.entries()
				.iter()
				.map(|(k, v)| {
					let ident = k.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_') && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
					let key = if ident {
						k.to_string()
					} else if !k.is_empty() && k.bytes().all(|b| b.is_ascii_digit()) {
						format!("[{k}]")
					} else {
						let mut q = String::new();
						json_quote(&mut q, k);
						format!("[{q}]")
					};
					format!("{key} = {}", lua(v))
				})
				.collect();
			format!("{{ {} }}", parts.join(", "))
		}
	}
}

/// SpecData.lua's text: the racial gear traits.
pub fn emit_spec_data(rac: &Obj, flb_version: &V) -> String {
	let mut l: Vec<String> = vec![
		forge_guide::emit::header("lua", "forge data gear", "ForeverLevelingBiS"),
		format!("-- Racial gear traits: ForeverLevelingBiS {} by Thunder (MIT), Racials.lua.", flb_version.to_js_string()),
		"-- The stat weights are in WeightBands.lua. See THIRD_PARTY.md.".into(),
		String::new(),
		"local ADDON, ns = ...".into(),
		"local Gear = ns.Gear".into(),
		String::new(),
		"-- Racial gear traits by race ID: weaponCrit (percent crit with these weapon subclasses), statMult.".into(),
		"Gear.RACIALS = {".into(),
	];
	let mut ids = rac.keys();
	ids.sort_by(|a, b| (str_to_num(a) - str_to_num(b)).partial_cmp(&0.0).unwrap_or(std::cmp::Ordering::Equal));
	for id in ids {
		let r = rac.get(&id);
		let mut parts = Vec::new();
		let wc = r.get("weaponCrit");
		if wc.truthy() {
			let subs = wc.get("subclasses").items().iter().map(|s| format!("[{}] = true", s.to_js_string())).collect::<Vec<_>>().join(", ");
			parts.push(format!("weaponCrit = {{ pct = {}, subclasses = {{ {subs} }}, name = {} }}", wc.get("pct").to_js_string(), lua(&wc.get("name"))));
		}
		let sm = r.get("statMult");
		if sm.truthy() {
			parts.push(format!("statMult = {}", lua(&sm)));
		}
		l.push(format!("\t[{id}] = {{ {} }},", parts.join(", ")));
	}
	l.push("}".into());
	l.push(String::new());
	l.join("\n")
}
