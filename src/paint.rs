use crate::metrics;
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Affine {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub e: f32,
    pub f: f32,
}

impl Affine {
    pub const IDENTITY: Affine = Affine {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };

    pub fn translate(self, x: f32, y: f32) -> Affine {
        Affine {
            e: self.a * x + self.c * y + self.e,
            f: self.b * x + self.d * y + self.f,
            ..self
        }
    }

    pub fn scale(self, s: f32) -> Affine {
        Affine {
            a: self.a * s,
            b: self.b * s,
            c: self.c * s,
            d: self.d * s,
            ..self
        }
    }

    pub fn rotate(self, ang: f32) -> Affine {
        let (s, c) = ang.sin_cos();
        Affine {
            a: self.a * c + self.c * s,
            b: self.b * c + self.d * s,
            c: -self.a * s + self.c * c,
            d: -self.b * s + self.d * c,
            ..self
        }
    }

    pub fn about(self, cx: f32, cy: f32, s: f32) -> Affine {
        self.translate(cx, cy).scale(s).translate(-cx, -cy)
    }

    pub fn apply(&self, x: f32, y: f32) -> [f32; 2] {
        [
            self.a * x + self.c * y + self.e,
            self.b * x + self.d * y + self.f,
        ]
    }

    pub fn factor(&self) -> f32 {
        (self.a * self.d - self.b * self.c).abs().sqrt()
    }
}

pub trait Canvas {
    fn quad(&mut self, points: [[f32; 2]; 4], argb: u32);
    fn gradient(&mut self, rect: [f32; 4], top: u32, bottom: u32);
    fn sprite(&mut self, name: &str, size: [f32; 2], dst: [f32; 4], tint: u32);
    fn text(&mut self, font: &str, text: &str, origin: [f32; 2], scale: f32, argb: u32);
    fn sprite_src(&mut self, name: &str, src: [f32; 4], dst: [f32; 4], tint: u32);
    fn push_clip(&mut self, rect: [f32; 4]);
    fn pop_clip(&mut self);
}

#[derive(Clone, Copy)]
pub struct Icon {
    pub sprite: &'static str,
    pub size: [f32; 2],
}

pub const ICON_COMBAT: Icon = Icon {
    sprite: "client:gui/category/combat",
    size: [96.0, 96.0],
};
pub const ICON_MOVEMENT: Icon = Icon {
    sprite: "client:gui/category/movement",
    size: [96.0, 96.0],
};
pub const ICON_RENDER: Icon = Icon {
    sprite: "client:gui/category/render",
    size: [96.0, 96.0],
};
pub const ICON_PLAYER: Icon = Icon {
    sprite: "client:gui/category/player",
    size: [96.0, 96.0],
};
pub const ICON_UTIL: Icon = Icon {
    sprite: "client:gui/category/util",
    size: [96.0, 96.0],
};
pub const ICON_CONFIGS: Icon = Icon {
    sprite: "client:gui/category/configs",
    size: [96.0, 96.0],
};
pub const ICON_THEMES: Icon = Icon {
    sprite: "client:gui/category/themes",
    size: [96.0, 96.0],
};
pub const ICON_POWER: Icon = Icon {
    sprite: "client:gui/logout",
    size: [96.0, 96.0],
};
pub const ICON_INFO_FPS: Icon = Icon {
    sprite: "client:gui/info/fps",
    size: [48.0, 44.0],
};
pub const ICON_INFO_PING: Icon = Icon {
    sprite: "client:gui/info/ping",
    size: [48.0, 35.0],
};
pub const ICON_INFO_COORDS: Icon = Icon {
    sprite: "client:gui/info/coords",
    size: [48.0, 46.0],
};
pub const ICON_INFO_SPEED: Icon = Icon {
    sprite: "client:gui/info/speed",
    size: [48.0, 45.0],
};
pub const ICON_GEAR: Icon = Icon {
    sprite: "client:gui/gear",
    size: [25.0, 24.0],
};
pub const ICON_SEARCH: Icon = Icon {
    sprite: "client:gui/search",
    size: [32.0, 32.0],
};
pub const ICON_CLOSE: Icon = Icon {
    sprite: "client:gui/close",
    size: [64.0, 64.0],
};
pub const ICON_ATOM: Icon = Icon {
    sprite: "client:gui/atom",
    size: [64.0, 64.0],
};

pub struct Theme {
    pub custom: bool,
    pub hue: f32,
    pub sat: f32,
    pub bri: f32,
    pub speed: f32,
    pub blur: bool,
    pub glow: bool,
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            custom: false,
            hue: 262.0,
            sat: 0.51,
            bri: 0.95,
            speed: 1.0,
            blur: true,
            glow: true,
        }
    }
}

const GA: [f32; 3] = [0.88, 0.78, 1.00];
const GB: [f32; 3] = [0.66, 0.47, 0.95];
const GC: [f32; 3] = [0.46, 0.27, 0.84];

impl Theme {
    pub fn palette(&self) -> [[f32; 3]; 3] {
        if !self.custom {
            return [GA, GB, GC];
        }
        let b = hsb(self.hue, self.sat, self.bri);
        let a = [
            b[0] + (1.0 - b[0]) * 0.55,
            b[1] + (1.0 - b[1]) * 0.55,
            b[2] + (1.0 - b[2]) * 0.55,
        ];
        let c = [b[0] * 0.70, b[1] * 0.57, b[2] * 0.88];
        [a, b, c]
    }
}

pub fn hsb(h: f32, s: f32, v: f32) -> [f32; 3] {
    let h = ((h % 360.0) + 360.0) % 360.0 / 60.0;
    let i = h as i32;
    let f = h - i as f32;
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));
    match i {
        0 => [v, t, p],
        1 => [q, v, p],
        2 => [p, v, t],
        3 => [p, q, v],
        4 => [t, p, v],
        _ => [v, p, q],
    }
}

fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn mix3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

pub fn flow_rgb(n: [f32; 2], center: [f32; 2], t: f32, pal: &[[f32; 3]; 3]) -> [f32; 3] {
    let th = t * 0.57 + center[0] * 0.0012 + center[1] * 0.0009;
    let dir = [th.cos(), th.sin()];
    let mut u = (n[0] * dir[0] + n[1] * dir[1]) * 0.36 + 0.5;
    let across = n[0] * -dir[1] + n[1] * dir[0];
    u += 0.05 * (std::f32::consts::TAU * (across * 0.25 + t * 0.09)).sin();
    let u = u.clamp(0.0, 1.0);
    let u = u * u * (3.0 - 2.0 * u);
    let c = mix3(pal[2], pal[1], smoothstep(0.0, 0.55, u));
    mix3(c, pal[0], smoothstep(0.45, 1.0, u))
}

pub fn gcol(pal: &[[f32; 3]; 3], u: f32) -> u32 {
    let u = u.clamp(0.0, 1.0);
    let rgb = if u < 0.5 {
        mix3(pal[0], pal[1], u * 2.0)
    } else {
        mix3(pal[1], pal[2], (u - 0.5) * 2.0)
    };
    argb(rgb, 1.0)
}

pub fn argb(rgb: [f32; 3], a: f32) -> u32 {
    let ch = |v: f32| ((v.clamp(0.0, 1.0) * 255.0).round() as u32) & 0xFF;
    (ch(a) << 24) | (ch(rgb[0]) << 16) | (ch(rgb[1]) << 8) | ch(rgb[2])
}

pub fn with_alpha(c: u32, k: f32) -> u32 {
    let a = (((c >> 24) & 0xFF) as f32 * k).round().clamp(0.0, 255.0) as u32;
    (a << 24) | (c & 0x00FF_FFFF)
}

pub fn mix(a: u32, b: u32, t: f32) -> u32 {
    let mut r = 0u32;
    for sh in [24u32, 16, 8, 0] {
        let ca = ((a >> sh) & 0xFF) as f32;
        let cb = ((b >> sh) & 0xFF) as f32;
        r |= (((ca + (cb - ca) * t).round() as i32 as u32) & 0xFF) << sh;
    }
    r
}

pub fn text_width(s: &str, size: f32) -> f32 {
    metrics::width(s, size)
}

pub fn font_for(physical_em: f32) -> &'static str {
    const NAMES: [&str; 8] = [
        "client:m10",
        "client:m13",
        "client:m16",
        "client:m20",
        "client:m25",
        "client:m32",
        "client:m40",
        "client:m50",
    ];
    for (i, p) in metrics::PPEMS.iter().enumerate() {
        if *p as f32 >= physical_em * 0.9 {
            return NAMES[i];
        }
    }
    NAMES[NAMES.len() - 1]
}

pub const FONT_SIZE: f32 = 10.0;
pub const FONT_BASELINE: f32 = 7.0;

fn sd_round_box(p: [f32; 2], half: [f32; 2], r: f32) -> f32 {
    let qx = p[0].abs() - half[0] + r;
    let qy = p[1].abs() - half[1] + r;
    let ox = qx.max(0.0);
    let oy = qy.max(0.0);
    (ox * ox + oy * oy).sqrt() + qx.max(qy).min(0.0) - r
}

fn clip_poly(subject: &[[f32; 2]], clip: &[[f32; 2]], sign: f32) -> Vec<[f32; 2]> {
    let mut out: Vec<[f32; 2]> = subject.to_vec();
    let n = clip.len();
    for i in 0..n {
        if out.is_empty() {
            break;
        }
        let p = clip[i];
        let q = clip[(i + 1) % n];
        if (q[0] - p[0]).abs() < 1e-6 && (q[1] - p[1]).abs() < 1e-6 {
            continue;
        }
        let side =
            |v: [f32; 2]| sign * ((q[0] - p[0]) * (v[1] - p[1]) - (q[1] - p[1]) * (v[0] - p[0]));
        let input = std::mem::take(&mut out);
        let m = input.len();
        for j in 0..m {
            let cur = input[j];
            let prev = input[(j + m - 1) % m];
            let sc = side(cur);
            let sp = side(prev);
            if sc >= 0.0 {
                if sp < 0.0 {
                    let t = sp / (sp - sc);
                    out.push([
                        prev[0] + (cur[0] - prev[0]) * t,
                        prev[1] + (cur[1] - prev[1]) * t,
                    ]);
                }
                out.push(cur);
            } else if sp >= 0.0 {
                let t = sp / (sp - sc);
                out.push([
                    prev[0] + (cur[0] - prev[0]) * t,
                    prev[1] + (cur[1] - prev[1]) * t,
                ]);
            }
        }
    }
    out
}

pub struct Painter<'a> {
    pub canvas: &'a mut dyn Canvas,
    pub m: Affine,
    pub alpha: f32,
    pub px: f32,
    pub time: f32,
    pub palette: [[f32; 3]; 3],
    pub glow: bool,
    pub rec: Option<Vec<Op>>,
}

#[derive(Clone, Copy)]
pub enum Op {
    Quad([[f32; 2]; 4], u32),
    Grad([f32; 4], u32, u32),
}

#[derive(Clone, Copy)]
enum FlowOp {
    Grad([f32; 4], [f32; 2], [f32; 2]),
    Quad([[f32; 2]; 4], [f32; 2], f32),
}

const CACHE_MAX: usize = 4096;

thread_local! {
    static CACHE: RefCell<HashMap<u64, Rc<Vec<Op>>>> = RefCell::new(HashMap::new());
    static FLOW_CACHE: RefCell<HashMap<u64, Rc<Vec<FlowOp>>>> = RefCell::new(HashMap::new());
}

const GLOW: [&str; 16] = [
    "client:gui/glow/g0",
    "client:gui/glow/g1",
    "client:gui/glow/g2",
    "client:gui/glow/g3",
    "client:gui/glow/g4",
    "client:gui/glow/g5",
    "client:gui/glow/g6",
    "client:gui/glow/g7",
    "client:gui/glow/g8",
    "client:gui/glow/g9",
    "client:gui/glow/g10",
    "client:gui/glow/g11",
    "client:gui/glow/g12",
    "client:gui/glow/g13",
    "client:gui/glow/g14",
    "client:gui/glow/g15",
];

struct Shape {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    r: f32,
}

impl Shape {
    fn new(x: f32, y: f32, w: f32, h: f32, r: f32) -> Shape {
        let r = r.clamp(0.0, (w * 0.5).min(h * 0.5));
        Shape { x, y, w, h, r }
    }

    fn center(&self) -> [f32; 2] {
        [self.x + self.w * 0.5, self.y + self.h * 0.5]
    }

    fn half(&self) -> [f32; 2] {
        [self.w * 0.5, self.h * 0.5]
    }

    fn outline(&self, grow: f32, n: usize, step: Option<f32>) -> Vec<[f32; 2]> {
        let mut x0 = self.x + self.r;
        let mut y0 = self.y + self.r;
        let mut x1 = self.x + self.w - self.r;
        let mut y1 = self.y + self.h - self.r;
        let mut rad = self.r + grow;
        if rad < 0.0 {
            let s = -rad;
            let mx = (x0 + x1) * 0.5;
            let my = (y0 + y1) * 0.5;
            x0 = (x0 + s).min(mx);
            x1 = (x1 - s).max(mx);
            y0 = (y0 + s).min(my);
            y1 = (y1 - s).max(my);
            rad = 0.0;
        }
        let base_w = (self.w - 2.0 * self.r).max(0.0);
        let base_h = (self.h - 2.0 * self.r).max(0.0);
        let kw = step.map_or(1, |s| ((base_w / s).ceil() as usize).max(1));
        let kh = step.map_or(1, |s| ((base_h / s).ceil() as usize).max(1));
        let corners = [
            (x0, y0, std::f32::consts::PI, kw, x1, y0),
            (x1, y0, std::f32::consts::PI * 1.5, kh, x1, y1),
            (x1, y1, 0.0, kw, x0, y1),
            (x0, y1, std::f32::consts::PI * 0.5, kh, x0, y0),
        ];
        let mut pts = Vec::with_capacity(4 * (n + 1) + 2 * (kw + kh));
        for (cx, cy, start, k, nx, ny) in corners {
            for i in 0..=n {
                let a = start + std::f32::consts::FRAC_PI_2 * (i as f32 / n.max(1) as f32);
                pts.push([cx + rad * a.cos(), cy + rad * a.sin()]);
            }
            let last = *pts.last().unwrap();
            let (dx, dy) = (nx - cx, ny - cy);
            for j in 1..k {
                let t = j as f32 / k as f32;
                pts.push([last[0] + dx * t, last[1] + dy * t]);
            }
        }
        pts
    }
}

impl<'a> Painter<'a> {
    pub fn col(&self, c: u32) -> u32 {
        with_alpha(c, self.alpha)
    }

    fn local_px(&self) -> f32 {
        self.px / self.m.factor().max(1e-4)
    }

    fn arc_steps(&self, r: f32) -> usize {
        let phys = r * self.m.factor() / self.px;
        ((phys / 2.0).ceil() as usize).clamp(2, 24)
    }

    fn tq(&mut self, p: [[f32; 2]; 4], c: u32) {
        if (c >> 24) == 0 {
            return;
        }
        let m = self.m;
        let q = [
            m.apply(p[0][0], p[0][1]),
            m.apply(p[1][0], p[1][1]),
            m.apply(p[2][0], p[2][1]),
            m.apply(p[3][0], p[3][1]),
        ];
        match &mut self.rec {
            Some(buf) => buf.push(Op::Quad(q, c)),
            None => self.canvas.quad(q, c),
        }
    }

    fn grad(&mut self, r: [f32; 4], top: u32, bottom: u32) {
        match &mut self.rec {
            Some(buf) => buf.push(Op::Grad(r, top, bottom)),
            None => self.canvas.gradient(r, top, bottom),
        }
    }

    fn key(&self, tag: u8, vals: &[f32], extra: &[u32]) -> u64 {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        tag.hash(&mut h);
        for v in vals {
            v.to_bits().hash(&mut h);
        }
        extra.hash(&mut h);
        let m = self.m;
        for v in [m.a, m.b, m.c, m.d, m.e, m.f, self.px] {
            v.to_bits().hash(&mut h);
        }
        h.finish()
    }

    fn replay(&mut self, ops: &[Op]) {
        for op in ops {
            match *op {
                Op::Quad(q, c) => self.canvas.quad(q, c),
                Op::Grad(r, t, b) => self.canvas.gradient(r, t, b),
            }
        }
    }

    fn cached(&mut self, key: u64, f: impl FnOnce(&mut Self)) {
        if self.rec.is_some() {
            f(self);
            return;
        }
        if let Some(ops) = CACHE.with(|c| c.borrow().get(&key).cloned()) {
            self.replay(&ops);
            return;
        }
        self.rec = Some(Vec::new());
        f(self);
        let ops = Rc::new(self.rec.take().unwrap_or_default());
        self.replay(&ops);
        CACHE.with(|c| {
            let mut c = c.borrow_mut();
            if c.len() >= CACHE_MAX {
                c.clear();
            }
            c.insert(key, ops);
        });
    }

    fn glow_sprites(&mut self, s: &Shape, spread: f32, color: &dyn Fn([f32; 2]) -> u32) {
        let e = s.r + spread;
        if e <= 0.0 || spread <= 0.0 {
            return;
        }
        let name = GLOW[((s.r / e * 16.0).round() as usize).min(15)];
        let (x0, y0) = (s.x - spread, s.y - spread);
        let (x1, y1) = (s.x + s.w + spread, s.y + s.h + spread);
        let xs = glow_axis(x0, x1, e, spread, 5.0, true);
        let ys = glow_axis(y0, y1, e, spread, 3.0, true);
        let xc = glow_axis(x0, x1, e, spread, 5.0, false);
        let yc = glow_axis(y0, y1, e, spread, 3.0, false);
        let (sx0, sy0, sx1, sy1) = (s.x, s.y, s.x + s.w, s.y + s.h);
        let mut cells = Vec::new();
        for &cx in &xs {
            if cx.4 {
                continue;
            }
            for &cy in &ys {
                if !cy.4 {
                    cells.push((cx, cy));
                }
            }
        }
        for &cx in xs.iter().filter(|c| c.4) {
            for &cy in yc.iter().filter(|c| !c.4) {
                cells.push((cx, cy));
            }
        }
        for &cy in ys.iter().filter(|c| c.4) {
            for &cx in xc.iter().filter(|c| !c.4) {
                cells.push((cx, cy));
            }
        }
        for ((ax, bx, ux, uw, _), (ay, by, uy, uh, _)) in cells {
            {
                let at = [
                    ((ax + bx) * 0.5).clamp(sx0, sx1),
                    ((ay + by) * 0.5).clamp(sy0, sy1),
                ];
                let c = color(at);
                if (c >> 24) == 0 {
                    continue;
                }
                let a = self.m.apply(ax, ay);
                let b = self.m.apply(bx, by);
                self.canvas
                    .sprite_src(name, [ux, uy, uw, uh], [a[0], a[1], b[0] - a[0], b[1] - a[1]], c);
            }
        }
    }

    fn fan(&mut self, center: [f32; 2], pts: &[[f32; 2]], c: u32) {
        let n = pts.len();
        if n < 2 {
            return;
        }
        let mut i = 0;
        while i < n {
            let a = pts[i];
            let b = pts[(i + 1) % n];
            if i + 1 < n {
                self.tq([center, a, b, pts[(i + 2) % n]], c);
            } else {
                self.tq([center, a, b, b], c);
            }
            i += 2;
        }
    }

    fn ring_with(
        &mut self,
        s: &Shape,
        g_in: f32,
        g_out: f32,
        step: Option<f32>,
        color: &dyn Fn([f32; 2]) -> u32,
    ) {
        let n = self.arc_steps(s.r + g_out.max(0.0));
        self.ring_n(s, g_in, g_out, n, step, color);
    }

    fn ring_n(
        &mut self,
        s: &Shape,
        g_in: f32,
        g_out: f32,
        n: usize,
        step: Option<f32>,
        color: &dyn Fn([f32; 2]) -> u32,
    ) {
        for (q, c) in ring_quads(s, g_in, g_out, n, step, color) {
            self.tq(q, c);
        }
    }

    pub fn fill(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, c: u32) {
        if w <= 0.0 || h <= 0.0 {
            return;
        }
        let c = self.col(c);
        if (c >> 24) == 0 {
            return;
        }
        let key = self.key(1, &[x, y, w, h, r], &[c]);
        self.cached(key, |p| p.fill_raw(x, y, w, h, r, c));
    }

    fn fill_raw(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, c: u32) {
        let s = Shape::new(x, y, w, h, r);
        let n = self.arc_steps(s.r);
        if s.r <= 0.0 {
            self.tq([[x, y], [x + w, y], [x + w, y + h], [x, y + h]], c);
        } else {
            let pts = s.outline(0.0, n, None);
            self.fan(s.center(), &pts, c);
        }
        let f = self.local_px() * 0.75;
        let fc = with_alpha(c, 0.45);
        self.ring_n(&s, 0.0, f, n, None, &|_| fc);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn border(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, t: f32, c: u32) {
        if w <= 0.0 || h <= 0.0 {
            return;
        }
        let c = self.col(c);
        if (c >> 24) == 0 {
            return;
        }
        let key = self.key(2, &[x, y, w, h, r, t], &[c]);
        self.cached(key, |p| {
            let s = Shape::new(x, y, w, h, r);
            p.ring_with(&s, -t, 0.0, None, &|_| c);
        });
    }

    fn flow_at(&self, s: &Shape, p: [f32; 2], a: f32) -> u32 {
        flow_color(s, p, a, &self.m, self.time, &self.palette)
    }

    pub fn flow(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, a: f32) {
        let a = (a * self.alpha).clamp(0.0, 1.0);
        if a <= 0.003 || w <= 0.0 || h <= 0.0 {
            return;
        }
        let key = self.key(3, &[x, y, w, h, r], &[]);
        let ops = match FLOW_CACHE.with(|c| c.borrow().get(&key).cloned()) {
            Some(ops) => ops,
            None => {
                let ops = Rc::new(self.flow_geom(x, y, w, h, r));
                FLOW_CACHE.with(|c| {
                    let mut c = c.borrow_mut();
                    if c.len() >= CACHE_MAX {
                        c.clear();
                    }
                    c.insert(key, ops.clone());
                });
                ops
            }
        };
        let s = Shape::new(x, y, w, h, r);
        for op in ops.iter() {
            match *op {
                FlowOp::Grad(rect, top, bottom) => {
                    let t = self.flow_at(&s, top, a);
                    let b = self.flow_at(&s, bottom, a);
                    self.grad(rect, t, b);
                }
                FlowOp::Quad(q, at, k) => {
                    let c = self.flow_at(&s, at, a * k);
                    if (c >> 24) != 0 {
                        match &mut self.rec {
                            Some(buf) => buf.push(Op::Quad(q, c)),
                            None => self.canvas.quad(q, c),
                        }
                    }
                }
            }
        }
    }

    fn flow_geom(&self, x: f32, y: f32, w: f32, h: f32, r: f32) -> Vec<FlowOp> {
        let mut ops = Vec::new();
        let m = self.m;
        let tr = |p: [f32; 2]| m.apply(p[0], p[1]);
        let cell = |ops: &mut Vec<FlowOp>, x0: f32, y0: f32, x1: f32, y1: f32| {
            let xc = (x0 + x1) * 0.5;
            let p0 = m.apply(x0, y0);
            let p1 = m.apply(x1, y1);
            ops.push(FlowOp::Grad(
                [p0[0], p0[1], p1[0] - p0[0], p1[1] - p0[1]],
                [xc, y0],
                [xc, y1],
            ));
        };
        let s = Shape::new(x, y, w, h, r);
        let lp = self.local_px();
        let cw = (lp * 7.0).max(0.25);
        let ch = (lp * 30.0).max(1.0);
        let n = self.arc_steps(s.r);
        let poly = s.outline(0.0, n, None);
        let mut area = 0.0;
        for i in 0..poly.len() {
            let p = poly[i];
            let q = poly[(i + 1) % poly.len()];
            area += p[0] * q[1] - q[0] * p[1];
        }
        let sign = if area >= 0.0 { 1.0 } else { -1.0 };
        let cols = ((w / cw).ceil() as usize).max(1);
        let rows = ((h / ch).ceil() as usize).max(1);
        let cwx = w / cols as f32;
        let chy = h / rows as f32;
        let half = s.half();
        let center = s.center();
        let inside =
            |p: [f32; 2]| sd_round_box([p[0] - center[0], p[1] - center[1]], half, s.r) <= 1e-4;
        for row in 0..rows {
            let y0 = y + chy * row as f32;
            let y1 = if row + 1 == rows { y + h } else { y0 + chy };
            for col in 0..cols {
                let x0 = x + cwx * col as f32;
                let x1 = if col + 1 == cols { x + w } else { x0 + cwx };
                let corners = [[x0, y0], [x1, y0], [x1, y1], [x0, y1]];
                if corners.iter().all(|&p| inside(p)) {
                    cell(&mut ops, x0, y0, x1, y1);
                    continue;
                }
                let subs = (((y1 - y0) / (lp * 3.0).max(0.25)).ceil() as usize).max(1);
                let sh = (y1 - y0) / subs as f32;
                for k in 0..subs {
                    let sy0 = y0 + sh * k as f32;
                    let sy1 = if k + 1 == subs { y1 } else { sy0 + sh };
                    let sub = [[x0, sy0], [x1, sy0], [x1, sy1], [x0, sy1]];
                    if sub.iter().all(|&p| inside(p)) {
                        cell(&mut ops, x0, sy0, x1, sy1);
                        continue;
                    }
                    let cxy = [(x0 + x1) * 0.5, (sy0 + sy1) * 0.5];
                    let diag = ((x1 - x0).powi(2) + (sy1 - sy0).powi(2)).sqrt() * 0.5;
                    if sd_round_box([cxy[0] - center[0], cxy[1] - center[1]], half, s.r) > diag {
                        continue;
                    }
                    let clipped = clip_poly(&sub, &poly, sign);
                    if clipped.len() < 3 {
                        continue;
                    }
                    let mut cen = [0.0f32, 0.0];
                    for p in &clipped {
                        cen[0] += p[0];
                        cen[1] += p[1];
                    }
                    cen[0] /= clipped.len() as f32;
                    cen[1] /= clipped.len() as f32;
                    let first = clipped[0];
                    let mut i = 1;
                    while i + 1 < clipped.len() {
                        let b = clipped[i];
                        let cc = clipped[i + 1];
                        if i + 2 < clipped.len() {
                            ops.push(FlowOp::Quad([tr(first), tr(b), tr(cc), tr(clipped[i + 2])], cen, 1.0));
                            i += 2;
                        } else {
                            ops.push(FlowOp::Quad([tr(first), tr(b), tr(cc), tr(cc)], cen, 1.0));
                            i += 1;
                        }
                    }
                }
            }
        }
        let f = lp * 0.75;
        let step = Some((lp * 16.0).max(1.0));
        let n = self.arc_steps(s.r);
        for (q, _) in ring_quads(&s, 0.0, f, n, step, &|_| 0) {
            let mid = [
                (q[0][0] + q[1][0] + q[2][0] + q[3][0]) * 0.25,
                (q[0][1] + q[1][1] + q[2][1] + q[3][1]) * 0.25,
            ];
            ops.push(FlowOp::Quad([tr(q[0]), tr(q[1]), tr(q[2]), tr(q[3])], mid, 0.45));
        }
        ops
    }

    #[allow(clippy::too_many_arguments)]
    pub fn flow_glow(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, spread: f32, alpha: f32) {
        if !self.glow || alpha <= 0.0 || w <= 0.0 || h <= 0.0 {
            return;
        }
        let s = Shape::new(x, y, w, h, r);
        let (m, t, pal) = (self.m, self.time, self.palette);
        self.glow_sprites(&s, spread, &|p| flow_color(&s, p, alpha, &m, t, &pal));
    }

    #[allow(clippy::too_many_arguments)]
    pub fn glow(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, spread: f32, c: u32) {
        let alpha = ((c >> 24) & 0xFF) as f32 / 255.0 * self.alpha;
        if !self.glow || alpha <= 0.0 || w <= 0.0 || h <= 0.0 {
            return;
        }
        let s = Shape::new(x, y, w, h, r);
        let col = (((alpha * 255.0).round() as u32).min(255) << 24) | (c & 0x00FF_FFFF);
        self.glow_sprites(&s, spread, &|_| col);
    }

    pub fn vgrad(&mut self, x: f32, y: f32, w: f32, h: f32, top: u32, bottom: u32) {
        let p0 = self.m.apply(x, y);
        let p1 = self.m.apply(x + w, y + h);
        let top = self.col(top);
        let bottom = self.col(bottom);
        self.canvas
            .gradient([p0[0], p0[1], p1[0] - p0[0], p1[1] - p0[1]], top, bottom);
    }

    pub fn text(&mut self, s: &str, x: f32, y: f32, size: f32, c: u32) -> f32 {
        let w = text_width(s, size);
        let c = self.col(c);
        if s.is_empty() || (c >> 24) == 0 {
            return w;
        }
        let o = self.m.apply(x, y);
        let sz = size * self.m.factor();
        let font = font_for(sz / self.px);
        let k = sz / FONT_SIZE;
        let baseline = o[1] + metrics::ASCENT * sz;
        self.canvas
            .text(font, s, [o[0], baseline - FONT_BASELINE * k], k, c);
        w
    }

    pub fn icon(&mut self, ico: Icon, x: f32, y: f32, size: f32, c: u32) {
        let c = self.col(c);
        if (c >> 24) == 0 {
            return;
        }
        let k = (size / ico.size[0]).min(size / ico.size[1]);
        let w = ico.size[0] * k;
        let h = ico.size[1] * k;
        let dx = x + (size - w) * 0.5;
        let dy = y + (size - h) * 0.5;
        let p0 = self.m.apply(dx, dy);
        let p1 = self.m.apply(dx + w, dy + h);
        self.canvas.sprite(
            ico.sprite,
            ico.size,
            [p0[0], p0[1], p1[0] - p0[0], p1[1] - p0[1]],
            c,
        );
    }

    pub fn chevron(&mut self, x: f32, y: f32, size: f32, c: u32) {
        let c = self.col(c);
        let tri = [[0.14f32, 0.30f32], [0.86, 0.30], [0.50, 0.74]];
        let d = 0.05;
        let mut out = [[0.0f32; 2]; 3];
        for i in 0..3 {
            let prev = tri[(i + 2) % 3];
            let cur = tri[i];
            let next = tri[(i + 1) % 3];
            let n1 = outward(prev, cur, &tri);
            let n2 = outward(cur, next, &tri);
            let dot = n1[0] * n2[0] + n1[1] * n2[1];
            let k = d / (1.0 + dot).max(0.2);
            out[i] = [cur[0] + (n1[0] + n2[0]) * k, cur[1] + (n1[1] + n2[1]) * k];
        }
        let p = |v: [f32; 2]| [x + v[0] * size, y + v[1] * size];
        self.tq([p(out[0]), p(out[1]), p(out[2]), p(out[2])], c);
    }

    pub fn push_clip(&mut self, x1: f32, y1: f32, x2: f32, y2: f32) {
        let a = self.m.apply(x1, y1);
        let b = self.m.apply(x2, y2);
        let l = a[0].min(b[0]).floor();
        let t = a[1].min(b[1]).floor();
        let r = a[0].max(b[0]).ceil();
        let bt = a[1].max(b[1]).ceil();
        self.canvas
            .push_clip([l, t, (r - l).max(0.0), (bt - t).max(0.0)]);
    }

    pub fn pop_clip(&mut self) {
        self.canvas.pop_clip();
    }
}

fn flow_color(s: &Shape, p: [f32; 2], a: f32, m: &Affine, t: f32, pal: &[[f32; 3]; 3]) -> u32 {
    let half = s.half();
    let c = s.center();
    let n = [
        (p[0] - c[0]) / half[0].max(1.0),
        (p[1] - c[1]) / half[1].max(1.0),
    ];
    let sc = m.apply(c[0], c[1]);
    argb(flow_rgb(n, sc, t, pal), a)
}

fn ring_quads(
    s: &Shape,
    g_in: f32,
    g_out: f32,
    n: usize,
    step: Option<f32>,
    color: &dyn Fn([f32; 2]) -> u32,
) -> Vec<([[f32; 2]; 4], u32)> {
    let inner = s.outline(g_in, n, step);
    let outer = s.outline(g_out, n, step);
    let len = inner.len().min(outer.len());
    let mut out = Vec::with_capacity(len);
    for i in 0..len {
        let j = (i + 1) % len;
        let mid = [
            (inner[i][0] + inner[j][0] + outer[i][0] + outer[j][0]) * 0.25,
            (inner[i][1] + inner[j][1] + outer[i][1] + outer[j][1]) * 0.25,
        ];
        out.push(([outer[i], outer[j], inner[j], inner[i]], color(mid)));
    }
    out
}

fn outward(a: [f32; 2], b: [f32; 2], tri: &[[f32; 2]; 3]) -> [f32; 2] {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let len = (dx * dx + dy * dy).sqrt().max(1e-6);
    let mut n = [dy / len, -dx / len];
    let cx = (tri[0][0] + tri[1][0] + tri[2][0]) / 3.0;
    let cy = (tri[0][1] + tri[1][1] + tri[2][1]) / 3.0;
    let mx = (a[0] + b[0]) * 0.5;
    let my = (a[1] + b[1]) * 0.5;
    if (mx - cx) * n[0] + (my - cy) * n[1] < 0.0 {
        n = [-n[0], -n[1]];
    }
    n
}

fn glow_axis(lo: f32, hi: f32, e: f32, spread: f32, seg: f32, fine: bool) -> Vec<(f32, f32, f32, f32, bool)> {
    let mut out = Vec::new();
    let r = e - spread;
    let corner = |out: &mut Vec<(f32, f32, f32, f32, bool)>, start: f32, flip: bool| {
        let mut cuts = vec![0.0f32];
        if fine && r > 0.01 {
            let (a, b) = if flip { (0.0, r) } else { (spread, e) };
            if !flip {
                cuts.push(spread);
            }
            let k = (r / 4.0).ceil().max(1.0) as usize;
            for i in 1..k {
                cuts.push(a + (b - a) * i as f32 / k as f32);
            }
            if flip {
                cuts.push(r);
            }
        }
        cuts.push(e);
        cuts.sort_by(f32::total_cmp);
        cuts.dedup_by(|x, y| (*x - *y).abs() < 1e-4);
        let base = if flip { 64.0 } else { 0.0 };
        for w in cuts.windows(2) {
            let (t0, t1) = (w[0] / e, w[1] / e);
            out.push((start + w[0], start + w[1], base + 64.0 * t0, 64.0 * (t1 - t0), false));
        }
    };
    corner(&mut out, lo, false);
    let len = hi - lo - 2.0 * e;
    if len > 0.01 {
        let k = (len / seg).ceil().max(1.0) as usize;
        let w = len / k as f32;
        for i in 0..k {
            let a = lo + e + w * i as f32;
            out.push((a, a + w, 63.25, 1.5, true));
        }
    }
    corner(&mut out, hi - e, true);
    out
}
