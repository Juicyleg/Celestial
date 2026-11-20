use crate::paint::{Theme, hsb};
use crate::registry::Registry;
use mod_api::client::hud_layout::HeartKind;
use mod_api::{
    BlockEntityDrawContext, BlockScreenEffect, ClientHost, EntityRenderLayer, EntityRenderState,
    ExtractBlockScreenEffectEvent, Flow, Hand, Identifier, ItemDisplayContext, ItemStack,
    ModelRef, PosedParts, RenderArmEvent, RenderHandEvent,
};
use std::sync::Mutex;

static SKINS: Mutex<Vec<(u32, bool)>> = Mutex::new(Vec::new());

pub struct SkinProbe {
    pub slim: bool,
}

impl EntityRenderLayer for SkinProbe {
    fn submit(
        &self,
        entity: &EntityRenderState,
        _parts: Option<&PosedParts>,
        _draw: &mut dyn BlockEntityDrawContext,
    ) -> Result<(), String> {
        let mut skins = SKINS.lock().unwrap_or_else(|e| e.into_inner());
        match skins.iter_mut().find(|(id, _)| *id == entity.id) {
            Some(entry) => entry.1 = self.slim,
            None => {
                if skins.len() >= 256 {
                    skins.clear();
                }
                skins.push((entity.id, self.slim));
            }
        }
        Ok(())
    }
}

fn skin_slim(id: u32) -> Option<bool> {
    let skins = SKINS.lock().unwrap_or_else(|e| e.into_inner());
    skins.iter().find(|(i, _)| *i == id).map(|(_, slim)| *slim)
}

fn plain_held<'a>(
    host: &dyn ClientHost,
    event: &RenderHandEvent<'_, 'a>,
) -> Option<(&'a ItemStack, bool)> {
    let stack = event.render.stack?;
    let id = stack.item().id(host);
    if id == "minecraft:filled_map" || id == "minecraft:crossbow" || id.ends_with("_spear") {
        return None;
    }
    let hand = event.render.hand;
    if host
        .local_item_use()
        .is_some_and(|u| u.hand == hand && u.ticks_remaining > 0)
    {
        return None;
    }
    let left = host.gui_options().main_hand_left != (hand == Hand::Off);
    Some((stack, left))
}

fn draw_held(draw: &mut dyn BlockEntityDrawContext, stack: &ItemStack, left: bool, seed: i32) -> bool {
    let context = if left {
        ItemDisplayContext::FirstPersonLeftHand
    } else {
        ItemDisplayContext::FirstPersonRightHand
    };
    draw.item(stack, context, None, seed)
}

fn rot(draw: &mut dyn BlockEntityDrawContext, axis: [f32; 3], degrees: f32) {
    draw.rotate_about(axis, degrees, [0.0; 3]);
}

const X: [f32; 3] = [1.0, 0.0, 0.0];
const Y: [f32; 3] = [0.0, 1.0, 0.0];
const Z: [f32; 3] = [0.0, 0.0, 1.0];

pub struct ThemeModule {
    pub module: usize,
    color: usize,
    hue: usize,
    sat: usize,
    bri: usize,
    speed: usize,
    blur: usize,
    glow: usize,
}

impl ThemeModule {
    pub fn register(reg: &mut Registry) -> ThemeModule {
        let module = reg.add_module(
            6,
            "ClickGUI",
            "Menu and HUD theme: colour, speed, blur and glow",
            true,
        );
        reg.set_role(module, crate::registry::Role::Static);
        let s0 = reg.section("Colour");
        let color = reg.mode("Colour", 0, &["Client", "Custom"]);
        let hue = reg.slider("First colour", 262.0, 0.0, 360.0, 1.0, "°");
        let sat = reg.slider("Saturation", 51.0, 0.0, 100.0, 1.0, "%");
        let bri = reg.slider("Brightness", 95.0, 30.0, 100.0, 1.0, "%");
        let speed = reg.slider("Colour speed", 1.0, 0.2, 3.0, 0.1, "x");
        let s1 = reg.section("Effects");
        let blur = reg.boolean("Blur", true);
        let glow = reg.boolean("Glow", true);
        reg.attach(module, &[s0, color, hue, sat, bri, speed, s1, blur, glow]);
        ThemeModule {
            module,
            color,
            hue,
            sat,
            bri,
            speed,
            blur,
            glow,
        }
    }

    pub fn apply(&self, reg: &Registry, theme: &mut Theme) {
        if !reg.modules[self.module].enabled {
            *theme = Theme::default();
            return;
        }
        theme.custom = reg.get_mode(self.color) == 1;
        theme.hue = reg.get_slider(self.hue);
        theme.sat = reg.get_slider(self.sat) / 100.0;
        theme.bri = reg.get_slider(self.bri) / 100.0;
        theme.speed = reg.get_slider(self.speed);
        theme.blur = reg.get_bool(self.blur);
        theme.glow = reg.get_bool(self.glow);
    }
}

pub struct ViewModel {
    pub module: usize,
    main: [usize; 3],
    off: [usize; 3],
    main_size: [usize; 3],
    off_size: [usize; 3],
}

impl ViewModel {
    pub fn register(reg: &mut Registry) -> ViewModel {
        let module = reg.add_module(
            2,
            "View Model",
            "Changes the position and size of the items in your hands",
            false,
        );
        let main = [
            reg.slider("Main hand X", 0.0, -2.0, 2.0, 0.1, ""),
            reg.slider("Main hand Y", 0.0, -2.0, 2.0, 0.1, ""),
            reg.slider("Main hand Z", 0.0, -2.0, 2.0, 0.1, ""),
        ];
        let off = [
            reg.slider("Off hand X", 0.0, -2.0, 2.0, 0.1, ""),
            reg.slider("Off hand Y", 0.0, -2.0, 2.0, 0.1, ""),
            reg.slider("Off hand Z", 0.0, -2.0, 2.0, 0.1, ""),
        ];
        let main_size = [
            reg.slider("Main hand size X", 1.0, 0.1, 2.0, 0.05, "x"),
            reg.slider("Main hand size Y", 1.0, 0.1, 2.0, 0.05, "x"),
            reg.slider("Main hand size Z", 1.0, 0.1, 2.0, 0.05, "x"),
        ];
        let off_size = [
            reg.slider("Off hand size X", 1.0, 0.1, 2.0, 0.05, "x"),
            reg.slider("Off hand size Y", 1.0, 0.1, 2.0, 0.05, "x"),
            reg.slider("Off hand size Z", 1.0, 0.1, 2.0, 0.05, "x"),
        ];
        let positions = [main[0], main[1], main[2], off[0], off[1], off[2]];
        let sizes = [
            main_size[0],
            main_size[1],
            main_size[2],
            off_size[0],
            off_size[1],
            off_size[2],
        ];
        let reset = reg.button(
            "Reset positions",
            Some(Box::new(move |r: &mut Registry| {
                for s in positions {
                    r.set_slider(s, 0.0);
                }
            })),
        );
        let reset_size = reg.button(
            "Reset sizes",
            Some(Box::new(move |r: &mut Registry| {
                for s in sizes {
                    r.set_slider(s, 1.0);
                }
            })),
        );
        let s0 = reg.section("Position");
        let s1 = reg.section("Size");
        reg.attach(
            module,
            &[
                s0,
                main[0],
                main[1],
                main[2],
                off[0],
                off[1],
                off[2],
                reset,
                s1,
                main_size[0],
                main_size[1],
                main_size[2],
                off_size[0],
                off_size[1],
                off_size[2],
                reset_size,
            ],
        );
        ViewModel {
            module,
            main,
            off,
            main_size,
            off_size,
        }
    }

    pub fn offset(&self, reg: &Registry, hand: Hand) -> [f32; 3] {
        if !reg.modules[self.module].enabled {
            return [0.0; 3];
        }
        let ids = if hand == Hand::Main { self.main } else { self.off };
        ids.map(|s| reg.get_slider(s))
    }

    pub fn size(&self, reg: &Registry, hand: Hand) -> [f32; 3] {
        if !reg.modules[self.module].enabled {
            return [1.0; 3];
        }
        let ids = if hand == Hand::Main {
            self.main_size
        } else {
            self.off_size
        };
        ids.map(|s| reg.get_slider(s))
    }

    pub fn redraw_plain(
        &self,
        host: &dyn ClientHost,
        offset: [f32; 3],
        size: [f32; 3],
        event: &mut RenderHandEvent<'_, '_>,
    ) -> bool {
        let Some((stack, left)) = plain_held(host, event) else {
            return false;
        };
        let i = if left { -1.0 } else { 1.0 };
        let a = event.render.swing;
        let pi = std::f32::consts::PI;
        let root = a.sqrt();
        let equip = event.render.equip;
        let seed = event.render.player_entity_id + if left { 3 } else { 4 };
        let draw = &mut *event.draw;
        draw.push();
        draw.translate(offset);
        draw.scale(size);
        draw.translate([i * 0.56, -0.52 - equip * 0.6, -0.72]);
        draw.translate([
            i * -0.4 * (root * pi).sin(),
            0.2 * (root * pi * 2.0).sin(),
            -0.2 * (a * pi).sin(),
        ]);
        rot(draw, Y, i * (45.0 - (a * a * pi).sin() * 20.0));
        rot(draw, Z, i * (root * pi).sin() * -20.0);
        rot(draw, X, (root * pi).sin() * -80.0);
        rot(draw, Y, i * -45.0);
        let drawn = draw_held(draw, stack, left, seed);
        draw.pop();
        drawn
    }
}

pub struct SwingAnimation {
    pub module: usize,
    mode: usize,
    angle: usize,
    tilt: usize,
    power: usize,
    last_item: std::cell::Cell<Option<mod_api::ItemKey>>,
    switching: std::cell::Cell<bool>,
}

impl SwingAnimation {
    pub fn register(reg: &mut Registry) -> SwingAnimation {
        let module = reg.add_module(2, "Swing Animation", "Changes the hand swing animation", false);
        let mode = reg.mode(
            "Animation mode",
            0,
            &["Mode 1", "Mode 2", "Mode 3", "Mode 4", "Mode 5"],
        );
        let angle = reg.slider("Rotation angle", 75.0, 0.0, 360.0, 1.0, "");
        let tilt = reg.slider("Tip tilt", -20.0, -90.0, 90.0, 1.0, "");
        let power = reg.slider("Swing intensity", 5.0, 1.0, 10.0, 1.0, "");
        reg.show_if(angle, Box::new(move |r: &Registry| r.get_mode(mode) == 0));
        reg.show_if(tilt, Box::new(move |r: &Registry| r.get_mode(mode) != 4));
        reg.attach(module, &[mode, angle, tilt, power]);
        SwingAnimation {
            module,
            mode,
            angle,
            tilt,
            power,
            last_item: std::cell::Cell::new(None),
            switching: std::cell::Cell::new(false),
        }
    }

    fn equip_dip(&self, item: mod_api::ItemKey, equip: f32) -> f32 {
        if self.last_item.replace(Some(item)) != Some(item) {
            self.switching.set(true);
        }
        if equip <= 0.001 {
            self.switching.set(false);
        }
        if self.switching.get() { equip } else { 0.0 }
    }

    pub fn render_hand(
        &self,
        host: &dyn ClientHost,
        reg: &Registry,
        offset: [f32; 3],
        size: [f32; 3],
        event: &mut RenderHandEvent<'_, '_>,
    ) -> bool {
        if !reg.modules[self.module].enabled || event.render.hand != Hand::Main {
            return false;
        }
        let Some((stack, left)) = plain_held(host, event) else {
            return false;
        };
        let arm = if left { -1.0 } else { 1.0 };
        let mode = reg.get_mode(self.mode);
        let sp = event.render.swing;
        let anim = (sp * std::f32::consts::PI).sin();
        let power = reg.get_slider(self.power) * 10.0;
        let tilt = reg.get_slider(self.tilt);
        let angle = reg.get_slider(self.angle);
        let equip = self.equip_dip(stack.item(), event.render.equip);
        let seed = event.render.player_entity_id + if left { 3 } else { 4 };
        let draw = &mut *event.draw;
        draw.push();
        draw.translate(offset);
        draw.scale(size);
        draw.translate([arm * 0.56, -0.52 - equip * 0.6, -0.72]);
        if mode == 4 {
            draw.translate([arm * 0.5, -0.5, -0.72]);
        } else {
            draw.translate([arm * 0.72, -0.5, -1.0]);
            rot(draw, X, -tilt);
        }
        match mode {
            0 => {
                rot(draw, Y, arm * 90.0);
                rot(draw, Z, arm * -70.0);
                rot(draw, X, -angle - power * anim);
            }
            1 => {
                rot(draw, Y, arm * 90.0);
                rot(draw, Z, arm * -65.0);
                rot(draw, X, -65.0 + power * anim);
            }
            2 => {
                rot(draw, Y, arm * -90.0);
                rot(draw, Z, arm * 60.0);
                rot(draw, X, 30.0);
                rot(draw, Z, arm * power * anim);
            }
            3 => {
                rot(draw, Y, arm * 90.0);
                rot(draw, Z, arm * -75.0);
                rot(draw, X, -45.0 - power * anim);
                rot(draw, Y, arm * power * anim * 0.5);
            }
            _ => {
                let strength = power / 80.0;
                let swing = anim * anim;
                let twist = (sp * sp * std::f32::consts::PI).sin();
                rot(draw, Y, arm * (45.0 + twist * -20.0 * strength));
                rot(draw, Z, arm * swing * -22.0 * strength);
                rot(draw, X, swing * -85.0 * strength);
                rot(draw, Y, arm * -45.0);
            }
        }
        let drawn = draw_held(draw, stack, left, seed);
        draw.pop();
        drawn
    }
}

pub struct HandsShader {
    pub module: usize,
    opacity: usize,
    slim: usize,
    model: Option<ModelRef>,
}

impl HandsShader {
    pub fn register(reg: &mut Registry) -> HandsShader {
        let module = reg.add_module(
            2,
            "Hands Shader",
            "Lays an animated shader over the first person arm",
            false,
        );
        let opacity = reg.slider("Opacity", 0.6, 0.0, 1.0, 0.05, "");
        let slim = reg.mode("Arm model", 0, &["Auto", "Wide", "Slim"]);
        reg.attach(module, &[opacity, slim]);
        HandsShader {
            module,
            opacity,
            slim,
            model: None,
        }
    }

    pub fn resolve(&mut self, host: &dyn ClientHost) {
        self.model = Identifier::parse("client:hand_overlay")
            .ok()
            .and_then(|id| host.render_assets().model(&id).ok());
    }

    pub fn render_arm(
        &mut self,
        host: &dyn ClientHost,
        reg: &Registry,
        theme: &Theme,
        event: &mut RenderArmEvent<'_>,
    ) {
        if !reg.modules[self.module].enabled {
            return;
        }
        if self.model.is_none() {
            self.resolve(host);
        }
        let Some(model) = self.model else {
            return;
        };
        let opacity = reg.get_slider(self.opacity);
        if opacity <= 0.0 {
            return;
        }
        let rgb = theme.palette()[1];
        let ch = |v: f32| ((v.clamp(0.0, 1.0) * 255.0).round() as u32) & 0xFF;
        let tint = (ch(opacity) << 24) | (ch(rgb[0]) << 16) | (ch(rgb[1]) << 8) | ch(rgb[2]);
        let left = event.left;
        let slim = match reg.get_mode(self.slim) {
            1 => false,
            2 => true,
            _ => skin_slim(event.player_entity_id as u32).unwrap_or(false),
        };
        let pivot = [if left { 5.0 } else { -5.0 }, if slim { 2.5 } else { 2.0 }, 0.0];
        let roll = if left { -0.1f32 } else { 0.1f32 }.to_degrees();
        let (x0, w) = match (left, slim) {
            (false, false) => (-3.0, 4.0),
            (false, true) => (-2.0, 3.0),
            (true, false) => (-1.0, 4.0),
            (true, true) => (-1.0, 3.0),
        };
        let pad = 0.35;
        let draw = &mut *event.draw;
        draw.push();
        draw.translate([pivot[0] / 16.0, pivot[1] / 16.0, pivot[2] / 16.0]);
        rot(draw, Z, roll);
        draw.translate([(x0 - pad) / 16.0, (-2.0 - pad) / 16.0, (-2.0 - pad) / 16.0]);
        draw.scale([
            (w + 2.0 * pad) / 4.0,
            (12.0 + 2.0 * pad) / 12.0,
            (4.0 + 2.0 * pad) / 4.0,
        ]);
        let _ = draw.model(model, tint);
        draw.pop();
    }
}

pub struct Ambience {
    pub module: usize,
    custom: usize,
    hue: usize,
    sat: usize,
    bri: usize,
}

impl Ambience {
    pub fn register(reg: &mut Registry) -> Ambience {
        let module = reg.add_module(
            2,
            "Ambience",
            "Changes the atmosphere of the game world",
            false,
        );
        let custom = reg.boolean("Custom fog", false);
        let hue = reg.slider("Fog hue", 0.0, 0.0, 360.0, 1.0, "°");
        let sat = reg.slider("Fog saturation", 0.0, 0.0, 100.0, 1.0, "%");
        let bri = reg.slider("Fog brightness", 100.0, 0.0, 100.0, 1.0, "%");
        for s in [hue, sat, bri] {
            reg.show_if(s, Box::new(move |r: &Registry| r.get_bool(custom)));
        }
        reg.attach(module, &[custom, hue, sat, bri]);
        Ambience {
            module,
            custom,
            hue,
            sat,
            bri,
        }
    }

    pub fn fog_color(&self, reg: &Registry, rgb: &mut [f32; 3]) {
        if reg.modules[self.module].enabled && reg.get_bool(self.custom) {
            *rgb = hsb(
                reg.get_slider(self.hue),
                reg.get_slider(self.sat) / 100.0,
                reg.get_slider(self.bri) / 100.0,
            );
        }
    }
}

pub struct Removals {
    pub module: usize,
    group: usize,
}

const SCOREBOARD: usize = 0;
const BOSS_BAR: usize = 1;
const FIRE: usize = 2;
const BLACK_HEARTS: usize = 3;
const WATER: usize = 4;
const GLOW: usize = 5;

impl Removals {
    pub fn register(reg: &mut Registry) -> Removals {
        let module = reg.add_module(
            2,
            "Removals",
            "Removes chosen visual effects and game elements",
            false,
        );
        let group = reg.multi(
            "Removed elements",
            &[
                ("Scoreboard", false),
                ("Boss bar", false),
                ("Fire", true),
                ("Black hearts", true),
                ("Underwater overlay", false),
                ("Glowing", true),
            ],
        );
        reg.attach(module, &[group]);
        Removals { module, group }
    }

    fn on(&self, reg: &Registry, i: usize) -> bool {
        reg.modules[self.module].enabled && reg.get_bool(reg.child(self.group, i))
    }

    pub fn gui_layer(&self, reg: &Registry, layer: &str) -> Flow {
        let hide = match layer {
            "minecraft:scoreboard_sidebar" => self.on(reg, SCOREBOARD),
            "minecraft:boss_overlay" => self.on(reg, BOSS_BAR),
            _ => false,
        };
        if hide { Flow::Cancel } else { Flow::Continue }
    }

    pub fn screen_effect(&self, reg: &Registry, event: &ExtractBlockScreenEffectEvent) -> Flow {
        let hide = match event.effect {
            BlockScreenEffect::Fire => self.on(reg, FIRE),
            BlockScreenEffect::Water => self.on(reg, WATER),
            _ => false,
        };
        if hide { Flow::Cancel } else { Flow::Continue }
    }

    pub fn heart(&self, reg: &Registry, kind: &mut HeartKind) {
        if self.on(reg, BLACK_HEARTS) && *kind == HeartKind::Withered {
            *kind = HeartKind::Normal;
        }
    }

    pub fn outline(&self, reg: &Registry, outline: &mut Option<u32>) {
        if self.on(reg, GLOW) {
            *outline = None;
        }
    }
}
