use crate::hud::Hud;
use crate::registry::{Event, Kind, Registry, Role};
use mod_api::serde_json::{self, Map, Value, json};
use mod_api::{ClientHost, ConfigPath};

const AUTO: &str = "client/auto.json";
const DIR: &str = "client/configs";
const CAT: usize = 5;

struct Entry {
    module: usize,
    name_s: usize,
    load_s: usize,
    save_s: usize,
    remove_s: usize,
    name: String,
    live: bool,
}

pub struct Configs {
    pub module: usize,
    entries: Vec<Entry>,
    active: Option<String>,
    loaded: bool,
    last: String,
    ticks: u32,
}

fn file(name: &str) -> Option<ConfigPath> {
    ConfigPath::new(&format!("{DIR}/{name}.json")).ok()
}

fn clean(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_'))
        .collect::<String>()
        .trim()
        .chars()
        .take(24)
        .collect()
}

fn saved(reg: &Registry, m: usize) -> bool {
    let module = &reg.modules[m];
    !module.hidden && matches!(module.role, Role::Normal | Role::Static)
}

fn snapshot(reg: &Registry, hud: &Hud) -> Value {
    let mut root = Map::new();
    for (mi, m) in reg.modules.iter().enumerate() {
        if !saved(reg, mi) {
            continue;
        }
        let mut settings = Map::new();
        for (i, &s) in reg.flat(mi).iter().enumerate() {
            let set = &reg.settings[s];
            let v = match &set.kind {
                Kind::Bool { value } => json!(value),
                Kind::Slider { value, .. } => json!(value),
                Kind::Mode { modes, index, .. } => json!(modes.get(*index)),
                Kind::Bind { key } => json!(key),
                _ => continue,
            };
            settings.insert(format!("{i}:{}", set.name), v);
        }
        let mut entry = Map::new();
        if m.role == Role::Normal {
            entry.insert("enabled".to_owned(), json!(m.enabled));
            entry.insert("key".to_owned(), json!(m.key));
        }
        entry.insert("settings".to_owned(), Value::Object(settings));
        root.insert(m.name.clone(), Value::Object(entry));
    }
    let hud: Vec<Value> = hud.positions().iter().map(|p| json!(p)).collect();
    root.insert("__hud".to_owned(), Value::Array(hud));
    Value::Object(root)
}

fn apply(reg: &mut Registry, hud: &mut Hud, v: &Value) {
    let Some(root) = v.as_object() else {
        return;
    };
    for mi in 0..reg.modules.len() {
        if !saved(reg, mi) {
            continue;
        }
        let Some(entry) = root.get(&reg.modules[mi].name).and_then(Value::as_object) else {
            continue;
        };
        if reg.modules[mi].role == Role::Normal {
            if let Some(e) = entry.get("enabled").and_then(Value::as_bool) {
                reg.modules[mi].enabled = e;
            }
            if let Some(k) = entry.get("key") {
                reg.modules[mi].key = k.as_str().map(str::to_owned);
            }
        }
        let Some(settings) = entry.get("settings").and_then(Value::as_object) else {
            continue;
        };
        let flat = reg.flat(mi);
        for (i, &s) in flat.iter().enumerate() {
            let Some(v) = settings.get(&format!("{i}:{}", reg.settings[s].name)) else {
                continue;
            };
            match &reg.settings[s].kind {
                Kind::Bool { .. } => {
                    if let Some(b) = v.as_bool() {
                        reg.set_bool(s, b);
                    }
                }
                Kind::Slider { .. } => {
                    if let Some(f) = v.as_f64() {
                        reg.set_slider(s, f as f32);
                    }
                }
                Kind::Mode { modes, .. } => {
                    if let Some(i) = v.as_str().and_then(|m| modes.iter().position(|x| x == m)) {
                        reg.select_mode(s, i);
                    }
                }
                Kind::Bind { .. } => reg.set_bind(s, v.as_str().map(str::to_owned)),
                _ => {}
            }
        }
    }
    if let Some(list) = root.get("__hud").and_then(Value::as_array) {
        let pos: Vec<[f32; 2]> = list
            .iter()
            .filter_map(|p| {
                let a = p.as_array()?;
                Some([a.first()?.as_f64()? as f32, a.get(1)?.as_f64()? as f32])
            })
            .collect();
        hud.set_positions(&pos);
    }
}

fn read_path(host: &dyn ClientHost, p: &ConfigPath) -> Option<Value> {
    let bytes = host.config_files().read(p).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn write_path(host: &dyn ClientHost, p: &ConfigPath, v: &Value) -> bool {
    let Ok(bytes) = serde_json::to_vec_pretty(v) else {
        return false;
    };
    host.config_files().write(p, &bytes).is_ok()
}

fn delete(host: &dyn ClientHost, name: &str) -> bool {
    let path = host
        .instance_dir()
        .join("config")
        .join("client")
        .join("configs")
        .join(format!("{name}.json"));
    std::fs::remove_file(path).is_ok()
}

fn list(host: &dyn ClientHost) -> Vec<String> {
    let Ok(dir) = ConfigPath::new(DIR) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut after: Option<String> = None;
    while let Ok(page) = host.config_files().list(&dir, after.as_deref(), 256) {
        if page.is_empty() {
            break;
        }
        after = page.last().map(|e| e.name.clone());
        for e in page {
            if let Some(n) = e.name.strip_suffix(".json") {
                out.push(n.to_owned());
            }
        }
    }
    out
}

impl Configs {
    pub fn register(reg: &mut Registry) -> Configs {
        let module = reg.add_module(
            CAT,
            "New Config",
            "Creates a config from your current settings",
            false,
        );
        reg.set_role(module, Role::Action);
        Configs {
            module,
            entries: Vec::new(),
            active: None,
            loaded: false,
            last: String::new(),
            ticks: 0,
        }
    }

    fn desc(name: &str, active: bool) -> String {
        if active {
            format!("{name} is the active config")
        } else {
            "Click to switch to this config".to_owned()
        }
    }

    fn add_entry(&mut self, reg: &mut Registry, name: &str) -> usize {
        if let Some(i) = self.entries.iter().position(|e| !e.live) {
            let e = &mut self.entries[i];
            e.live = true;
            e.name = name.to_owned();
            let m = &mut reg.modules[e.module];
            m.hidden = false;
            m.enabled = false;
            m.name = name.to_owned();
            m.desc = Self::desc(name, false);
            reg.set_text(e.name_s, name);
            return i;
        }
        let module = reg.add_module(CAT, name, &Self::desc(name, false), false);
        reg.set_role(module, Role::Choice);
        let name_s = reg.text("Name", name);
        let load_s = reg.button("Load", None);
        let save_s = reg.button("Save", None);
        let remove_s = reg.button("Remove", None);
        reg.attach(module, &[name_s, load_s, save_s, remove_s]);
        self.entries.push(Entry {
            module,
            name_s,
            load_s,
            save_s,
            remove_s,
            name: name.to_owned(),
            live: true,
        });
        self.entries.len() - 1
    }

    fn mark_active(&mut self, reg: &mut Registry, which: Option<usize>) {
        self.active = which.map(|i| self.entries[i].name.clone());
        for (i, e) in self.entries.iter().enumerate() {
            let on = Some(i) == which && e.live;
            reg.modules[e.module].enabled = on;
            reg.modules[e.module].desc = Self::desc(&e.name, on);
        }
    }

    fn find_name(&self, name: &str) -> Option<usize> {
        self.entries
            .iter()
            .position(|e| e.live && e.name.eq_ignore_ascii_case(name))
    }

    fn unique(&self, base: &str) -> String {
        if self.find_name(base).is_none() {
            return base.to_owned();
        }
        (2..)
            .map(|n| format!("{base} {n}"))
            .find(|n| self.find_name(n).is_none())
            .unwrap_or_else(|| base.to_owned())
    }

    fn load(&mut self, reg: &mut Registry, hud: &mut Hud, host: &dyn ClientHost, i: usize) {
        let Some(p) = file(&self.entries[i].name) else {
            return;
        };
        if let Some(v) = read_path(host, &p) {
            apply(reg, hud, &v);
        }
        self.mark_active(reg, Some(i));
    }

    fn save(&self, reg: &Registry, hud: &Hud, host: &dyn ClientHost, i: usize) {
        if let Some(p) = file(&self.entries[i].name) {
            write_path(host, &p, &snapshot(reg, hud));
        }
    }

    fn rename(&mut self, reg: &mut Registry, host: &dyn ClientHost, i: usize) {
        let old = self.entries[i].name.clone();
        let wanted = clean(reg.get_text(self.entries[i].name_s));
        let taken = self.find_name(&wanted).is_some_and(|j| j != i);
        if wanted.is_empty() || taken || wanted == old {
            reg.set_text(self.entries[i].name_s, &old);
            return;
        }
        let (Some(from), Some(to)) = (file(&old), file(&wanted)) else {
            reg.set_text(self.entries[i].name_s, &old);
            return;
        };
        if let Some(v) = read_path(host, &from)
            && !write_path(host, &to, &v)
        {
            reg.set_text(self.entries[i].name_s, &old);
            return;
        }
        delete(host, &old);
        let e = &mut self.entries[i];
        e.name = wanted.clone();
        reg.set_text(e.name_s, &wanted);
        reg.modules[e.module].name = wanted.clone();
        let on = reg.modules[e.module].enabled;
        reg.modules[e.module].desc = Self::desc(&wanted, on);
        if self.active.as_deref() == Some(old.as_str()) {
            self.active = Some(wanted);
        }
    }

    fn remove(&mut self, reg: &mut Registry, host: &dyn ClientHost, i: usize) {
        let name = self.entries[i].name.clone();
        delete(host, &name);
        let e = &mut self.entries[i];
        e.live = false;
        reg.modules[e.module].hidden = true;
        reg.modules[e.module].enabled = false;
        if self.active.as_deref() == Some(name.as_str()) {
            self.active = None;
        }
    }

    pub fn tick(&mut self, reg: &mut Registry, hud: &mut Hud, host: &dyn ClientHost) {
        if host.local_player().is_none() {
            reg.events.clear();
            return;
        }
        if !self.loaded {
            self.loaded = true;
            let auto = ConfigPath::new(AUTO).ok().and_then(|p| read_path(host, &p));
            if let Some(v) = &auto {
                apply(reg, hud, v);
            }
            let mut names = list(host);
            names.sort_by_key(|n| n.to_lowercase());
            for n in names {
                self.add_entry(reg, &n);
            }
            let active = auto
                .as_ref()
                .and_then(|v| v.get("__active"))
                .and_then(Value::as_str)
                .and_then(|n| self.find_name(n));
            self.mark_active(reg, active);
            self.last = self.auto_value(reg, hud).to_string();
            reg.events.clear();
            return;
        }
        for ev in std::mem::take(&mut reg.events) {
            match ev {
                Event::Click(m) if m == self.module => {
                    let name = self.unique("Config");
                    let i = self.add_entry(reg, &name);
                    self.save(reg, hud, host, i);
                    self.mark_active(reg, Some(i));
                }
                Event::Click(m) => {
                    if let Some(i) = self.entries.iter().position(|e| e.live && e.module == m) {
                        self.load(reg, hud, host, i);
                    }
                }
                Event::Press(s) => {
                    let Some(i) = self
                        .entries
                        .iter()
                        .position(|e| e.live && [e.load_s, e.save_s, e.remove_s].contains(&s))
                    else {
                        continue;
                    };
                    let e = &self.entries[i];
                    if s == e.load_s {
                        self.load(reg, hud, host, i);
                    } else if s == e.save_s {
                        self.save(reg, hud, host, i);
                    } else {
                        self.remove(reg, host, i);
                    }
                }
                Event::TextDone(s) => {
                    if let Some(i) = self.entries.iter().position(|e| e.live && e.name_s == s) {
                        self.rename(reg, host, i);
                    }
                }
            }
        }
        self.ticks += 1;
        if self.ticks.is_multiple_of(40) {
            self.autosave(reg, hud, host, self.ticks.is_multiple_of(1200));
        }
    }

    fn auto_value(&self, reg: &Registry, hud: &Hud) -> Value {
        let mut v = snapshot(reg, hud);
        if let (Some(obj), Some(a)) = (v.as_object_mut(), &self.active) {
            obj.insert("__active".to_owned(), json!(a));
        }
        v
    }

    pub fn autosave(&mut self, reg: &Registry, hud: &Hud, host: &dyn ClientHost, force: bool) {
        if !self.loaded || host.local_player().is_none() {
            return;
        }
        let v = self.auto_value(reg, hud);
        let s = v.to_string();
        if (force || s != self.last)
            && let Ok(p) = ConfigPath::new(AUTO)
            && write_path(host, &p, &v)
        {
            self.last = s;
        }
    }
}
