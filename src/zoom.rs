use crate::registry::Registry;
use mod_api::{CalculatePlayerTurnEvent, ClientHost, MouseScrollingEvent, Zoom as ZoomView};
use std::collections::HashSet;

pub struct Zoom {
    pub module: usize,
    activation: usize,
    amount: usize,
    smooth: usize,
    mouse: usize,
    scroll: usize,
    extra: f32,
}

impl Zoom {
    pub fn register(reg: &mut Registry) -> Zoom {
        let module = reg.add_module(4, "Zoom", "Zooms the camera in while active", false);
        reg.modules[module].key = Some("key.keyboard.c".to_owned());
        let activation = reg.mode("Activation", 1, &["Toggle", "Hold"]);
        let amount = reg.slider("Zoom amount", 75.0, 10.0, 95.0, 1.0, "%");
        let smooth = reg.boolean("Smooth transition", true);
        let mouse = reg.mode(
            "Mouse while zoomed",
            1,
            &["Normal speed", "Reduced speed", "Cinematic smoothing"],
        );
        let scroll = reg.boolean("Scroll to adjust", true);
        reg.attach(module, &[activation, amount, smooth, mouse, scroll]);
        Zoom {
            module,
            activation,
            amount,
            smooth,
            mouse,
            scroll,
            extra: 1.0,
        }
    }

    pub fn tick(&mut self, reg: &mut Registry, host: &dyn ClientHost, mouse: &HashSet<String>) {
        if reg.get_mode(self.activation) == 1 {
            let down = host.playing()
                && host.mod_screen().is_none()
                && match reg.modules[self.module].key.as_deref() {
                    Some(k) if k.starts_with("key.mouse.") => mouse.contains(k),
                    Some(k) => host.is_key_down(k),
                    None => false,
                };
            if reg.modules[self.module].key.is_some() {
                reg.modules[self.module].enabled = down;
            }
        }
        if !reg.modules[self.module].enabled {
            self.extra = 1.0;
        }
    }

    fn factor(&self, reg: &Registry) -> f32 {
        let pct = reg.get_slider(self.amount).clamp(10.0, 95.0) / 100.0;
        (1.0 / (1.0 - pct) * self.extra).clamp(1.0, 50.0)
    }

    pub fn view(&self, reg: &Registry) -> Option<ZoomView> {
        if !reg.modules[self.module].enabled {
            return None;
        }
        let factor = self.factor(reg);
        let turn = if reg.get_mode(self.mouse) == 1 {
            1.0 / factor
        } else {
            1.0
        };
        Some(ZoomView {
            factor,
            smooth: reg.get_bool(self.smooth),
            turn,
        })
    }

    pub fn player_turn(&self, reg: &Registry, event: &mut CalculatePlayerTurnEvent) {
        if reg.modules[self.module].enabled && reg.get_mode(self.mouse) == 2 {
            event.cinematic_camera_enabled = true;
        }
    }

    pub fn scroll(&mut self, reg: &Registry, event: &MouseScrollingEvent) -> bool {
        if !reg.modules[self.module].enabled || !reg.get_bool(self.scroll) {
            return false;
        }
        let dy = event.scroll[1];
        if dy == 0.0 {
            return false;
        }
        let step = if dy > 0.0 { 1.25 } else { 0.8 };
        let base = self.factor(reg) / self.extra;
        self.extra = (self.extra * step).clamp(1.0 / base, 50.0 / base);
        true
    }
}
