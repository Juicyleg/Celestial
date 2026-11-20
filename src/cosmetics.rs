use crate::paint::{argb, hsb};
use crate::registry::Registry;
use mod_api::client::camera::CameraType;
use mod_api::{ClientHost, EquipmentSlot, WorldDepth, WorldDrawContext, WorldQuad};
use std::f64::consts::TAU;

const SEGMENTS: usize = 180;

#[derive(Clone, Copy, Default)]
struct Pose {
    pos: [f64; 3],
    yaw: f32,
    pitch: f32,
}

pub struct ChinaHat {
    pub module: usize,
    hue: usize,
    sat: usize,
    bri: usize,
    prev: Option<Pose>,
    cur: Option<Pose>,
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

fn lerp_angle(a: f32, b: f32, t: f32) -> f32 {
    let mut d = (b - a) % 360.0;
    if d >= 180.0 {
        d -= 360.0;
    }
    if d < -180.0 {
        d += 360.0;
    }
    a + d * t
}

fn mix(a: u32, b: u32, t: f32) -> u32 {
    let ch = |c: u32, s: u32| ((c >> s) & 0xFF) as f32;
    let m = |s: u32| (ch(a, s) + (ch(b, s) - ch(a, s)) * t).round() as u32 & 0xFF;
    (m(24) << 24) | (m(16) << 16) | (m(8) << 8) | m(0)
}

impl ChinaHat {
    pub fn register(reg: &mut Registry) -> ChinaHat {
        let module = reg.add_module(2, "China Hat", "Draws a cone hat above your head", false);
        let hue = reg.slider("Hat hue", 229.0, 0.0, 360.0, 1.0, "°");
        let sat = reg.slider("Hat saturation", 48.0, 0.0, 100.0, 1.0, "%");
        let bri = reg.slider("Hat brightness", 100.0, 0.0, 100.0, 1.0, "%");
        reg.attach(module, &[hue, sat, bri]);
        ChinaHat {
            module,
            hue,
            sat,
            bri,
            prev: None,
            cur: None,
        }
    }

    pub fn tick_post(&mut self, host: &dyn ClientHost) {
        let pose = host.local_player().and_then(|p| {
            let e = p.entity();
            let pos = e.position(host)?;
            let rot = e.rotation(host)?;
            let yaw = e.head_yaw(host).unwrap_or(rot[0]);
            Some(Pose {
                pos,
                yaw,
                pitch: rot[1],
            })
        });
        self.prev = self.cur.or(pose);
        self.cur = pose;
    }

    pub fn draw(
        &self,
        reg: &Registry,
        host: &dyn ClientHost,
        partial: f32,
        draw: &mut dyn WorldDrawContext,
    ) {
        if !reg.modules[self.module].enabled || host.camera_type() == CameraType::FirstPerson {
            return;
        }
        let (Some(prev), Some(cur)) = (self.prev, self.cur) else {
            return;
        };
        let Some(player) = host.local_player() else {
            return;
        };
        let me = player.entity();
        let t = partial as f64;
        let pos = [
            lerp(prev.pos[0], cur.pos[0], t),
            lerp(prev.pos[1], cur.pos[1], t),
            lerp(prev.pos[2], cur.pos[2], t),
        ];
        let yaw = (lerp_angle(prev.yaw, cur.yaw, partial) as f64).to_radians();
        let pitch = (lerp_angle(prev.pitch, cur.pitch, partial) as f64).to_radians();
        let eye = me.eye_height(host).unwrap_or(1.62);
        let width = me.dimensions(host).map(|d| d[0]).unwrap_or(0.6);
        let helmet = host
            .living(me)
            .and_then(|l| l.equipment(host, EquipmentSlot::Head))
            .is_some_and(|s| s.count() > 0);
        let lift = if helmet { 0.31 } else { 0.25 };
        let origin = [pos[0], pos[1] + eye, pos[2]];
        let (sy, cy) = yaw.sin_cos();
        let (sp, cp) = pitch.sin_cos();
        let place = |x: f64, y: f64, z: f64| -> [f64; 3] {
            let y2 = y * cp - z * sp;
            let z2 = z * cp + y * sp;
            let right = [-cy, 0.0, -sy];
            let fwd = [-sy, 0.0, cy];
            [
                origin[0] + right[0] * x + fwd[0] * z2,
                origin[1] + y2,
                origin[2] + right[2] * x + fwd[2] * z2,
            ]
        };
        let base = argb(
            hsb(
                reg.get_slider(self.hue),
                reg.get_slider(self.sat) / 100.0,
                reg.get_slider(self.bri) / 100.0,
            ),
            1.0,
        );
        let dark = {
            let c = |s: u32| (((base >> s) & 0xFF) / 2) << s;
            0xFF00_0000 | c(16) | c(8) | c(0)
        };
        let millis = crate::gui::now_millis() as f64;
        let color = |i: usize, alpha: u32| -> u32 {
            let phase = ((i as f64 * 8.0 + millis / 10.0) % 360.0) / 360.0 * TAU * 3.0;
            let k = ((phase.sin() + 1.0) / 2.0) as f32;
            (mix(base, dark, k) & 0x00FF_FFFF) | (alpha << 24)
        };
        let ring = |r: f64, y: f64, i: usize| {
            let a = i as f64 / SEGMENTS as f64 * TAU;
            place(a.cos() * r, y, a.sin() * r)
        };
        let tip = place(0.0, lift + 0.3, 0.0);
        let mut quads = Vec::with_capacity(SEGMENTS * 4);
        for i in 0..SEGMENTS {
            let c = color(i, 0x96);
            let a = ring(width, lift, i);
            let b = ring(width, lift, i + 1);
            quads.push(WorldQuad::new([tip, a, b, tip], c, WorldDepth::ThroughWalls));
            quads.push(WorldQuad::new([tip, b, a, tip], c, WorldDepth::ThroughWalls));
            let ia = ring(width - 0.02, lift + 0.02 * 0.3 / width, i);
            let ib = ring(width - 0.02, lift + 0.02 * 0.3 / width, i + 1);
            let edge = color(i, 0xFF);
            quads.push(WorldQuad::new([ia, a, b, ib], edge, WorldDepth::ThroughWalls));
            quads.push(WorldQuad::new([ib, b, a, ia], edge, WorldDepth::ThroughWalls));
        }
        let _ = draw.quads(&quads);
    }
}
