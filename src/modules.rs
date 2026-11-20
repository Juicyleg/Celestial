use crate::registry::Registry;
use mod_api::client::camera::CameraType;
use mod_api::{
    ClientHost, ClipBlock, ClipFluid, Identifier, MovementInputEvent, ServerboundPlayPacket,
};
use std::collections::HashSet;

pub struct Sprint {
    pub module: usize,
}

impl Sprint {
    pub fn register(reg: &mut Registry) -> Sprint {
        let module = reg.add_module(1, "Sprint", "Sprints automatically while moving", false);
        Sprint { module }
    }

    pub fn movement_input(&self, reg: &Registry, event: &mut MovementInputEvent) {
        if reg.modules[self.module].enabled && event.forward > 0.0 {
            event.sprint = true;
        }
    }
}

pub struct AutoRespawn {
    pub module: usize,
    ticks: u32,
}

impl AutoRespawn {
    pub fn register(reg: &mut Registry) -> AutoRespawn {
        let module = reg.add_module(
            3,
            "Auto Respawn",
            "Respawns the player automatically after death",
            false,
        );
        AutoRespawn { module, ticks: 0 }
    }

    pub fn tick(&mut self, reg: &Registry, host: &mut dyn ClientHost) {
        let dead = host
            .observed_screen_id()
            .is_some_and(|s| s.id == "death");
        if !reg.modules[self.module].enabled || !dead {
            self.ticks = 0;
            return;
        }
        self.ticks += 1;
        if self.ticks >= 5
            && (self.ticks - 5).is_multiple_of(40)
            && let Ok(id) = Identifier::parse("minecraft:client_command")
        {
            let _ = host.send_packet(ServerboundPlayPacket::new(id, vec![0]));
        }
    }
}

pub struct ThirdPerson {
    pub module: usize,
    pub mode: usize,
    pub key: usize,
    pub active: bool,
    held: [f32; 2],
    free: [f32; 2],
    was_down: bool,
    prev_pos: Option<[f64; 3]>,
    cur_pos: Option<[f64; 3]>,
}

fn wrap(v: f32) -> f32 {
    let mut f = v % 360.0;
    if f >= 180.0 {
        f -= 360.0;
    }
    if f < -180.0 {
        f += 360.0;
    }
    f
}

fn rotation(host: &dyn ClientHost) -> Option<[f32; 2]> {
    let me = host.local_player()?.entity();
    me.rotation(host)
}

impl ThirdPerson {
    pub fn register(reg: &mut Registry) -> ThirdPerson {
        let module = reg.add_module(
            3,
            "Third Person",
            "Free third person look without changing movement direction",
            false,
        );
        let mode = reg.mode("Look activation", 0, &["On press", "On hold"]);
        let key = reg.bind("Look key", Some("key.keyboard.left.alt"));
        reg.attach(module, &[mode, key]);
        ThirdPerson {
            module,
            mode,
            key,
            active: false,
            held: [0.0; 2],
            free: [0.0; 2],
            was_down: false,
            prev_pos: None,
            cur_pos: None,
        }
    }

    fn set(&mut self, host: &mut dyn ClientHost, on: bool) {
        if on {
            let Some(rot) = rotation(host) else {
                return;
            };
            self.held = rot;
            self.free = rot;
            host.set_camera_type(CameraType::ThirdPersonBack);
        } else if self.active {
            self.free = self.held;
            host.set_camera_type(CameraType::FirstPerson);
        }
        self.active = on;
    }

    pub fn tick(&mut self, reg: &Registry, host: &mut dyn ClientHost, mouse: &HashSet<String>) {
        let enabled = reg.modules[self.module].enabled;
        let playing = host.playing() && host.mod_screen().is_none();
        let down = match reg.get_bind(self.key) {
            Some(k) if k.starts_with("key.mouse.") => mouse.contains(k),
            Some(k) => host.is_key_down(k),
            None => false,
        };
        let hold = reg.get_mode(self.mode) == 1;
        if enabled && playing {
            if down && !self.was_down {
                if hold {
                    self.set(host, true);
                } else {
                    let next = !self.active;
                    self.set(host, next);
                }
            } else if !down && self.was_down && self.active && hold {
                self.set(host, false);
            }
        }
        self.was_down = down;
        if self.active && (!enabled || !playing) {
            self.set(host, false);
        }
        if self.active {
            if let Some(rot) = rotation(host) {
                self.free[0] += wrap(rot[0] - self.held[0]);
                self.free[1] = (self.free[1] + rot[1] - self.held[1]).clamp(-90.0, 90.0);
            }
            let _ = host.set_local_rotation(self.held[0], self.held[1].clamp(-89.0, 89.0));
        }
    }

    pub fn tick_post(&mut self, host: &dyn ClientHost) {
        let pos = host
            .local_player()
            .and_then(|p| p.entity().position(host));
        self.prev_pos = self.cur_pos.or(pos);
        self.cur_pos = pos;
    }

    pub fn camera_pose(
        &self,
        host: &dyn ClientHost,
        partial: f32,
        position: &mut [f64; 3],
        yaw: &mut f32,
        pitch: &mut f32,
    ) {
        if !self.active {
            return;
        }
        let Some(rot) = rotation(host) else {
            return;
        };
        let ny = self.free[0] + wrap(rot[0] - self.held[0]);
        let np = (self.free[1] + rot[1] - self.held[1]).clamp(-90.0, 90.0);
        let Some(player) = host.local_player() else {
            return;
        };
        let me = player.entity();
        let (Some(cur), Some(eye_now)) = (me.position(host), me.eye_position(host)) else {
            return;
        };
        let prev = self.prev_pos.unwrap_or(cur);
        let base = self.cur_pos.unwrap_or(cur);
        let t = partial as f64;
        let eye_h = eye_now[1] - cur[1];
        let eye = [
            prev[0] + (base[0] - prev[0]) * t,
            prev[1] + (base[1] - prev[1]) * t + eye_h,
            prev[2] + (base[2] - prev[2]) * t,
        ];
        let (yr, pr) = ((ny as f64).to_radians(), (np as f64).to_radians());
        let fwd = [-yr.sin() * pr.cos(), -pr.sin(), yr.cos() * pr.cos()];
        let mut dist = 4.0f64;
        let level = host
            .camera()
            .and_then(|c| host.level(&c.dimension));
        if let Some(level) = level {
            for i in 0..8 {
                let off = [
                    if i & 1 == 0 { -0.1 } else { 0.1 },
                    if i & 2 == 0 { -0.1 } else { 0.1 },
                    if i & 4 == 0 { -0.1 } else { 0.1 },
                ];
                let from = [eye[0] + off[0], eye[1] + off[1], eye[2] + off[2]];
                let to = [
                    from[0] - fwd[0] * dist,
                    from[1] - fwd[1] * dist,
                    from[2] - fwd[2] * dist,
                ];
                if let Some(hit) = level.clip(host, from, to, ClipBlock::Outline, ClipFluid::None)
                    && !hit.miss
                {
                    let d = ((hit.location[0] - eye[0]).powi(2)
                        + (hit.location[1] - eye[1]).powi(2)
                        + (hit.location[2] - eye[2]).powi(2))
                    .sqrt();
                    if d < dist {
                        dist = d;
                    }
                }
            }
        }
        *position = [
            eye[0] - fwd[0] * dist,
            eye[1] - fwd[1] * dist,
            eye[2] - fwd[2] * dist,
        ];
        *yaw = ny;
        *pitch = np;
    }
}

pub fn local_clock() -> String {
    let (h, m, sec) = local_hm();
    let (h12, ap) = match h {
        0 => (12, "AM"),
        1..=11 => (h, "AM"),
        12 => (12, "PM"),
        _ => (h - 12, "PM"),
    };
    format!("{h12:02}:{m:02}:{sec:02} {ap}")
}

#[cfg(windows)]
fn local_hm() -> (u32, u32, u32) {
    #[repr(C)]
    struct SystemTime {
        year: u16,
        month: u16,
        day_of_week: u16,
        day: u16,
        hour: u16,
        minute: u16,
        second: u16,
        millis: u16,
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetLocalTime(out: *mut SystemTime);
    }
    let mut st = SystemTime {
        year: 0,
        month: 0,
        day_of_week: 0,
        day: 0,
        hour: 0,
        minute: 0,
        second: 0,
        millis: 0,
    };
    unsafe { GetLocalTime(&mut st) };
    (st.hour as u32, st.minute as u32, st.second as u32)
}

#[cfg(unix)]
fn local_hm() -> (u32, u32, u32) {
    unsafe extern "C" {
        fn localtime_r(t: *const i64, out: *mut i32) -> *mut i32;
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let mut tm = [0i64; 8];
    let ok = unsafe { localtime_r(&now, tm.as_mut_ptr().cast::<i32>()) };
    if ok.is_null() {
        let s = now.rem_euclid(86_400);
        return ((s / 3600) as u32, ((s / 60) % 60) as u32, (s % 60) as u32);
    }
    let ints = tm.as_ptr().cast::<i32>();
    let (hour, min, sec) = unsafe { (*ints.add(2), *ints.add(1), *ints) };
    (
        hour.clamp(0, 23) as u32,
        min.clamp(0, 59) as u32,
        sec.clamp(0, 59) as u32,
    )
}

#[cfg(not(any(windows, unix)))]
fn local_hm() -> (u32, u32, u32) {
    let s = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
        % 86_400;
    ((s / 3600) as u32, ((s / 60) % 60) as u32, (s % 60) as u32)
}
