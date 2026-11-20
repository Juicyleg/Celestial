#![allow(dead_code)]

pub type Visibility = Box<dyn Fn(&Registry) -> bool + Send + Sync>;
pub type Action = Box<dyn Fn(&mut Registry) + Send + Sync>;

pub enum Kind {
    Section,
    Bool {
        value: bool,
    },
    Mode {
        modes: Vec<String>,
        index: usize,
        open: bool,
    },
    Slider {
        min: f32,
        max: f32,
        step: f32,
        value: f32,
        suffix: String,
    },
    Group {
        children: Vec<usize>,
        open: bool,
    },
    Button {
        action: Option<Action>,
    },
    Bind {
        key: Option<String>,
    },
    Text {
        value: String,
    },
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Normal,
    Static,
    Action,
    Choice,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Click(usize),
    Press(usize),
    TextDone(usize),
}

pub struct Setting {
    pub name: String,
    pub kind: Kind,
    pub visible: Option<Visibility>,
}

pub struct Module {
    pub name: String,
    pub desc: String,
    pub category: usize,
    pub enabled: bool,
    pub key: Option<String>,
    pub settings: Vec<usize>,
    pub role: Role,
    pub hidden: bool,
}

impl Module {
    pub fn has_settings(&self) -> bool {
        !self.settings.is_empty()
    }

    pub fn bindable(&self) -> bool {
        self.role == Role::Normal
    }
}

#[derive(Default)]
pub struct Registry {
    pub modules: Vec<Module>,
    pub settings: Vec<Setting>,
    pub events: Vec<Event>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_module(&mut self, category: usize, name: &str, desc: &str, enabled: bool) -> usize {
        self.modules.push(Module {
            name: name.to_owned(),
            desc: desc.to_owned(),
            category,
            enabled,
            key: None,
            settings: Vec::new(),
            role: Role::Normal,
            hidden: false,
        });
        self.modules.len() - 1
    }

    fn push(&mut self, name: &str, kind: Kind) -> usize {
        self.settings.push(Setting {
            name: name.to_owned(),
            kind,
            visible: None,
        });
        self.settings.len() - 1
    }

    pub fn section(&mut self, name: &str) -> usize {
        self.push(name, Kind::Section)
    }

    pub fn boolean(&mut self, name: &str, value: bool) -> usize {
        self.push(name, Kind::Bool { value })
    }

    pub fn mode(&mut self, name: &str, index: usize, modes: &[&str]) -> usize {
        self.push(
            name,
            Kind::Mode {
                modes: modes.iter().map(|m| (*m).to_owned()).collect(),
                index,
                open: false,
            },
        )
    }

    pub fn slider(
        &mut self,
        name: &str,
        value: f32,
        min: f32,
        max: f32,
        step: f32,
        suffix: &str,
    ) -> usize {
        self.push(
            name,
            Kind::Slider {
                min,
                max,
                step,
                value,
                suffix: suffix.to_owned(),
            },
        )
    }

    pub fn group(&mut self, name: &str, children: &[usize]) -> usize {
        self.push(
            name,
            Kind::Group {
                children: children.to_vec(),
                open: true,
            },
        )
    }

    pub fn button(&mut self, name: &str, action: Option<Action>) -> usize {
        self.push(name, Kind::Button { action })
    }

    pub fn set_role(&mut self, module: usize, role: Role) {
        self.modules[module].role = role;
        if role == Role::Static {
            self.modules[module].enabled = true;
        }
    }

    pub fn text(&mut self, name: &str, value: &str) -> usize {
        self.push(
            name,
            Kind::Text {
                value: value.to_owned(),
            },
        )
    }

    pub fn get_text(&self, s: usize) -> &str {
        match &self.settings[s].kind {
            Kind::Text { value } => value,
            _ => "",
        }
    }

    pub fn set_text(&mut self, s: usize, v: &str) {
        if let Kind::Text { value } = &mut self.settings[s].kind {
            *value = v.to_owned();
        }
    }

    pub fn bind(&mut self, name: &str, key: Option<&str>) -> usize {
        self.push(
            name,
            Kind::Bind {
                key: key.map(str::to_owned),
            },
        )
    }

    pub fn multi(&mut self, name: &str, items: &[(&str, bool)]) -> usize {
        let children: Vec<usize> = items.iter().map(|(n, v)| self.boolean(n, *v)).collect();
        self.group(name, &children)
    }

    pub fn show_if(&mut self, setting: usize, cond: Visibility) {
        self.settings[setting].visible = Some(cond);
    }

    pub fn attach(&mut self, module: usize, settings: &[usize]) {
        self.modules[module].settings.extend_from_slice(settings);
    }

    pub fn is_visible(&self, s: usize) -> bool {
        let setting = &self.settings[s];
        let own = setting.visible.as_ref().is_none_or(|f| f(self));
        if !own {
            return false;
        }
        if let Kind::Group { children, .. } = &setting.kind {
            return children.is_empty() || children.iter().any(|&c| self.is_visible(c));
        }
        true
    }

    pub fn rows(&self, module: usize) -> Vec<usize> {
        let mut out = Vec::new();
        self.collect(&self.modules[module].settings, &mut out);
        out
    }

    fn collect(&self, list: &[usize], out: &mut Vec<usize>) {
        for &s in list {
            if !self.is_visible(s) {
                continue;
            }
            out.push(s);
            if let Kind::Group {
                children,
                open: true,
            } = &self.settings[s].kind
            {
                self.collect(children, out);
            }
        }
    }

    pub fn flat(&self, module: usize) -> Vec<usize> {
        let mut out = Vec::new();
        self.flatten(&self.modules[module].settings, &mut out);
        out
    }

    fn flatten(&self, list: &[usize], out: &mut Vec<usize>) {
        for &s in list {
            out.push(s);
            if let Kind::Group { children, .. } = &self.settings[s].kind {
                self.flatten(children, out);
            }
        }
    }

    pub fn set_bool(&mut self, s: usize, v: bool) {
        if let Kind::Bool { value } = &mut self.settings[s].kind {
            *value = v;
        }
    }

    pub fn set_slider(&mut self, s: usize, v: f32) {
        if let Kind::Slider {
            min,
            max,
            step,
            value,
            ..
        } = &mut self.settings[s].kind
        {
            let mut v = ((v - *min) / *step).round() * *step + *min;
            v = v.clamp(*min, *max);
            *value = v;
        }
    }

    pub fn select_mode(&mut self, s: usize, i: usize) {
        if let Kind::Mode { modes, index, .. } = &mut self.settings[s].kind {
            *index = i.min(modes.len().saturating_sub(1));
        }
    }

    pub fn press(&mut self, s: usize) {
        let action = match &mut self.settings[s].kind {
            Kind::Button { action } => action.take(),
            _ => None,
        };
        self.events.push(Event::Press(s));
        if let Some(a) = action {
            a(self);
            if let Kind::Button { action } = &mut self.settings[s].kind {
                *action = Some(a);
            }
        }
    }

    pub fn get_bool(&self, s: usize) -> bool {
        matches!(self.settings[s].kind, Kind::Bool { value: true })
    }

    pub fn get_mode(&self, s: usize) -> usize {
        match &self.settings[s].kind {
            Kind::Mode { index, .. } => *index,
            _ => 0,
        }
    }

    pub fn get_slider(&self, s: usize) -> f32 {
        match &self.settings[s].kind {
            Kind::Slider { value, .. } => *value,
            _ => 0.0,
        }
    }

    pub fn get_bind(&self, s: usize) -> Option<&str> {
        match &self.settings[s].kind {
            Kind::Bind { key } => key.as_deref(),
            _ => None,
        }
    }

    pub fn set_bind(&mut self, s: usize, k: Option<String>) {
        if let Kind::Bind { key } = &mut self.settings[s].kind {
            *key = k;
        }
    }

    pub fn child(&self, group: usize, i: usize) -> usize {
        match &self.settings[group].kind {
            Kind::Group { children, .. } => children[i],
            _ => group,
        }
    }

    pub fn slider_frac(&self, s: usize) -> f32 {
        match &self.settings[s].kind {
            Kind::Slider {
                min, max, value, ..
            } => (value - min) / (max - min),
            _ => 0.0,
        }
    }

    pub fn slider_text(&self, s: usize) -> String {
        match &self.settings[s].kind {
            Kind::Slider {
                step,
                value,
                suffix,
                ..
            } => {
                let n = if *step >= 1.0 {
                    format!("{}", value.round() as i64)
                } else if *step >= 0.1 {
                    format!("{value:.1}")
                } else {
                    format!("{value:.2}")
                };
                format!("{n}{suffix}")
            }
            _ => String::new(),
        }
    }
}
