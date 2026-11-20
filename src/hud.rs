use crate::gui::key_label;
use crate::paint::{
    Affine, Canvas, ICON_INFO_COORDS, ICON_INFO_FPS, ICON_INFO_PING, ICON_INFO_SPEED, Icon,
    Painter, Theme, argb, gcol, text_width,
};
use crate::registry::Registry;
use std::time::Instant;

const BODY: u32 = 0xFF0A0A0C;
const WHITE: u32 = 0xFFFFFFFF;
const RED: u32 = 0xFFFF4A4A;
const HEAD: f32 = 19.0;
const ROW: f32 = 14.0;
const PANEL_W: f32 = 130.0;
const R: f32 = 7.0;
const ARR_H: f32 = 11.0;
const ARR_SIZE: f32 = 8.5;

fn clamp01(v: f32) -> f32 {
    v.clamp(0.0, 1.0)
}

fn ease_out_back(t: f32, s: f32) -> f32 {
    let t = clamp01(t);
    let u = t - 1.0;
    1.0 + (s + 1.0) * u * u * u + s * u * u
}

fn ease_out_cubic(t: f32) -> f32 {
    let u = 1.0 - clamp01(t);
    1.0 - u * u * u
}

fn lin(cur: f32, target: f32, dur: f32, dt: f32) -> f32 {
    let st = dt / dur;
    if cur < target {
        target.min(cur + st)
    } else {
        target.max(cur - st)
    }
}

fn ty(center: f32, size: f32) -> f32 {
    center - size * 0.58
}

pub struct HudIds {
    pub module: usize,
    pub size: usize,
    pub wm_on: usize,
    pub wm_lower: usize,
    pub wm_size: usize,
    pub arr_on: usize,
    pub arr_glow: usize,
    pub arr_lower: usize,
    pub po_on: usize,
    pub kb_on: usize,
    pub armor_on: usize,
    pub info_on: usize,
    pub info_side: usize,
    pub info_split: usize,
    pub info_fps: usize,
    pub info_ping: usize,
    pub info_coords: usize,
    pub info_speed: usize,
}

impl HudIds {
    pub fn register(reg: &mut Registry) -> HudIds {
        let module = reg.add_module(
            2,
            "HUD",
            "Watermark, module list, potions and keybinds panels",
            true,
        );
        let sg = reg.section("General");
        let size = reg.slider("Size", 1.0, 0.5, 2.0, 0.05, "x");
        let s0 = reg.section("Watermark");
        let wm_on = reg.boolean("Enabled", true);
        let wm_lower = reg.boolean("Lowercase", false);
        let wm_size = reg.slider("Text size", 8.0, 6.0, 14.0, 1.0, "");
        let s1 = reg.section("Module list");
        let arr_on = reg.boolean("Enabled", true);
        let arr_glow = reg.boolean("Glow", true);
        let arr_lower = reg.boolean("Lowercase", false);
        let s2 = reg.section("Potions");
        let po_on = reg.boolean("Enabled", true);
        let s3 = reg.section("Keybinds");
        let kb_on = reg.boolean("Enabled", true);
        let s4 = reg.section("Armor");
        let armor_on = reg.boolean("Enabled", true);
        let s5 = reg.section("Info panel");
        let info_on = reg.boolean("Enabled", true);
        let info_side = reg.boolean("Side display", false);
        let info_split = reg.boolean("Separate elements", false);
        let info_fps = reg.boolean("Frame rate", true);
        let info_ping = reg.boolean("Player ping", true);
        let info_coords = reg.boolean("Coordinates", true);
        let info_speed = reg.boolean("Player speed", true);
        reg.attach(
            module,
            &[
                sg, size, s0, wm_on, wm_lower, wm_size, s1, arr_on, arr_glow, arr_lower, s2, po_on,
                s3, kb_on, s4, armor_on, s5, info_on, info_side, info_split, info_fps, info_ping,
                info_coords, info_speed,
            ],
        );
        HudIds {
            module,
            size,
            wm_on,
            wm_lower,
            wm_size,
            arr_on,
            arr_glow,
            arr_lower,
            po_on,
            kb_on,
            armor_on,
            info_on,
            info_side,
            info_split,
            info_fps,
            info_ping,
            info_coords,
            info_speed,
        }
    }
}

pub struct Row {
    pub id: String,
    pub label: String,
    pub tag: Option<String>,
    pub tag_c: u32,
    pub value: String,
    pub val_c: u32,
}

pub struct HudData {
    pub username: String,
    pub clock: String,
    pub potions: Vec<Row>,
    pub fps: f32,
    pub ping: u32,
    pub pos: [f64; 3],
    pub bps: f64,
}

pub struct HudFrame {
    pub size: [f32; 2],
    pub mouse: [f32; 2],
    pub gui_scale: f32,
    pub chat: bool,
}

#[derive(Clone, Copy)]
struct El {
    fx: f32,
    fy: f32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    vis: f32,
}

impl El {
    fn new(fx: f32, fy: f32) -> El {
        El {
            fx,
            fy,
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
            vis: 0.0,
        }
    }

    fn contains(&self, m: [f32; 2]) -> bool {
        m[0] >= self.x && m[0] < self.x + self.w && m[1] >= self.y && m[1] < self.y + self.h
    }
}

struct RowS {
    id: String,
    label: String,
    tag: Option<String>,
    tag_c: u32,
    value: String,
    val_c: u32,
    t: f32,
    present: bool,
}

struct ListPanel {
    title: &'static str,
    el: El,
    rows: Vec<RowS>,
}

impl ListPanel {
    fn new(title: &'static str, fx: f32, fy: f32) -> ListPanel {
        ListPanel {
            title,
            el: El::new(fx, fy),
            rows: Vec::new(),
        }
    }

    fn want(&mut self, mut cur: Vec<Row>, preview: Row, chat: bool) -> bool {
        if cur.is_empty() && chat {
            cur = vec![preview];
        }
        if cur.is_empty() {
            return false;
        }
        for r in &mut self.rows {
            r.present = false;
        }
        for r in cur {
            let slot = match self.rows.iter().position(|s| s.id == r.id) {
                Some(i) => i,
                None => {
                    self.rows.push(RowS {
                        id: r.id.clone(),
                        label: String::new(),
                        tag: None,
                        tag_c: 0,
                        value: String::new(),
                        val_c: 0,
                        t: 0.0,
                        present: false,
                    });
                    self.rows.len() - 1
                }
            };
            let s = &mut self.rows[slot];
            s.label = r.label;
            s.tag = r.tag;
            s.tag_c = r.tag_c;
            s.value = r.value;
            s.val_c = r.val_c;
            s.present = true;
        }
        true
    }

    fn layout(&mut self, dt: f32) {
        let mut need = PANEL_W;
        let mut sum = 0.0;
        self.rows.retain_mut(|r| {
            r.t = lin(r.t, if r.present { 1.0 } else { 0.0 }, 0.25, dt);
            !(!r.present && r.t <= 0.0)
        });
        for r in &self.rows {
            let tag_w = r.tag.as_ref().map_or(0.0, |t| 4.0 + text_width(t, 8.0));
            let need2 =
                7.0 + text_width(&r.label, 8.0) + tag_w + 14.0 + text_width(&r.value, 8.0) + 7.0;
            need = need.max(need2);
            sum += ROW * ease_out_cubic(r.t);
        }
        self.el.w = need;
        self.el.h = HEAD + sum + 4.0;
    }

    fn draw(&self, p: &mut Painter, border: u32) {
        let El { x, y, w, h, .. } = self.el;
        p.flow_glow(x, y, w, h, R, 13.0, 0x78 as f32 / 255.0);
        p.fill(x, y, w, h, R, BODY);
        p.flow(x, y, w, (HEAD + R).min(h), R, 1.0);
        p.fill(
            x,
            y + HEAD,
            w,
            (R + 1.0).min(h - HEAD - 2.0).max(0.0),
            0.0,
            BODY,
        );
        p.border(x, y, w, h, R, 1.0, border);
        let tw = text_width(self.title, 10.0);
        p.text(self.title, x + (w - tw) / 2.0, ty(y + HEAD / 2.0, 10.0), 10.0, WHITE);
        let mut ry = y + HEAD;
        let base = p.alpha;
        for r in &self.rows {
            let e = ease_out_cubic(r.t);
            let rh = ROW * e;
            let c = ry + rh / 2.0;
            if e > 0.02 {
                p.alpha = base * e;
                let sl = (1.0 - e) * -8.0;
                let lx = x + 7.0 + sl;
                let lw = p.text(&r.label, lx, ty(c, 8.0), 8.0, WHITE);
                if let Some(tag) = &r.tag {
                    p.text(tag, lx + lw + 4.0, ty(c, 8.0), 8.0, r.tag_c);
                }
                p.text(
                    &r.value,
                    x + w - 7.0 - text_width(&r.value, 8.0) + sl,
                    ty(c, 8.0),
                    8.0,
                    r.val_c,
                );
                p.alpha = base;
            }
            ry += rh;
        }
    }
}

struct Ae {
    module: usize,
    t: f32,
    present: bool,
    name: String,
    w: f32,
}

#[derive(Clone, Copy, PartialEq)]
enum Drag {
    List,
    El(usize),
}

pub struct Hud {
    pub ids: HudIds,
    wm: El,
    wm_text: String,
    keybinds: ListPanel,
    potions: ListPanel,
    arr: Vec<Ae>,
    arr_fx: f32,
    arr_fy: f32,
    arr_box: [f32; 4],
    drag: Option<Drag>,
    grab: [f32; 2],
    pressed: bool,
    last_size: [f32; 2],
    scale: f32,
    border: u32,
    fps: f32,
    last: Option<Instant>,
    dt: f32,
    time: f32,
    flow_time: f64,
}

impl Hud {
    pub fn new(ids: HudIds) -> Hud {
        Hud {
            ids,
            wm: El::new(0.003, 0.012),
            wm_text: String::new(),
            keybinds: ListPanel::new("Keybinds", 0.0, 0.481),
            potions: ListPanel::new("Potions", 0.494, 0.303),
            arr: Vec::new(),
            arr_fx: 1.0,
            arr_fy: 0.004,
            arr_box: [0.0; 4],
            drag: None,
            grab: [0.0; 2],
            pressed: false,
            last_size: [1.0, 1.0],
            scale: 1.0,
            border: 0x90A582EC,
            fps: 0.0,
            last: None,
            dt: 0.016,
            time: 0.0,
            flow_time: 0.0,
        }
    }

    pub fn positions(&self) -> [[f32; 2]; 4] {
        [
            [self.wm.fx, self.wm.fy],
            [self.keybinds.el.fx, self.keybinds.el.fy],
            [self.potions.el.fx, self.potions.el.fy],
            [self.arr_fx, self.arr_fy],
        ]
    }

    pub fn set_positions(&mut self, p: &[[f32; 2]]) {
        for (i, v) in p.iter().enumerate().take(4) {
            let [x, y] = [clamp01(v[0]), clamp01(v[1])];
            if i == 3 {
                self.arr_fx = x;
                self.arr_fy = y;
            } else {
                let e = self.el(i);
                e.fx = x;
                e.fy = y;
            }
        }
    }

    fn el(&mut self, i: usize) -> &mut El {
        match i {
            0 => &mut self.wm,
            1 => &mut self.keybinds.el,
            _ => &mut self.potions.el,
        }
    }

    pub fn mouse_pressed(&mut self, mouse: [f32; 2]) -> bool {
        self.pressed = true;
        let k = self.scale.max(0.01);
        let mouse = [mouse[0] / k, mouse[1] / k];
        let [bx, by, bw, bh] = self.arr_box;
        if !self.arr.is_empty()
            && bw > 0.0
            && mouse[0] >= bx
            && mouse[0] < bx + bw
            && mouse[1] >= by
            && mouse[1] < by + bh
        {
            self.drag = Some(Drag::List);
            self.grab = [mouse[0] - self.arr_fx * self.last_size[0], mouse[1] - by];
            return true;
        }
        for i in (0..3).rev() {
            let e = *self.el(i);
            if e.vis > 0.5 && e.contains(mouse) {
                self.drag = Some(Drag::El(i));
                self.grab = [mouse[0] - e.x, mouse[1] - e.y];
                return true;
            }
        }
        false
    }

    pub fn mouse_released(&mut self) -> bool {
        self.pressed = false;
        self.drag.take().is_some()
    }

    fn handle_drag(&mut self, chat: bool, mouse: [f32; 2], sw: f32, sh: f32) {
        if !chat {
            self.drag = None;
            self.pressed = false;
            return;
        }
        match self.drag {
            Some(Drag::List) => {
                self.arr_fx = clamp01((mouse[0] - self.grab[0]) / sw);
                self.arr_fy = clamp01((mouse[1] - self.grab[1]) / sh);
            }
            Some(Drag::El(i)) => {
                let g = self.grab;
                let e = self.el(i);
                let nx = (mouse[0] - g[0]).min(sw - e.w).max(0.0);
                let ny = (mouse[1] - g[1]).min(sh - e.h).max(0.0);
                e.fx = nx / sw;
                e.fy = ny / sh;
            }
            None => {}
        }
    }

    fn tick_time(&mut self, theme: &Theme) {
        let now = Instant::now();
        self.dt = match self.last {
            Some(l) => now.duration_since(l).as_secs_f32().min(0.05),
            None => 0.016,
        };
        self.last = Some(now);
        self.time += self.dt;
        self.flow_time = (self.flow_time + self.dt as f64 * theme.speed as f64) % 100.0;
    }

    pub fn render(
        &mut self,
        canvas: &mut dyn Canvas,
        reg: &Registry,
        theme: &Theme,
        data: &HudData,
        frame: &HudFrame,
    ) {
        self.tick_time(theme);
        let k = reg.get_slider(self.ids.size).clamp(0.5, 2.0);
        self.scale = k;
        let [sw, sh] = [frame.size[0] / k, frame.size[1] / k];
        let mouse = [frame.mouse[0] / k, frame.mouse[1] / k];
        self.last_size = [sw, sh];
        let base = Affine::IDENTITY.scale(k);
        self.border = argb(theme.palette()[1], 0x90 as f32 / 255.0);
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
        let on = reg.modules[self.ids.module].enabled;
        self.handle_drag(frame.chat, mouse, sw, sh);
        self.draw_list(&mut p, reg, on, sw, sh, base);
        if frame.chat && !self.arr.is_empty() && self.arr_box[2] > 0.0 {
            let [bx, by, bw, bh] = self.arr_box;
            let hovered =
                mouse[0] >= bx && mouse[0] < bx + bw && mouse[1] >= by && mouse[1] < by + bh;
            let dragging = self.drag == Some(Drag::List);
            if dragging || hovered {
                p.m = base;
                p.alpha = 1.0;
                p.border(
                    bx - 2.0,
                    by - 2.0,
                    bw + 4.0,
                    bh + 4.0,
                    4.0,
                    1.0,
                    if dragging { 0xCCFFFFFF } else { 0x55FFFFFF },
                );
            }
        }

        let dt = self.dt;
        let chat = frame.chat;
        for i in 0..3 {
            let show = on && self.want(i, reg, data, chat);
            {
                let e = self.el(i);
                e.vis = lin(
                    e.vis,
                    if show { 1.0 } else { 0.0 },
                    if show { 0.5 } else { 0.22 },
                    dt,
                );
            }
            if self.el(i).vis <= 0.0 {
                match i {
                    1 => self.keybinds.rows.clear(),
                    2 => self.potions.rows.clear(),
                    _ => {}
                }
                continue;
            }
            match i {
                0 => {
                    let size = reg.get_slider(self.ids.wm_size);
                    self.wm.w = text_width(&self.wm_text, size) + 12.0;
                    self.wm.h = size + 8.0;
                }
                1 => self.keybinds.layout(dt),
                _ => self.potions.layout(dt),
            }
            let e = self.el(i);
            e.x = (e.fx * sw).min(sw - e.w).max(0.0);
            e.y = (e.fy * sh).min(sh - e.h).max(0.0);
            let e = *e;
            let k = if show {
                ease_out_back(e.vis, 1.8)
            } else {
                e.vis * e.vis
            };
            let sc = 0.7 + 0.3 * k;
            let dy = if i == 0 { -(1.0 - k) * 14.0 } else { 0.0 };
            let (cx, cy) = (e.x + e.w / 2.0, e.y + e.h / 2.0);
            p.m = base.translate(0.0, dy).about(cx, cy, sc);
            p.alpha = clamp01(e.vis * 2.5);
            match i {
                0 => self.draw_watermark(&mut p, reg),
                1 => self.keybinds.draw(&mut p, self.border),
                _ => self.potions.draw(&mut p, self.border),
            }
            if chat && (self.drag == Some(Drag::El(i)) || e.contains(mouse)) {
                let c = if self.drag == Some(Drag::El(i)) {
                    0xCCFFFFFF
                } else {
                    0x55FFFFFF
                };
                p.border(e.x - 2.0, e.y - 2.0, e.w + 4.0, e.h + 4.0, R + 2.0, 1.0, c);
            }
        }
        if on && reg.get_bool(self.ids.info_on) {
            p.m = base;
            p.alpha = 1.0;
            self.draw_info(&mut p, reg, data, sw);
        }
    }

    fn draw_info(&mut self, p: &mut Painter, reg: &Registry, data: &HudData, sw: f32) {
        const D: f32 = 16.0;
        const E: f32 = 8.0;
        const ICON: f32 = E;
        const PAD: f32 = 6.0;
        const ICON_GAP: f32 = 3.5;
        self.fps = if self.fps <= 0.0 {
            data.fps
        } else {
            self.fps + (data.fps - self.fps) * (self.dt * 8.0).clamp(0.0, 1.0)
        };
        let ids = &self.ids;
        let side = reg.get_bool(ids.info_side);
        let split = reg.get_bool(ids.info_split);
        let gap = if split { 2.0 } else { 5.0 };
        let mut top: Vec<(Icon, String)> = Vec::new();
        if reg.get_bool(ids.info_fps) {
            top.push((ICON_INFO_FPS, format!("{} FPS", self.fps as i32)));
        }
        if reg.get_bool(ids.info_ping) {
            top.push((ICON_INFO_PING, format!("{} ms", data.ping)));
        }
        let mut bottom: Vec<(Icon, String)> = Vec::new();
        if reg.get_bool(ids.info_coords) {
            bottom.push((
                ICON_INFO_COORDS,
                format!(
                    "x {} y {} z {}",
                    data.pos[0] as i64, data.pos[1] as i64, data.pos[2] as i64
                ),
            ));
        }
        if reg.get_bool(ids.info_speed) {
            bottom.push((ICON_INFO_SPEED, format!("{:.2} BPS", data.bps)));
        }
        let width = |sections: &[(Icon, String)]| -> f32 {
            if split {
                let mut w = 0.0;
                for (_, t) in sections {
                    w += PAD * 2.0 + ICON + ICON_GAP + stable_width(t, E) + gap;
                }
                return (w - gap).max(0.0);
            }
            let mut w = PAD;
            for (i, (_, t)) in sections.iter().enumerate() {
                if i > 0 {
                    w += 1.0 + gap;
                }
                w += ICON + ICON_GAP + stable_width(t, E) + gap;
            }
            w
        };
        if top.is_empty() {
            top = std::mem::take(&mut bottom);
        }
        if top.is_empty() {
            return;
        }
        let top_w = width(&top);
        let bottom_w = width(&bottom);
        let x = if side { 5.0 } else { (sw - top_w) / 2.0 };
        let y = if side && self.wm.vis > 0.0 && self.wm.x < x + top_w {
            (self.wm.y + self.wm.h + 6.0).max(5.0)
        } else {
            5.0
        };
        let primary = argb(p.palette[1], 1.0);
        self.info_row(p, x, y, top_w, &top, split, gap, primary);
        if !bottom.is_empty() {
            let bx = if side { x } else { x + (top_w - bottom_w) / 2.0 };
            self.info_row(p, bx, y + D + 4.0, bottom_w, &bottom, split, gap, primary);
        }
    }

    fn info_box(&self, p: &mut Painter, x: f32, y: f32, w: f32, h: f32) {
        p.flow_glow(x, y, w, h, 6.0, 11.0, 0x70 as f32 / 255.0);
        p.fill(x, y, w, h, 6.0, BODY);
        p.border(x, y, w, h, 6.0, 1.0, self.border);
    }

    #[allow(clippy::too_many_arguments)]
    fn info_row(
        &self,
        p: &mut Painter,
        x: f32,
        y: f32,
        w: f32,
        sections: &[(Icon, String)],
        split: bool,
        gap: f32,
        primary: u32,
    ) {
        const D: f32 = 16.0;
        const E: f32 = 8.0;
        const ICON: f32 = E;
        const PAD: f32 = 6.0;
        const ICON_GAP: f32 = 3.5;
        let text_y = ty(y + D / 2.0, E) - 0.5;
        if split {
            let mut cursor = x;
            for (icon, text) in sections {
                let sw = PAD * 2.0 + ICON + ICON_GAP + stable_width(text, E);
                self.info_box(p, cursor, y, sw, D);
                let inner = cursor + PAD;
                p.icon(*icon, inner, y + (D - ICON) / 2.0, ICON, primary);
                p.text(text, inner + ICON + ICON_GAP, text_y, E, WHITE);
                cursor += sw + gap;
            }
            return;
        }
        self.info_box(p, x, y, w, D);
        let divider = |p: &mut Painter, cx: f32| p.fill(cx, y + 4.0, 1.0, D - 8.0, 0.0, 0xFF2A2A32);
        let mut cursor = x + PAD;
        for (i, (icon, text)) in sections.iter().enumerate() {
            if i > 0 {
                divider(p, cursor);
                cursor += 1.0 + gap;
            }
            p.icon(*icon, cursor, y + (D - ICON) / 2.0, ICON, primary);
            cursor += ICON + ICON_GAP;
            p.text(text, cursor, text_y, E, WHITE);
            cursor += stable_width(text, E) + gap;
        }
    }

    fn want(&mut self, i: usize, reg: &Registry, data: &HudData, chat: bool) -> bool {
        match i {
            0 => {
                if !reg.get_bool(self.ids.wm_on) {
                    return false;
                }
                let s = format!("Celestial » {} » UID: 2 » {}", data.username, data.clock);
                self.wm_text = if reg.get_bool(self.ids.wm_lower) {
                    s.to_lowercase()
                } else {
                    s.to_uppercase()
                };
                true
            }
            1 => {
                if !reg.get_bool(self.ids.kb_on) {
                    return false;
                }
                let cur: Vec<Row> = reg
                    .modules
                    .iter()
                    .filter(|m| m.enabled && m.key.is_some())
                    .map(|m| Row {
                        id: m.name.clone(),
                        label: m.name.clone(),
                        tag: None,
                        tag_c: 0,
                        value: format!("[{}]", key_label(m.key.as_deref().unwrap_or(""))),
                        val_c: WHITE,
                    })
                    .collect();
                let preview = Row {
                    id: "p".to_owned(),
                    label: "Preview Bind".to_owned(),
                    tag: None,
                    tag_c: 0,
                    value: "[Key]".to_owned(),
                    val_c: WHITE,
                };
                self.keybinds.want(cur, preview, chat)
            }
            _ => {
                if !reg.get_bool(self.ids.po_on) {
                    return false;
                }
                let cur: Vec<Row> = data
                    .potions
                    .iter()
                    .map(|r| Row {
                        id: r.id.clone(),
                        label: r.label.clone(),
                        tag: r.tag.clone(),
                        tag_c: r.tag_c,
                        value: r.value.clone(),
                        val_c: r.val_c,
                    })
                    .collect();
                let preview = Row {
                    id: "p".to_owned(),
                    label: "Preview Potion".to_owned(),
                    tag: Some("999".to_owned()),
                    tag_c: RED,
                    value: "**:**".to_owned(),
                    val_c: WHITE,
                };
                self.potions.want(cur, preview, chat)
            }
        }
    }

    fn draw_watermark(&self, p: &mut Painter, reg: &Registry) {
        let El { x, y, w, h, .. } = self.wm;
        let size = reg.get_slider(self.ids.wm_size);
        p.flow_glow(x, y, w, h, 6.0, 11.0, 0x70 as f32 / 255.0);
        p.fill(x, y, w, h, 6.0, BODY);
        p.border(x, y, w, h, 6.0, 1.0, self.border);
        p.text(&self.wm_text, x + 6.0, ty(y + h / 2.0, size), size, WHITE);
    }

    fn arr_u(&self, yy: f32, total: f32) -> f32 {
        let u = yy / total.max(60.0);
        clamp01(u * 0.95 + 0.06 * (self.time * 1.2 - yy * 0.05).sin())
    }

    fn draw_list(
        &mut self,
        p: &mut Painter,
        reg: &Registry,
        hud_on: bool,
        sw: f32,
        sh: f32,
        base: Affine,
    ) {
        let on = hud_on && reg.get_bool(self.ids.arr_on);
        for a in &mut self.arr {
            a.present = false;
        }
        if on {
            for (i, m) in reg.modules.iter().enumerate() {
                if !m.enabled || i == self.ids.module || m.category >= 5 {
                    continue;
                }
                match self.arr.iter_mut().find(|a| a.module == i) {
                    Some(a) => a.present = true,
                    None => self.arr.push(Ae {
                        module: i,
                        t: 0.0,
                        present: true,
                        name: String::new(),
                        w: 0.0,
                    }),
                }
            }
        }
        let dt = self.dt;
        let lower = reg.get_bool(self.ids.arr_lower);
        self.arr.retain_mut(|a| {
            a.t = lin(a.t, if a.present { 1.0 } else { 0.0 }, 0.3, dt);
            if !a.present && a.t <= 0.0 {
                return false;
            }
            let name = &reg.modules[a.module].name;
            a.name = if lower { name.to_lowercase() } else { name.clone() };
            a.w = text_width(&a.name, ARR_SIZE) + 9.0;
            true
        });
        if self.arr.is_empty() {
            self.arr_box = [0.0; 4];
            return;
        }
        self.arr.sort_by(|a, b| b.w.total_cmp(&a.w));

        let n = self.arr.len();
        let mut ys = vec![0.0f32; n];
        let mut hs = vec![0.0f32; n];
        let mut y = 0.0;
        for i in 0..n {
            ys[i] = y;
            hs[i] = ARR_H * ease_out_cubic(self.arr[i].t);
            y += hs[i];
        }
        let total = y;
        let max_w = self.arr.iter().fold(0.0f32, |m, a| m.max(a.w));
        let ax = (self.arr_fx * sw).min(sw).max(0.0);
        let left = ax < sw / 2.0;
        let top = (self.arr_fy * sh).min(sh - total).max(0.0);
        self.arr_box = [if left { ax } else { ax - max_w }, top, max_w, total];
        p.m = base;
        p.alpha = 1.0;
        let pal = p.palette;

        if reg.get_bool(self.ids.arr_glow) {
            for i in 0..n {
                let a = &self.arr[i];
                if hs[i] < 0.5 {
                    continue;
                }
                let col = gcol(&pal, self.arr_u(ys[i] + hs[i] / 2.0, total));
                let alpha = ((0x78 as f32 * ease_out_cubic(a.t)).round() as u32).min(255);
                let x = if left { ax } else { ax - a.w };
                p.glow(
                    x,
                    top + ys[i],
                    a.w,
                    hs[i],
                    0.0,
                    8.0,
                    (alpha << 24) | (col & 0x00FF_FFFF),
                );
            }
        }
        for i in 0..n {
            let a = &self.arr[i];
            if hs[i] < 0.3 {
                continue;
            }
            let x = if left { ax } else { ax - a.w };
            p.fill(x, top + ys[i], a.w, hs[i] + 0.6, 0.0, 0xFF07070A);
        }
        for i in 0..n {
            let a = &self.arr[i];
            let e = ease_out_cubic(a.t);
            if e < 0.35 {
                continue;
            }
            let col = gcol(&pal, self.arr_u(ys[i] + hs[i] / 2.0, total));
            let alpha = ((255.0 * clamp01((e - 0.35) / 0.65)).round() as u32).min(255);
            let x = if left { ax } else { ax - a.w };
            p.text(
                &a.name,
                x + 4.5,
                ty(top + ys[i] + hs[i] / 2.0, ARR_SIZE),
                ARR_SIZE,
                (alpha << 24) | (col & 0x00FF_FFFF),
            );
        }
    }
}

fn stable_width(s: &str, size: f32) -> f32 {
    let digit = ('0'..='9')
        .map(|d| text_width(d.encode_utf8(&mut [0; 4]), size))
        .fold(0.0f32, f32::max);
    let mut w = 0.0;
    let mut rest = String::new();
    for c in s.chars() {
        if c.is_ascii_digit() {
            w += digit;
        } else {
            rest.push(c);
        }
    }
    w + text_width(&rest, size)
}
