use crate::paint::{
    Affine, Canvas, ICON_ATOM, ICON_CLOSE, ICON_COMBAT, ICON_CONFIGS,
    ICON_GEAR, ICON_MOVEMENT, ICON_PLAYER, ICON_POWER, ICON_RENDER, ICON_SEARCH,
    ICON_THEMES, ICON_UTIL, Icon, Painter, Theme, mix, text_width,
};
use crate::registry::{Event, Kind, Registry, Role};
use std::collections::HashMap;
use std::f32::consts::PI;
use std::time::Instant;

const W: f32 = 463.0;
const H: f32 = 383.0;
const SW: f32 = 130.0;
const CARD_W: f32 = 148.0;
const CARD_H: f32 = 40.0;
const CARD_PITCH: f32 = 47.0;
const COL1: f32 = 138.0;
const COL2: f32 = 296.0;
const CLIP_T: f32 = 44.0;
const CLIP_B: f32 = H - 6.0;
const PANEL_W: f32 = 194.0;
const PANEL_MAX_H: f32 = 258.0;
const HEAD_H: f32 = 20.0;
const LABEL_W: f32 = 128.0;
const OPT_H: f32 = 16.0;
const SLIDER_H: f32 = 33.0;
const PANEL_BG: u32 = 0xFF0A0A0C;
const TOG_OFF: u32 = 0xFF78989F;
const KNOB: u32 = 0xFFFBFFFF;
const THUMB: u32 = 0xFF6E858D;
const WIN: u32 = 0xFF0A0A0C;
const WIN_BORDER: u32 = 0xFF202026;
const SEP: u32 = 0xFF1B1B20;
const WHITE: u32 = 0xFFFFFFFF;
const DIM: u32 = 0xFF8E8E98;
const PURPLE_TXT: u32 = 0xFFA582EC;
const CARD_OFF: u32 = 0xFF0D0D0F;
const CARD_OFF_BORDER: u32 = 0xFF2C2C32;
const OPEN_TIME: f32 = 0.6;
const CLOSE_TIME: f32 = 0.22;
const PANEL_OPEN: f32 = 0.45;
const PANEL_CLOSE: f32 = 0.2;
const BOUNCE: f32 = 2.2;
const MAX_QUERY: usize = 24;

struct Nav {
    label: &'static str,
    icon: Icon,
    cat: usize,
    cy: f32,
}

const NAV: [Nav; 7] = [
    Nav {
        label: "Combat",
        icon: ICON_COMBAT,
        cat: 0,
        cy: 81.0,
    },
    Nav {
        label: "Movement",
        icon: ICON_MOVEMENT,
        cat: 1,
        cy: 112.5,
    },
    Nav {
        label: "Render",
        icon: ICON_RENDER,
        cat: 2,
        cy: 144.0,
    },
    Nav {
        label: "Player",
        icon: ICON_PLAYER,
        cat: 3,
        cy: 175.5,
    },
    Nav {
        label: "Util",
        icon: ICON_UTIL,
        cat: 4,
        cy: 207.0,
    },
    Nav {
        label: "Configs",
        icon: ICON_CONFIGS,
        cat: 5,
        cy: 258.0,
    },
    Nav {
        label: "Themes",
        icon: ICON_THEMES,
        cat: 6,
        cy: 290.0,
    },
];

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Key {
    Nav(usize),
    Module(usize),
    Setting(usize),
    Gui,
}

const EN: u8 = 0;
const HOV: u8 = 1;
const TOG: u8 = 2;
const DD: u8 = 3;
const SEL: u8 = 4;
const SL: u8 = 5;
const POP: u8 = 6;
const NAVK: u8 = 7;
const SIZE: f32 = 0.82;

#[derive(Clone, Copy)]
enum Act {
    SelectCat(usize),
    Close,
    FocusSearch,
    ToggleModule(usize),
    OpenPanel(usize),
    Bind(usize),
    BindSetting(usize),
    ClearBind(usize),
    Front(u32),
    DragPanel(u32),
    ClosePanel(u32),
    ToggleGroup(usize),
    PressButton(usize),
    ToggleBool(usize),
    SliderStart { setting: usize, x: f32, w: f32 },
    ModeToggle { module: usize, setting: usize },
    ModeSelect { setting: usize, index: usize },
    ClickModule(usize),
    EditText(usize),
}

struct Hit {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    left: Option<Act>,
    right: Option<Act>,
    mid: Option<Act>,
}

struct Panel {
    id: u32,
    module: usize,
    x: f32,
    y: f32,
    scroll: f32,
    scroll_t: f32,
    max_scroll: f32,
    t: f32,
    closing: bool,
}

#[derive(Clone, Copy)]
struct Popup {
    setting: usize,
    bx: f32,
    by: f32,
    bw: f32,
    bh: f32,
}

pub struct Frame {
    pub size: [f32; 2],
    pub mouse: [f32; 2],
    pub gui_scale: f32,
    pub dt: Option<f32>,
    pub millis: u64,
}

pub enum KeyResult {
    Ignored,
    Consumed,
}

pub struct Gui {
    hits: Vec<Hit>,
    panels: Vec<Panel>,
    next_panel: u32,
    anim: HashMap<(Key, u8), f32>,
    cat: usize,
    scroll: f32,
    scroll_t: f32,
    max_scroll: f32,
    query: String,
    search_focus: bool,
    drag: Option<u32>,
    drag_slider: Option<(usize, f32, f32)>,
    drag_x: f32,
    drag_y: f32,
    ox: f32,
    oy: f32,
    s: f32,
    mlx: f32,
    mly: f32,
    last: Option<Instant>,
    dt: f32,
    time: f32,
    open_t: f32,
    ga: f32,
    closing: bool,
    covered: bool,
    binding: Option<usize>,
    bind_setting: Option<usize>,
    text_edit: Option<usize>,
    dd_keep: bool,
    cx: f32,
    cy: f32,
    popup: Option<Popup>,
    flow_time: f64,
}

fn clamp01(v: f32) -> f32 {
    v.clamp(0.0, 1.0)
}

fn ease_out_back(t: f32, s: f32) -> f32 {
    let t = clamp01(t);
    let u = t - 1.0;
    1.0 + (s + 1.0) * u * u * u + s * u * u
}

fn ease_in_out(t: f32) -> f32 {
    let t = clamp01(t);
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

fn ty(center: f32, size: f32) -> f32 {
    center - size * 0.58
}

fn alpha_byte(k: f32, base: u32) -> u32 {
    (((base as f32) * k) as i32).clamp(0, 255) as u32
}

fn wrap(s: &str, size: f32, max_w: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut cur = String::new();
    for word in s.split(' ') {
        let t = if cur.is_empty() {
            word.to_owned()
        } else {
            format!("{cur} {word}")
        };
        if text_width(&t, size) > max_w && !cur.is_empty() {
            lines.push(std::mem::replace(&mut cur, word.to_owned()));
        } else {
            cur = t;
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}

pub fn mouse_key(button: i32) -> String {
    match button {
        0 => "key.mouse.left".to_owned(),
        1 => "key.mouse.right".to_owned(),
        2 => "key.mouse.middle".to_owned(),
        n => format!("key.mouse.{}", n + 1),
    }
}

pub fn key_label(key: &str) -> String {
    let (prefix, rest) = if let Some(r) = key.strip_prefix("key.keyboard.") {
        ("", r)
    } else if let Some(r) = key.strip_prefix("key.mouse.") {
        ("Mouse ", r)
    } else {
        ("", key)
    };
    let words: Vec<String> = rest
        .split('.')
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        })
        .collect();
    format!("{prefix}{}", words.join(" "))
}

fn row_h(reg: &Registry, s: usize) -> f32 {
    let setting = &reg.settings[s];
    match &setting.kind {
        Kind::Section | Kind::Group { .. } => 23.0,
        Kind::Button { .. } | Kind::Bind { .. } | Kind::Text { .. } => 25.5,
        Kind::Bool { .. } => 25.5 + (wrap(&setting.name, 9.5, LABEL_W).len() as f32 - 1.0) * 11.5,
        Kind::Slider { .. } => SLIDER_H,
        Kind::Mode { .. } => 28.0,
    }
}

fn content_h(reg: &Registry, module: usize) -> f32 {
    reg.rows(module).iter().map(|&s| row_h(reg, s)).sum()
}

fn panel_height(reg: &Registry, module: usize) -> f32 {
    (HEAD_H + content_h(reg, module) + 4.0).min(PANEL_MAX_H)
}

impl Default for Gui {
    fn default() -> Self {
        Gui {
            hits: Vec::new(),
            panels: Vec::new(),
            next_panel: 0,
            anim: HashMap::new(),
            cat: 0,
            scroll: 0.0,
            scroll_t: 0.0,
            max_scroll: 0.0,
            query: String::new(),
            search_focus: false,
            drag: None,
            drag_slider: None,
            drag_x: 0.0,
            drag_y: 0.0,
            ox: 0.0,
            oy: 0.0,
            s: 1.0,
            mlx: 0.0,
            mly: 0.0,
            last: None,
            dt: 0.0,
            time: 0.0,
            open_t: 0.0,
            ga: 1.0,
            closing: false,
            covered: false,
            binding: None,
            bind_setting: None,
            text_edit: None,
            dd_keep: false,
            cx: 0.0,
            cy: 0.0,
            popup: None,
            flow_time: 0.0,
        }
    }
}

impl Gui {
    pub fn reopen(&mut self) {
        let flow_time = self.flow_time;
        let cat = self.cat;
        *self = Gui::default();
        self.flow_time = flow_time;
        self.cat = cat;
    }

    pub fn is_closing(&self) -> bool {
        self.closing
    }

    pub fn finished_closing(&self) -> bool {
        self.closing && self.open_t <= 0.0
    }

    pub fn close(&mut self) {
        self.closing = true;
        self.drag = None;
        self.drag_slider = None;
    }

    fn lerp_to(&self, cur: f32, target: f32, speed: f32) -> f32 {
        let v = cur + (target - cur) * (1.0 - (-speed * self.dt).exp());
        if (target - v).abs() < 0.0008 {
            target
        } else {
            v
        }
    }

    fn ap(&mut self, k: Key, kind: u8, target: f32, speed: f32) -> f32 {
        let cur = self.anim.get(&(k, kind)).copied().unwrap_or(target);
        let v = self.lerp_to(cur, target, speed);
        self.anim.insert((k, kind), v);
        v
    }

    fn lin(&mut self, k: Key, kind: u8, target: f32, dur: f32) -> f32 {
        let v = self.anim.get(&(k, kind)).copied().unwrap_or(target);
        let step = self.dt / dur;
        let v = if v < target {
            (v + step).min(target)
        } else {
            (v - step).max(target)
        };
        self.anim.insert((k, kind), v);
        v
    }

    fn peek(&self, k: Key, kind: u8, def: f32) -> f32 {
        self.anim.get(&(k, kind)).copied().unwrap_or(def)
    }

    fn hov(&self, x: f32, y: f32, w: f32, h: f32) -> bool {
        !self.covered
            && !self.closing
            && self.mlx >= x
            && self.mlx < x + w
            && self.mly >= y
            && self.mly < y + h
    }

    fn hov_clip(&self, x: f32, y: f32, w: f32, h: f32, vt: f32, vb: f32) -> bool {
        let y0 = y.max(vt);
        let y1 = (y + h).min(vb);
        y1 > y0 && self.hov(x, y0, w, y1 - y0)
    }

    fn hit(&mut self, x: f32, y: f32, w: f32, h: f32, left: Option<Act>, right: Option<Act>) {
        if !self.closing {
            self.hits.push(Hit {
                x,
                y,
                w,
                h,
                left,
                right,
                mid: None,
            });
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn hit_clip(&mut self, x: f32, y: f32, w: f32, h: f32, vt: f32, vb: f32, left: Option<Act>) {
        let y0 = y.max(vt);
        let y1 = (y + h).min(vb);
        if y1 > y0 {
            self.hit(x, y0, w, y1 - y0, left, None);
        }
    }

    fn update_cover(&mut self, reg: &Registry, idx: Option<usize>) {
        self.covered = false;
        let start = idx.map_or(0, |i| i + 1);
        for p in self.panels.iter().skip(start) {
            if !p.closing
                && self.mlx >= p.x
                && self.mlx < p.x + PANEL_W
                && self.mly >= p.y
                && self.mly < p.y + panel_height(reg, p.module)
            {
                self.covered = true;
                return;
            }
        }
    }

    fn visible(&self, reg: &Registry) -> Vec<usize> {
        let q = self.query.to_lowercase();
        reg.modules
            .iter()
            .enumerate()
            .filter(|(_, m)| {
                if m.hidden {
                    return false;
                }
                if q.is_empty() {
                    m.category == self.cat
                } else {
                    m.name.to_lowercase().contains(&q)
                }
            })
            .map(|(i, _)| i)
            .collect()
    }

    fn open_panel(&mut self, module: usize) {
        if let Some(i) = self.panels.iter().position(|p| p.module == module) {
            self.panels[i].closing = false;
            let id = self.panels[i].id;
            self.front(id);
            return;
        }
        let n = (self.panels.len() % 4) as f32;
        let id = self.next_panel;
        self.next_panel += 1;
        self.panels.push(Panel {
            id,
            module,
            x: 188.0 + 14.0 * n,
            y: 27.0 + 22.0 * n,
            scroll: 0.0,
            scroll_t: 0.0,
            max_scroll: 0.0,
            t: 0.0,
            closing: false,
        });
    }

    fn front(&mut self, id: u32) {
        if let Some(i) = self.panels.iter().position(|p| p.id == id) {
            let p = self.panels.remove(i);
            self.panels.push(p);
        }
    }

    fn select_cat(&mut self, target: usize) {
        if self.cat == target && self.query.is_empty() {
            return;
        }
        self.cat = target;
        self.query.clear();
        self.scroll = 0.0;
        self.scroll_t = 0.0;
    }

    pub fn render(
        &mut self,
        canvas: &mut dyn Canvas,
        reg: &mut Registry,
        theme: &Theme,
        frame: &Frame,
    ) {
        self.hits.clear();
        self.panels.retain(|pn| !reg.modules[pn.module].hidden);
        self.dt = match frame.dt {
            Some(d) => d,
            None => {
                let now = Instant::now();
                let d = match self.last {
                    Some(l) => (now - l).as_secs_f32().min(0.05),
                    None => 0.016,
                };
                self.last = Some(now);
                d
            }
        };
        self.time += self.dt;
        self.flow_time = (self.flow_time + self.dt as f64 * theme.speed as f64) % 100.0;

        let step = if self.closing {
            -self.dt / CLOSE_TIME
        } else {
            self.dt / OPEN_TIME
        };
        self.open_t = clamp01(self.open_t + step);
        let k = if self.closing {
            self.open_t * self.open_t
        } else {
            ease_out_back(self.open_t, BOUNCE)
        };
        self.ga = clamp01(self.open_t * 2.5);

        let [width, height] = frame.size;
        let mut p = Painter {
            canvas,
            m: Affine::IDENTITY,
            alpha: 1.0,
            px: 1.0 / frame.gui_scale.max(1.0),
            time: self.flow_time as f32,
            palette: theme.palette(),
            glow: theme.glow,
            rec: None,
        };
        if !theme.blur {
            let a = alpha_byte(self.ga, 0x70);
            p.canvas.quad(
                [[0.0, 0.0], [width, 0.0], [width, height], [0.0, height]],
                a << 24,
            );
        }
        if self.ga <= 0.003 {
            return;
        }

        let base = SIZE * 1.0f32.min(((height - 10.0) / H).min((width - 10.0) / W));
        self.s = base * (0.65 + 0.35 * k);
        self.ox = (width - W * self.s) / 2.0;
        self.oy = (height - H * self.s) / 2.0;
        self.mlx = (frame.mouse[0] - self.ox) / self.s;
        self.mly = (frame.mouse[1] - self.oy) / self.s;
        let main = Affine::IDENTITY.translate(self.ox, self.oy).scale(self.s);
        p.m = main;
        p.alpha = self.ga;

        self.update_cover(reg, None);
        self.draw_window(&mut p);
        self.draw_sidebar(&mut p);
        self.draw_top(&mut p, frame.millis);
        self.draw_cards(&mut p, reg);
        let ids: Vec<u32> = self.panels.iter().map(|pn| pn.id).collect();
        for id in ids {
            p.m = main;
            p.alpha = self.ga;
            self.draw_panel(&mut p, reg, id);
        }
    }

    fn draw_window(&mut self, p: &mut Painter) {
        p.fill(0.0, 0.0, W, H, 14.0, WIN);
        p.border(0.0, 0.0, W, H, 14.0, 1.0, WIN_BORDER);
        p.fill(SW, 12.0, 1.0, H - 24.0, 0.0, SEP);
        self.hit(0.0, 0.0, W, H, None, None);
    }

    #[allow(clippy::too_many_arguments)]
    fn glow(&self, p: &mut Painter, x: f32, y: f32, w: f32, h: f32, r: f32, spread: f32, a: u32) {
        let v = (a as f32 * p.alpha).round();
        if v > 0.0 {
            p.flow_glow(x, y, w, h, r, spread, v / 255.0);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn glow_k(
        &self,
        p: &mut Painter,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        r: f32,
        spread: f32,
        a: u32,
        k: f32,
    ) {
        let v = (a as f32 * p.alpha * k).round();
        if v > 0.0 {
            p.flow_glow(x, y, w, h, r, spread, v / 255.0);
        }
    }

    fn draw_sidebar(&mut self, p: &mut Painter) {
        self.glow(p, 7.0, 7.0, 34.0, 34.0, 9.0, 8.0, 0x50);
        p.flow(7.0, 7.0, 34.0, 34.0, 9.0, 1.0);
        p.icon(ICON_ATOM, 12.0, 12.0, 24.0, WHITE);
        p.text("Celestial", 48.0, ty(24.0, 15.0), 15.0, WHITE);

        p.text("Main", 10.0, ty(56.0, 8.0), 8.0, DIM);
        p.text("Other", 10.0, ty(234.0, 8.0), 8.0, DIM);

        for (i, n) in NAV.iter().enumerate() {
            let sel = n.cat == self.cat && self.query.is_empty();
            let tc = WHITE;
            let top = n.cy - 13.0;
            let t = self.lin(Key::Nav(i), NAVK, if sel { 1.0 } else { 0.0 }, 0.28);
            let over = self.hov(8.0, top, 113.0, 26.0);
            let hv = self.ap(Key::Nav(i), HOV, if over { 1.0 } else { 0.0 }, 16.0);
            if hv > 0.01 {
                p.fill(
                    8.0,
                    top,
                    113.0,
                    26.0,
                    8.0,
                    (alpha_byte(hv, 0x16) << 24) | 0xFFFFFF,
                );
            }
            let sw = 113.0 * ease_in_out(t);
            if sw >= 3.0 {
                let f = clamp01(sw / 22.0);
                self.glow_k(p, 8.0, top, sw, 26.0, 8.0, 8.0, 0x80, f);
                p.flow(8.0, top, sw, 26.0, 8.0, 1.0);
            }
            p.icon(n.icon, 13.0, n.cy - 7.0, 14.0, tc);
            p.text(n.label, 34.0, ty(n.cy, 11.0), 11.0, tc);
            self.hit(8.0, top, 113.0, 26.0, Some(Act::SelectCat(n.cat)), None);
        }

        let cy = 368.0;
        let over = self.hov(8.0, cy - 13.0, 113.0, 26.0);
        let hv = self.ap(Key::Nav(NAV.len()), HOV, if over { 1.0 } else { 0.0 }, 16.0);
        if hv > 0.01 {
            p.fill(
                8.0,
                cy - 13.0,
                113.0,
                26.0,
                8.0,
                (alpha_byte(hv, 0x16) << 24) | 0xFF5A5A,
            );
        }
        let ec = mix(DIM, 0xFFFF6B6B, hv);
        p.icon(ICON_POWER, 13.0, cy - 7.0, 14.0, ec);
        p.text("Exit", 34.0, ty(cy, 11.0), 11.0, ec);
        self.hit(8.0, cy - 13.0, 113.0, 26.0, Some(Act::Close), None);
    }

    fn draw_top(&mut self, p: &mut Painter, millis: u64) {
        let (sx, sy, sw, sh) = (138.0, 12.0, 313.0, 23.0);
        let fo = self.ap(
            Key::Gui,
            EN,
            if self.search_focus { 1.0 } else { 0.0 },
            14.0,
        );
        p.fill(sx, sy, sw, sh, 11.5, 0xFF111113);
        p.border(sx, sy, sw, sh, 11.5, 1.0, mix(0xFF2A2A30, 0xFF6C46B8, fo));
        p.icon(ICON_SEARCH, sx + 8.0, sy + 6.0, 11.0, WHITE);
        if self.query.is_empty() && !self.search_focus {
            p.text("Search", sx + 24.0, ty(sy + sh / 2.0, 9.5), 9.5, WHITE);
        } else {
            let caret = if self.search_focus && (millis / 500).is_multiple_of(2) {
                "|"
            } else {
                ""
            };
            let s = format!("{}{}", self.query, caret);
            p.text(&s, sx + 24.0, ty(sy + sh / 2.0, 9.5), 9.5, WHITE);
        }
        self.hit(sx, sy, sw, sh, Some(Act::FocusSearch), None);

    }

    fn draw_cards(&mut self, p: &mut Painter, reg: &mut Registry) {
        let list = self.visible(reg);
        let rows = list.len().div_ceil(2);
        let total = rows as f32 * CARD_PITCH + 10.0;
        let view_h = CLIP_B - CLIP_T + 12.0;
        self.max_scroll = (total - view_h).max(0.0);
        self.scroll_t = self.scroll_t.clamp(0.0, self.max_scroll);
        self.scroll = self
            .lerp_to(self.scroll, self.scroll_t, 18.0)
            .clamp(0.0, self.max_scroll);

        p.push_clip(COL1 - 12.0, CLIP_T, W - 4.0, CLIP_B);
        let base = p.alpha;
        for (i, &m) in list.iter().enumerate() {
            let x = if i % 2 == 0 { COL1 } else { COL2 };
            let y = 56.0 + (i / 2) as f32 * CARD_PITCH - self.scroll;
            if y + CARD_H < CLIP_T || y > CLIP_B {
                continue;
            }
            self.draw_card(p, reg, m, x, y);
        }
        p.alpha = base;
        p.pop_clip();

        if self.max_scroll > 0.0 {
            let track_t = CLIP_T + 6.0;
            let track_h = CLIP_B - CLIP_T - 12.0;
            let th = (track_h * view_h / total).max(30.0);
            let thumb_y = track_t + (track_h - th) * (self.scroll / self.max_scroll);
            p.fill(W - 13.0, track_t, 3.0, track_h, 1.5, 0xFF18181C);
            p.flow(W - 13.0, thumb_y, 3.0, th, 1.5, 1.0);
        }
    }

    fn draw_card(&mut self, p: &mut Painter, reg: &Registry, m: usize, x: f32, base_y: f32) {
        let module = &reg.modules[m];
        let vy = base_y.max(CLIP_T);
        let vh = (base_y + CARD_H).min(CLIP_B) - vy;
        let en = self.ap(
            Key::Module(m),
            EN,
            if module.enabled { 1.0 } else { 0.0 },
            11.0,
        );
        let over = vh > 0.0 && self.hov(x, vy, CARD_W, vh);
        let hv = self.ap(Key::Module(m), HOV, if over { 1.0 } else { 0.0 }, 16.0);
        let pop = self.ap(Key::Module(m), POP, 0.0, 9.0);
        let y = base_y;

        let saved = p.m;
        let sc = 1.0 - 0.045 * pop;
        if pop > 0.002 {
            p.m = saved.about(x + CARD_W / 2.0, y + CARD_H / 2.0, sc);
        }

        self.glow_k(p, x, y, CARD_W, CARD_H, 7.0, 9.0, 0x80, en);
        p.fill(x, y, CARD_W, CARD_H, 7.0, CARD_OFF);
        p.flow(x, y, CARD_W, CARD_H, 7.0, en);
        p.border(
            x,
            y,
            CARD_W,
            CARD_H,
            7.0,
            1.0,
            mix(mix(CARD_OFF_BORDER, 0xFF4A4A56, hv), 0x38FFFFFF, en),
        );
        if hv > 0.01 {
            p.fill(
                x,
                y,
                CARD_W,
                CARD_H,
                7.0,
                (alpha_byte(hv * (1.0 - en), 0x0C) << 24) | 0xFFFFFF,
            );
        }

        let ts = 11.5;
        p.text(&module.name, x + 8.0, y + 4.0, ts, WHITE);
        let ix = x + 8.0 + text_width(&module.name, ts) + 4.0;
        let mut gear_x = -1.0;
        if module.has_settings() {
            gear_x = ix;
            p.icon(ICON_GEAR, ix, y + 5.2, 11.0, WHITE);
        }

        if module.bindable() && (self.binding == Some(m) || module.key.is_some()) {
            let bt = if self.binding == Some(m) {
                "[...]".to_owned()
            } else {
                format!("[{}]", key_label(module.key.as_deref().unwrap_or("")))
            };
            let c = if self.binding == Some(m) {
                PURPLE_TXT
            } else {
                0xFFE9E3FA
            };
            p.text(
                &bt,
                x + CARD_W - 8.0 - text_width(&bt, 8.0),
                y + 5.5,
                8.0,
                c,
            );
        }
        let dc = mix(0xFFDADAE1, 0xFFF3EDFF, en);
        let mut ly = y + 17.0;
        for line in wrap(&module.desc, 7.5, CARD_W - 14.0) {
            if ly + 9.0 > y + CARD_H {
                break;
            }
            p.text(&line, x + 8.0, ly, 7.5, dc);
            ly += 8.8;
        }
        p.m = saved;

        if vh <= 0.0 {
            return;
        }
        let has = module.has_settings();
        if !self.closing {
            self.hits.push(Hit {
                x,
                y: vy,
                w: CARD_W,
                h: vh,
                left: match module.role {
                    Role::Normal => Some(Act::ToggleModule(m)),
                    Role::Static => has.then_some(Act::OpenPanel(m)),
                    Role::Action | Role::Choice => Some(Act::ClickModule(m)),
                },
                right: if has { Some(Act::OpenPanel(m)) } else { None },
                mid: module.bindable().then_some(Act::Bind(m)),
            });
        }
        if gear_x >= 0.0 {
            self.hit(
                gear_x - 2.0,
                base_y + 3.0,
                15.0,
                15.0,
                Some(Act::OpenPanel(m)),
                Some(Act::OpenPanel(m)),
            );
        }
    }

    fn draw_panel(&mut self, p: &mut Painter, reg: &mut Registry, id: u32) {
        let Some(idx) = self.panels.iter().position(|pn| pn.id == id) else {
            return;
        };
        {
            let dt = self.dt;
            let pn = &mut self.panels[idx];
            let step = if pn.closing {
                -dt / PANEL_CLOSE
            } else {
                dt / PANEL_OPEN
            };
            pn.t = clamp01(pn.t + step);
            if pn.closing && pn.t <= 0.0 {
                self.panels.remove(idx);
                return;
            }
        }
        let (module, closing, t, x, y) = {
            let pn = &self.panels[idx];
            (pn.module, pn.closing, pn.t, pn.x, pn.y)
        };
        let k = if closing {
            t * t
        } else {
            ease_out_back(t, BOUNCE)
        };
        let pa = clamp01(t * 2.5);

        for s in reg.flat(module) {
            if let Kind::Mode { open, .. } = reg.settings[s].kind {
                self.ap(Key::Setting(s), DD, if open { 1.0 } else { 0.0 }, 13.0);
            }
        }

        let h = panel_height(reg, module);
        let pcx = x + PANEL_W / 2.0;
        let pcy = y + h / 2.0;
        let sc = 0.65 + 0.35 * k;
        let outer = p.m;
        p.m = outer.about(pcx, pcy, sc);
        p.alpha = self.ga * pa;
        self.update_cover(reg, Some(idx));

        let vt = y + HEAD_H;
        let vb = y + h - 3.0;
        let vh = vb - vt;
        let ch = content_h(reg, module);
        let (scroll, max_scroll) = {
            let dt = self.dt;
            let pn = &mut self.panels[idx];
            pn.max_scroll = (ch - vh).max(0.0);
            pn.scroll_t = pn.scroll_t.clamp(0.0, pn.max_scroll);
            let v = pn.scroll + (pn.scroll_t - pn.scroll) * (1.0 - (-18.0 * dt).exp());
            let v = if (pn.scroll_t - v).abs() < 0.0008 {
                pn.scroll_t
            } else {
                v
            };
            pn.scroll = v.clamp(0.0, pn.max_scroll);
            (pn.scroll, pn.max_scroll)
        };
        if !closing {
            self.hit(x, y, PANEL_W, h, Some(Act::Front(id)), Some(Act::Front(id)));
        }

        self.glow(p, x, y, PANEL_W, h, 9.0, 11.0, 0x55);
        p.fill(x, y, PANEL_W, h, 9.0, PANEL_BG);
        p.border(x, y, PANEL_W, h, 9.0, 1.0, 0xFF25252B);

        p.flow(x, y, PANEL_W, HEAD_H, 8.0, 1.0);
        p.icon(ICON_GEAR, x + 8.0, y + 4.0, 12.0, WHITE);
        let name = reg.modules[module].name.clone();
        let tw = text_width(&name, 11.0);
        p.text(
            &name,
            x + (PANEL_W - tw) / 2.0,
            ty(y + HEAD_H / 2.0, 11.0),
            11.0,
            WHITE,
        );
        p.icon(ICON_CLOSE, x + PANEL_W - 18.0, y + 4.5, 11.0, WHITE);
        self.hit(x, y, PANEL_W - 24.0, HEAD_H, Some(Act::DragPanel(id)), None);
        self.hit(
            x + PANEL_W - 24.0,
            y,
            24.0,
            HEAD_H,
            Some(Act::ClosePanel(id)),
            None,
        );

        self.popup = None;
        p.push_clip(x + 1.0, vt, x + PANEL_W - 1.0, vb);
        let mut ry = vt - scroll;
        for s in reg.rows(module) {
            let rh = row_h(reg, s);
            if ry + rh >= vt && ry <= vb {
                self.draw_setting(p, reg, module, s, x, ry, rh, vt, vb);
            }
            ry += rh;
        }
        p.pop_clip();

        let clear = 0x000A0A0C;
        if scroll > 0.5 {
            p.vgrad(x + 1.0, vt, PANEL_W - 2.0, 14.0, PANEL_BG, clear);
        }
        if scroll < max_scroll - 0.5 {
            p.vgrad(x + 1.0, vb - 14.0, PANEL_W - 2.0, 14.0, clear, PANEL_BG);
        }

        if max_scroll > 0.0 {
            let tr_t = vt + 6.0;
            let tr_h = vh - 12.0;
            let th = (tr_h * vh / ch).max(24.0);
            let th_y = tr_t + (tr_h - th) * (scroll / max_scroll);
            let sx = x + PANEL_W - 11.0;
            p.fill(sx, tr_t, 4.0, tr_h, 2.0, 0xFF16161A);
            p.fill(sx, th_y, 4.0, th, 2.0, THUMB);
        }
        if let Some(pp) = self.popup.take() {
            self.draw_popup(p, reg, pp);
        }
        p.m = outer;
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_setting(
        &mut self,
        p: &mut Painter,
        reg: &Registry,
        module: usize,
        s: usize,
        x: f32,
        ry: f32,
        rh: f32,
        vt: f32,
        vb: f32,
    ) {
        let right = x + PANEL_W - 19.0;
        match &reg.settings[s].kind {
            Kind::Section => {
                let name = reg.settings[s].name.clone();
                let cy = ry + rh / 2.0;
                let w = text_width(&name, 9.5);
                let cx = x + (PANEL_W - w) / 2.0;
                p.text(&name, cx, ty(cy, 9.5), 9.5, PURPLE_TXT);
                p.fill(x + 8.0, cy, cx - 7.0 - (x + 8.0), 1.0, 0.0, 0xFF222228);
                p.fill(
                    cx + w + 7.0,
                    cy,
                    right - (cx + w + 7.0),
                    1.0,
                    0.0,
                    0xFF222228,
                );
            }
            Kind::Group { .. } => self.draw_group(p, reg, s, x, ry, rh, right, vt, vb),
            Kind::Button { .. } => self.draw_button(p, reg, s, x, ry, right, vt, vb),
            Kind::Bind { .. } => self.draw_bind(p, reg, s, x, ry, right, vt, vb),
            Kind::Text { .. } => self.draw_text(p, reg, s, x, ry, right, vt, vb),
            Kind::Bool { .. } => self.draw_bool(p, reg, s, x, ry, rh, right, vt, vb),
            Kind::Slider { .. } => self.draw_slider(p, reg, s, x, ry, right, vt, vb),
            Kind::Mode { .. } => self.draw_mode(p, reg, module, s, x, ry, right, vt, vb),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_group(
        &mut self,
        p: &mut Painter,
        reg: &Registry,
        s: usize,
        x: f32,
        ry: f32,
        rh: f32,
        right: f32,
        vt: f32,
        vb: f32,
    ) {
        let name = &reg.settings[s].name;
        let open = matches!(reg.settings[s].kind, Kind::Group { open: true, .. });
        let cy = ry + rh / 2.0;
        let w = text_width(name, 9.5);
        let cx = x + (PANEL_W - w) / 2.0;
        let over = self.hov_clip(x + 4.0, ry, PANEL_W - 8.0, rh, vt, vb);
        let hv = self.ap(Key::Setting(s), HOV, if over { 1.0 } else { 0.0 }, 16.0);
        p.text(name, cx, ty(cy, 9.5), 9.5, mix(PURPLE_TXT, WHITE, hv * 0.5));
        p.fill(x + 8.0, cy, cx - 7.0 - (x + 8.0), 1.0, 0.0, 0xFF222228);
        p.fill(
            cx + w + 7.0,
            cy,
            right - 12.0 - (cx + w + 7.0),
            1.0,
            0.0,
            0xFF222228,
        );
        let t = self.ap(Key::Setting(s), TOG, if open { 1.0 } else { 0.0 }, 14.0);
        let saved = p.m;
        let (ax, ay) = (right - 7.0, cy);
        p.m = saved.translate(ax, ay).rotate(PI * t).translate(-ax, -ay);
        p.chevron(ax - 3.25, ay - 3.25, 6.5, WHITE);
        p.m = saved;
        self.hit_clip(
            x + 4.0,
            ry,
            PANEL_W - 8.0,
            rh,
            vt,
            vb,
            Some(Act::ToggleGroup(s)),
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_button(
        &mut self,
        p: &mut Painter,
        reg: &Registry,
        s: usize,
        x: f32,
        ry: f32,
        right: f32,
        vt: f32,
        vb: f32,
    ) {
        let bx = x + 8.0;
        let bw = right - bx;
        let bh = 17.0;
        let by = ry + (25.5 - bh) / 2.0;
        let over = self.hov_clip(bx, by, bw, bh, vt, vb);
        let hv = self.ap(Key::Setting(s), HOV, if over { 1.0 } else { 0.0 }, 16.0);
        let pr = self.ap(Key::Setting(s), POP, 0.0, 10.0);
        p.fill(bx, by, bw, bh, 8.0, 0xFF111113);
        p.flow(bx, by, bw, bh, 8.0, 0.35 + 0.4 * hv + 0.25 * pr);
        p.border(bx, by, bw, bh, 8.0, 1.0, mix(0xFF2A2A30, 0x38FFFFFF, hv));
        let name = &reg.settings[s].name;
        let tw = text_width(name, 8.5);
        p.text(
            name,
            bx + (bw - tw) / 2.0,
            ty(by + bh / 2.0, 8.5),
            8.5,
            WHITE,
        );
        self.hit_clip(bx, by, bw, bh, vt, vb, Some(Act::PressButton(s)));
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_bind(
        &mut self,
        p: &mut Painter,
        reg: &Registry,
        s: usize,
        x: f32,
        ry: f32,
        right: f32,
        vt: f32,
        vb: f32,
    ) {
        let cy = ry + 12.75;
        let active = self.bind_setting == Some(s);
        let label = if active {
            "...".to_owned()
        } else {
            reg.get_bind(s).map_or_else(|| "None".to_owned(), key_label)
        };
        let lw = text_width(&label, 8.0);
        let bw = (lw + 14.0).max(34.0);
        let bh = 15.0;
        let bx = right - bw;
        let by = cy - bh / 2.0;
        let name_w = (bx - 6.0 - (x + 8.0)).max(10.0);
        let lines = wrap(&reg.settings[s].name, 9.5, name_w);
        if let Some(line) = lines.first() {
            p.text(line, x + 8.0, ty(cy, 9.5), 9.5, WHITE);
        }
        let over = self.hov_clip(bx, by, bw, bh, vt, vb);
        let hv = self.ap(Key::Setting(s), HOV, if over { 1.0 } else { 0.0 }, 16.0);
        let on = self.ap(Key::Setting(s), TOG, if active { 1.0 } else { 0.0 }, 14.0);
        p.fill(bx, by, bw, bh, 7.5, 0xFF111113);
        p.flow(bx, by, bw, bh, 7.5, 0.25 * hv + 0.75 * on);
        p.border(bx, by, bw, bh, 7.5, 1.0, mix(0xFF2A2A30, 0x38FFFFFF, hv));
        p.text(
            &label,
            bx + (bw - lw) / 2.0,
            ty(cy, 8.0),
            8.0,
            mix(PURPLE_TXT, WHITE, on),
        );
        let y0 = ry.max(vt);
        let y1 = (ry + 25.5).min(vb);
        if y1 > y0 && !self.closing {
            self.hits.push(Hit {
                x: x + 4.0,
                y: y0,
                w: PANEL_W - 8.0,
                h: y1 - y0,
                left: Some(Act::BindSetting(s)),
                right: None,
                mid: Some(Act::ClearBind(s)),
            });
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_text(
        &mut self,
        p: &mut Painter,
        reg: &Registry,
        s: usize,
        x: f32,
        ry: f32,
        right: f32,
        vt: f32,
        vb: f32,
    ) {
        let cy = ry + 12.75;
        let active = self.text_edit == Some(s);
        let name = &reg.settings[s].name;
        let nw = text_width(name, 9.5);
        p.text(name, x + 8.0, ty(cy, 9.5), 9.5, WHITE);
        let bx = x + 8.0 + nw + 8.0;
        let bw = (right - bx).max(30.0);
        let bh = 15.0;
        let by = cy - bh / 2.0;
        let over = self.hov_clip(bx, by, bw, bh, vt, vb);
        let hv = self.ap(Key::Setting(s), HOV, if over { 1.0 } else { 0.0 }, 16.0);
        let on = self.ap(Key::Setting(s), TOG, if active { 1.0 } else { 0.0 }, 14.0);
        p.fill(bx, by, bw, bh, 7.5, 0xFF111113);
        p.flow(bx, by, bw, bh, 7.5, 0.25 * hv + 0.75 * on);
        p.border(bx, by, bw, bh, 7.5, 1.0, mix(0xFF2A2A30, 0x38FFFFFF, hv));
        let caret = if active && (self.time * 2.0) as i64 % 2 == 0 { "|" } else { "" };
        let shown = format!("{}{}", reg.get_text(s), caret);
        p.push_clip(bx + 3.0, by, bx + bw - 3.0, by + bh);
        p.text(&shown, bx + 6.0, ty(cy, 8.0), 8.0, mix(0xFFDADAE1, WHITE, on));
        p.pop_clip();
        self.hit_clip(bx, by, bw, bh, vt, vb, Some(Act::EditText(s)));
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_bool(
        &mut self,
        p: &mut Painter,
        reg: &Registry,
        s: usize,
        x: f32,
        ry: f32,
        rh: f32,
        right: f32,
        vt: f32,
        vb: f32,
    ) {
        let lines = wrap(&reg.settings[s].name, 9.5, LABEL_W);
        let value = matches!(reg.settings[s].kind, Kind::Bool { value: true });
        let c0 = ry + 12.75;
        for (i, line) in lines.iter().enumerate() {
            p.text(line, x + 8.0, ty(c0 + i as f32 * 11.5, 9.5), 9.5, WHITE);
        }
        let (tw, th) = (25.0, 12.0);
        let tx = right - tw;
        let tyy = c0 - th / 2.0;
        let kn = th - 3.0;
        let t = self.ap(Key::Setting(s), TOG, if value { 1.0 } else { 0.0 }, 14.0);
        let kw = kn + (PI * t).sin() * 3.0;
        p.fill(tx, tyy, tw, th, 6.0, TOG_OFF);
        p.flow(tx, tyy, tw, th, 6.0, t);
        p.fill(
            tx + 1.5 + t * (tw - 3.0 - kw),
            tyy + 1.5,
            kw,
            kn,
            kn / 2.0,
            KNOB,
        );
        self.hit_clip(
            x + 4.0,
            ry,
            PANEL_W - 8.0,
            rh,
            vt,
            vb,
            Some(Act::ToggleBool(s)),
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_slider(
        &mut self,
        p: &mut Painter,
        reg: &Registry,
        s: usize,
        x: f32,
        ry: f32,
        right: f32,
        vt: f32,
        vb: f32,
    ) {
        p.text(
            &reg.settings[s].name,
            x + 8.0,
            ty(ry + 10.0, 9.5),
            9.5,
            WHITE,
        );
        let val = reg.slider_text(s);
        p.text(
            &val,
            right - text_width(&val, 8.5),
            ty(ry + 10.0, 8.5),
            8.5,
            PURPLE_TXT,
        );

        let trx = x + 8.0;
        let trw = right - trx;
        let tr_y = ry + 23.0;
        let dragging = self.drag_slider.is_some_and(|d| d.0 == s);
        let over = self.hov_clip(trx - 5.0, ry + 14.0, trw + 10.0, 18.0, vt, vb);
        let shown = self.ap(
            Key::Setting(s),
            SL,
            reg.slider_frac(s),
            if dragging { 40.0 } else { 16.0 },
        );
        let hv = self.ap(
            Key::Setting(s),
            HOV,
            if dragging || over { 1.0 } else { 0.0 },
            16.0,
        );

        p.fill(trx, tr_y - 2.0, trw, 4.0, 2.0, 0xFF1E1E24);
        let fw = (trw * shown).max(4.0);
        p.flow(trx, tr_y - 2.0, fw, 4.0, 2.0, 1.0);
        let kr = 4.0 + 1.5 * hv;
        let kx = trx + trw * shown;
        self.glow_k(p, kx - kr, tr_y - kr, kr * 2.0, kr * 2.0, kr, 5.0, 0x60, hv);
        p.fill(kx - kr, tr_y - kr, kr * 2.0, kr * 2.0, kr, KNOB);
        self.hit_clip(
            trx - 5.0,
            ry + 14.0,
            trw + 10.0,
            18.0,
            vt,
            vb,
            Some(Act::SliderStart {
                setting: s,
                x: trx,
                w: trw,
            }),
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_mode(
        &mut self,
        p: &mut Painter,
        reg: &Registry,
        module: usize,
        s: usize,
        x: f32,
        ry: f32,
        right: f32,
        vt: f32,
        vb: f32,
    ) {
        let Kind::Mode { modes, index, open } = &reg.settings[s].kind else {
            return;
        };
        let c0 = ry + 14.0;
        p.text(&reg.settings[s].name, x + 8.0, ty(c0, 9.5), 9.5, WHITE);
        let max_w = modes.iter().map(|o| text_width(o, 8.5)).fold(0.0, f32::max);
        let dd = self.peek(Key::Setting(s), DD, if *open { 1.0 } else { 0.0 });
        let cur = &modes[*index];
        let cw = text_width(cur, 8.5) + 7.0 + 15.0;
        let mw = max_w + 7.0 + 15.0;
        let bw = cw + (mw - cw) * dd;
        let bh = 15.0;
        let bx = right - bw;
        let by = c0 - bh / 2.0;
        if dd < 0.01 {
            self.glow(p, bx, by, bw, bh, 7.5, 5.0, 0x50);
            p.flow(bx, by, bw, bh, 7.5, 1.0);
            p.text(cur, bx + 7.0, ty(c0, 8.5), 8.5, WHITE);
            p.chevron(bx + bw - 11.5, c0 - 3.25, 6.5, WHITE);
        } else {
            self.popup = Some(Popup {
                setting: s,
                bx,
                by,
                bw,
                bh,
            });
        }
        self.hit_clip(
            bx,
            by,
            bw,
            bh,
            vt,
            vb,
            Some(Act::ModeToggle { module, setting: s }),
        );
    }

    fn draw_popup(&mut self, p: &mut Painter, reg: &Registry, pp: Popup) {
        let Kind::Mode { modes, index, open } = &reg.settings[pp.setting].kind else {
            return;
        };
        let (open, index) = (*open, *index);
        let dd = self.peek(Key::Setting(pp.setting), DD, if open { 1.0 } else { 0.0 });
        let n = modes.len();
        let full = n as f32 * OPT_H + 4.0;
        let list_h = dd * full;
        let h = pp.bh + list_h;
        let (bx, by, bw) = (pp.bx, pp.by, pp.bw);
        let c0 = by + pp.bh / 2.0;

        self.glow(p, bx, by, bw, pp.bh, 7.5, 5.0, 0x50);
        p.fill(bx, by, bw, h, 7.5, 0xFF0E0E11);
        p.border(bx, by, bw, h, 7.5, 1.0, 0xFF2A2A30);
        p.flow(bx, by, bw, pp.bh, 7.5, 1.0);
        p.text(&modes[index], bx + 7.0, ty(c0, 8.5), 8.5, WHITE);
        let saved = p.m;
        let acx = bx + bw - 8.25;
        p.m = saved
            .translate(acx, c0)
            .rotate(PI * dd)
            .translate(-acx, -c0);
        p.chevron(bx + bw - 11.5, c0 - 3.25, 6.5, WHITE);
        p.m = saved;

        let sa = self.ap(Key::Setting(pp.setting), SEL, index as f32, 16.0);
        let list_top = by + pp.bh + 2.0;
        let base = p.alpha;
        let pill_a = clamp01((list_h - 2.0 - sa * OPT_H) / OPT_H);
        p.flow(
            bx + 3.0,
            list_top + sa * OPT_H,
            bw - 6.0,
            OPT_H - 2.0,
            5.0,
            pill_a,
        );
        for (i, label) in modes.iter().enumerate() {
            let oy0 = list_top + i as f32 * OPT_H;
            let oa = clamp01((list_h - 2.0 - i as f32 * OPT_H) / OPT_H);
            if oa <= 0.01 {
                break;
            }
            p.alpha = base * oa;
            let sel = i == index;
            if !sel && open && self.hov_clip(bx + 3.0, oy0, bw - 6.0, OPT_H - 2.0, -1e6, 1e6) {
                p.fill(bx + 3.0, oy0, bw - 6.0, OPT_H - 2.0, 5.0, 0x1CFFFFFF);
            }
            p.text(
                label,
                bx + 7.0,
                ty(oy0 + (OPT_H - 2.0) / 2.0, 8.5),
                8.5,
                if sel { WHITE } else { 0xFFA8A8B2 },
            );
            p.alpha = base;
            if open && dd > 0.6 {
                self.hit(
                    bx + 3.0,
                    oy0,
                    bw - 6.0,
                    OPT_H - 2.0,
                    Some(Act::ModeSelect {
                        setting: pp.setting,
                        index: i,
                    }),
                    None,
                );
            }
        }
    }

    fn close_modes(reg: &mut Registry, modules: &[usize], except: Option<usize>) {
        for &m in modules {
            for s in reg.flat(m) {
                if Some(s) == except {
                    continue;
                }
                if let Kind::Mode { open, .. } = &mut reg.settings[s].kind {
                    *open = false;
                }
            }
        }
    }

    fn run(&mut self, act: Act, reg: &mut Registry) {
        match act {
            Act::SelectCat(c) => self.select_cat(c),
            Act::Close => self.close(),
            Act::FocusSearch => self.search_focus = true,
            Act::ToggleModule(m) => {
                reg.modules[m].enabled = !reg.modules[m].enabled;
                self.anim.insert((Key::Module(m), POP), 1.0);
            }
            Act::ClickModule(m) => {
                reg.events.push(Event::Click(m));
                self.anim.insert((Key::Module(m), POP), 1.0);
            }
            Act::EditText(s) => {
                self.text_edit = Some(s);
            }
            Act::OpenPanel(m) => self.open_panel(m),
            Act::Bind(m) => {
                self.bind_setting = None;
                self.binding = Some(m);
            }
            Act::BindSetting(s) => {
                self.binding = None;
                self.bind_setting = Some(s);
            }
            Act::ClearBind(s) => {
                self.bind_setting = None;
                reg.set_bind(s, None);
            }
            Act::Front(id) => self.front(id),
            Act::DragPanel(id) => {
                self.front(id);
                if let Some(pn) = self.panels.iter().find(|pn| pn.id == id) {
                    self.drag_x = self.cx - pn.x;
                    self.drag_y = self.cy - pn.y;
                    self.drag = Some(id);
                }
            }
            Act::ClosePanel(id) => {
                if let Some(pn) = self.panels.iter_mut().find(|pn| pn.id == id) {
                    pn.closing = true;
                }
            }
            Act::ToggleGroup(s) => {
                if let Kind::Group { open, .. } = &mut reg.settings[s].kind {
                    *open = !*open;
                }
            }
            Act::PressButton(s) => {
                reg.press(s);
                self.anim.insert((Key::Setting(s), POP), 1.0);
            }
            Act::ToggleBool(s) => {
                let v = matches!(reg.settings[s].kind, Kind::Bool { value: true });
                reg.set_bool(s, !v);
            }
            Act::SliderStart { setting, x, w } => {
                self.drag_slider = Some((setting, x, w));
                self.set_slider_from(reg, setting, self.cx, x, w);
            }
            Act::ModeToggle { module, setting } => {
                self.dd_keep = true;
                let now_open = if let Kind::Mode { open, .. } = &mut reg.settings[setting].kind {
                    *open = !*open;
                    *open
                } else {
                    false
                };
                if now_open {
                    Self::close_modes(reg, &[module], Some(setting));
                }
            }
            Act::ModeSelect { setting, index } => {
                reg.select_mode(setting, index);
                if let Kind::Mode { open, .. } = &mut reg.settings[setting].kind {
                    *open = false;
                }
            }
        }
    }

    fn set_slider_from(&self, reg: &mut Registry, s: usize, lx: f32, x: f32, w: f32) {
        if let Kind::Slider { min, max, .. } = reg.settings[s].kind {
            reg.set_slider(s, min + clamp01((lx - x) / w) * (max - min));
        }
    }

    pub fn mouse_pressed(&mut self, reg: &mut Registry, mouse: [f32; 2], button: i32) -> bool {
        if self.closing {
            return true;
        }
        if let Some(m) = self.binding.take() {
            reg.modules[m].key = Some(mouse_key(button));
            return true;
        }
        if let Some(st) = self.bind_setting.take() {
            reg.set_bind(st, Some(mouse_key(button)));
            return true;
        }
        self.cx = (mouse[0] - self.ox) / self.s;
        self.cy = (mouse[1] - self.oy) / self.s;
        self.search_focus = false;
        if let Some(t) = self.text_edit.take() {
            reg.events.push(Event::TextDone(t));
        }
        self.dd_keep = false;
        let (cx, cy) = (self.cx, self.cy);
        let found = self
            .hits
            .iter()
            .rev()
            .find(|h| cx >= h.x && cx < h.x + h.w && cy >= h.y && cy < h.y + h.h)
            .map(|h| match button {
                1 => h.right,
                2 => h.mid,
                _ => h.left,
            });
        let Some(act) = found else {
            return false;
        };
        if let Some(a) = act {
            self.run(a, reg);
        }
        if !self.dd_keep {
            let modules: Vec<usize> = self.panels.iter().map(|pn| pn.module).collect();
            Self::close_modes(reg, &modules, None);
        }
        true
    }

    pub fn mouse_dragged(&mut self, reg: &mut Registry, mouse: [f32; 2]) -> bool {
        let lx = (mouse[0] - self.ox) / self.s;
        let ly = (mouse[1] - self.oy) / self.s;
        if let Some((s, x, w)) = self.drag_slider {
            self.set_slider_from(reg, s, lx, x, w);
            return true;
        }
        if let Some(id) = self.drag {
            let (dx, dy) = (self.drag_x, self.drag_y);
            if let Some(pn) = self.panels.iter_mut().find(|pn| pn.id == id) {
                pn.x = lx - dx;
                pn.y = ly - dy;
            }
            return true;
        }
        false
    }

    pub fn mouse_released(&mut self) {
        self.drag = None;
        self.drag_slider = None;
    }

    pub fn mouse_scrolled(&mut self, reg: &Registry, mouse: [f32; 2], amount: f32) {
        let lx = (mouse[0] - self.ox) / self.s;
        let ly = (mouse[1] - self.oy) / self.s;
        for pn in self.panels.iter_mut().rev() {
            if !pn.closing
                && lx >= pn.x
                && lx < pn.x + PANEL_W
                && ly >= pn.y
                && ly < pn.y + panel_height(reg, pn.module)
            {
                pn.scroll_t = (pn.scroll_t - amount * 28.0).clamp(0.0, pn.max_scroll);
                return;
            }
        }
        self.scroll_t = (self.scroll_t - amount * 28.0).clamp(0.0, self.max_scroll);
    }

    pub fn char_typed_reg(&mut self, reg: &mut Registry, c: char) -> bool {
        if let Some(t) = self.text_edit {
            let mut v = reg.get_text(t).to_owned();
            if (c.is_alphanumeric() || matches!(c, ' ' | '-' | '_')) && v.chars().count() < 24 {
                v.push(c);
                reg.set_text(t, &v);
            }
            return true;
        }
        self.char_typed(c)
    }

    pub fn char_typed(&mut self, c: char) -> bool {
        if self.search_focus && (c as u32) >= 32 && self.query.chars().count() < MAX_QUERY {
            self.query.push(c);
            self.scroll = 0.0;
            self.scroll_t = 0.0;
            return true;
        }
        false
    }

    pub fn key_pressed(&mut self, reg: &mut Registry, key: &str) -> KeyResult {
        if let Some(m) = self.binding.take() {
            let clear = matches!(
                key,
                "key.keyboard.escape" | "key.keyboard.backspace" | "key.keyboard.delete"
            );
            reg.modules[m].key = if clear { None } else { Some(key.to_owned()) };
            return KeyResult::Consumed;
        }
        if let Some(st) = self.bind_setting.take() {
            match key {
                "key.keyboard.escape" => {}
                "key.keyboard.backspace" | "key.keyboard.delete" => reg.set_bind(st, None),
                _ => reg.set_bind(st, Some(key.to_owned())),
            }
            return KeyResult::Consumed;
        }
        if let Some(t) = self.text_edit {
            match key {
                "key.keyboard.backspace" => {
                    let mut v = reg.get_text(t).to_owned();
                    v.pop();
                    reg.set_text(t, &v);
                }
                "key.keyboard.enter" | "key.keyboard.keypad.enter" | "key.keyboard.escape" => {
                    self.text_edit = None;
                    reg.events.push(Event::TextDone(t));
                }
                _ => {}
            }
            return KeyResult::Consumed;
        }
        if self.search_focus && key == "key.keyboard.backspace" && !self.query.is_empty() {
            self.query.pop();
            return KeyResult::Consumed;
        }
        if self.search_focus && key == "key.keyboard.escape" {
            self.search_focus = false;
            return KeyResult::Consumed;
        }
        if key == "key.keyboard.escape" {
            self.close();
            return KeyResult::Consumed;
        }
        KeyResult::Ignored
    }
}

pub fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
