mod config;
mod cosmetics;
mod gui;
mod hud;
mod inventory;
mod metrics;
mod modules;
mod paint;
mod player;
mod registry;
mod shulker;
mod visuals;
mod zoom;

use gui::{Frame, Gui, KeyResult};
use mod_api::client::hud_layout::PoseOp;
use mod_api::text::TextStyle;
use hud::{Hud, HudData, HudFrame, HudIds, Row};
use mod_api::{
    self, CameraPoseEvent, CameraTypeCycleEvent, ClientHost, ClientResourcesReloadedEvent,
    ClientSetupEvent, ClientTickPostEvent, ClientTickPreEvent, ComputeFogColorEvent, DrawContext,
    EntityOutlineEvent, ExtractBlockScreenEffectEvent, Flow, GuiDraw, GuiLayerPreEvent,
    GuiRenderPostEvent, Hooks, KeyRegistration, LayerTarget, ModelBakingCompletedEvent,
    MouseButtonPreEvent, PlayerModel, RegisterEntityLayersEvent,
    MovementInputEvent, PlayerHeartTypeEvent, Registrar, RenderArmEvent, RenderHandEvent,
    StandaloneModelsEvent, RegistrationContext, ScreenCharTypedPreEvent, ScreenClosingEvent,
    ScreenKeyPressedPreEvent, ScreenMouseButtonPressedPreEvent, ScreenMouseButtonReleasedPreEvent,
    ScreenMouseDraggedPreEvent, ScreenMouseScrolledPreEvent, ScreenRenderPostEvent,
    ScreenTransparentBackgroundEvent, SubmitCustomGeometryEvent, Text, EquipmentSlot,
    CalculatePlayerTurnEvent, MouseScrollingEvent, ZoomEvent,
};
use config::Configs;
use cosmetics::ChinaHat;
use player::{AutoEat, AutoTool, ElytraHelper};
use modules::{AutoRespawn, Sprint, ThirdPerson};
use paint::{Canvas, Theme};
use visuals::{Ambience, HandsShader, Removals, SwingAnimation, ThemeModule, ViewModel};
use registry::Registry;
use std::collections::HashSet;
use std::sync::{Arc, Mutex, MutexGuard, TryLockError};

const ID: &str = "client";
const OPEN_KEY: &str = "open_gui";

struct State {
    gui: Gui,
    registry: Registry,
    theme: Theme,
    open: bool,
    held: HashSet<usize>,
    mouse_down: HashSet<String>,
    hud: Hud,
    sprint: Sprint,
    respawn: AutoRespawn,
    third: ThirdPerson,
    theme_mod: ThemeModule,
    view_model: ViewModel,
    swing: SwingAnimation,
    hands: HandsShader,
    ambience: Ambience,
    removals: Removals,
    china_hat: ChinaHat,
    shulker: usize,
    auto_tool: AutoTool,
    auto_eat: AutoEat,
    elytra: ElytraHelper,
    configs: Configs,
    inventory: inventory::Inventory,
    zoom: zoom::Zoom,
    last_pos: Option<[f64; 3]>,
    bps: f64,
}

impl State {
    fn sync_theme(&mut self) {
        let State {
            theme_mod,
            registry,
            theme,
            ..
        } = self;
        theme_mod.apply(registry, theme);
    }
}

impl Default for State {
    fn default() -> Self {
        let mut registry = Registry::new();
        let sprint = Sprint::register(&mut registry);
        let hud = Hud::new(HudIds::register(&mut registry));
        let view_model = ViewModel::register(&mut registry);
        let swing = SwingAnimation::register(&mut registry);
        let hands = HandsShader::register(&mut registry);
        let ambience = Ambience::register(&mut registry);
        let removals = Removals::register(&mut registry);
        let china_hat = ChinaHat::register(&mut registry);
        let shulker = shulker::register(&mut registry);
        let respawn = AutoRespawn::register(&mut registry);
        let third = ThirdPerson::register(&mut registry);
        let auto_tool = AutoTool::register(&mut registry);
        let auto_eat = AutoEat::register(&mut registry);
        let elytra = ElytraHelper::register(&mut registry);
        let theme_mod = ThemeModule::register(&mut registry);
        let zoom = zoom::Zoom::register(&mut registry);
        let configs = Configs::register(&mut registry);
        State {
            gui: Gui::default(),
            registry,
            theme: Theme::default(),
            open: false,
            held: HashSet::new(),
            mouse_down: HashSet::new(),
            hud,
            sprint,
            respawn,
            third,
            theme_mod,
            view_model,
            swing,
            hands,
            ambience,
            removals,
            china_hat,
            shulker,
            auto_tool,
            auto_eat,
            elytra,
            configs,
            inventory: inventory::Inventory::default(),
            zoom,
            last_pos: None,
            bps: 0.0,
        }
    }
}

fn hud_data(host: &dyn ClientHost, bps: f64) -> HudData {
    let me = host.local_player().map(|p| p.entity());
    let username = me
        .and_then(|e| e.name(host))
        .unwrap_or_else(|| "Player".to_owned());
    let mut potions = Vec::new();
    if let Some(effects) = me
        .and_then(|e| host.living(e))
        .and_then(|l| l.effects(host))
    {
        for e in effects {
            let id = e.id.id(host);
            let (ns, path) = id.split_once(':').unwrap_or(("minecraft", id.as_str()));
            let key = format!("effect.{ns}.{path}");
            let label = host.translate(&key).unwrap_or_else(|| path.to_owned());
            let tag = (e.amplifier > 0).then(|| (e.amplifier + 1).to_string());
            let value = if e.duration < 0 {
                "**:**".to_owned()
            } else {
                let s = e.duration / 20;
                format!("{}:{:02}", s / 60, s % 60)
            };
            potions.push(Row {
                id: key,
                label,
                tag,
                tag_c: 0xFFFF4A4A,
                value,
                val_c: 0xFFFFFFFF,
            });
        }
    }
    HudData {
        username,
        clock: modules::local_clock(),
        potions,
        fps: host.client_session().client_frame_stats().map_or(0.0, |f| {
            match f.frame_time_ns {
                Some(ns) if ns > 0 => 1.0e9 / ns as f32,
                _ => f.fps,
            }
        }),
        ping: host.client_session().client_latency().unwrap_or(0),
        pos: me.and_then(|e| e.position(host)).unwrap_or([0.0; 3]),
        bps,
    }
}

const HOTBAR_SPRITES: [&str; 3] = ["gui/sprites/hud/hotbar", "minecraft:hud/hotbar", "hud/hotbar"];

fn draw_armor(host: &dyn ClientHost, draw: &mut dyn DrawContext) {
    let Some(me) = host.local_player().map(|p| p.entity()) else {
        return;
    };
    let Some(living) = host.living(me) else {
        return;
    };
    let items: Vec<_> = [
        EquipmentSlot::Feet,
        EquipmentSlot::Legs,
        EquipmentSlot::Chest,
        EquipmentSlot::Head,
    ]
    .into_iter()
    .filter_map(|s| living.equipment(host, s).filter(|i| i.count() > 0))
    .collect();
    if items.is_empty() {
        return;
    }
    let [sw, sh] = draw.size();
    let offhand = living
        .equipment(host, EquipmentSlot::OffHand)
        .is_some_and(|i| i.count() > 0);
    let mut x = (sw / 2.0).floor() + 95.0;
    if offhand && host.gui_options().main_hand_left {
        x += 30.0;
    }
    let y = sh - 22.0;
    let w = items.len() as f32 * 20.0 + 1.0;
    for name in HOTBAR_SPRITES {
        if draw.sprite(name, [0.0, 0.0, w, 22.0], [x, y, w, 22.0], None) {
            draw.sprite(name, [181.0, 0.0, 1.0, 22.0], [x + w, y, 1.0, 22.0], None);
            break;
        }
    }
    for (i, s) in items.iter().enumerate() {
        draw.gui_draw_item_with(s, [x + 3.0 + i as f32 * 20.0, y + 3.0], None);
    }
}

#[derive(Default)]
pub struct ClientMod {
    state: Mutex<State>,
}

impl ClientMod {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn ours(host: &dyn ClientHost) -> bool {
        host.mod_screen() == Some(ID)
    }
}

struct HorizonCanvas<'a> {
    draw: &'a mut dyn DrawContext,
    clips: Vec<bool>,
}

impl Canvas for HorizonCanvas<'_> {
    fn quad(&mut self, points: [[f32; 2]; 4], argb: u32) {
        self.draw.quad(points, argb);
    }

    fn gradient(&mut self, rect: [f32; 4], top: u32, bottom: u32) {
        self.draw.fill_gradient(rect, top, bottom);
    }

    fn sprite(&mut self, name: &str, size: [f32; 2], dst: [f32; 4], tint: u32) {
        self.draw
            .sprite(name, [0.0, 0.0, size[0], size[1]], dst, Some(tint));
    }

    fn text(&mut self, font: &str, text: &str, origin: [f32; 2], scale: f32, argb: u32) {
        let mut style = TextStyle::default();
        style.font = Some(font.to_owned());
        let ops = [
            GuiDraw::Push(vec![
                PoseOp::Translate(origin[0], origin[1]),
                PoseOp::Scale(scale, scale),
            ]),
            GuiDraw::Text {
                text: Text::literal(text).with_style(style),
                x: 0,
                y: 0,
                color: argb,
                shadow: false,
            },
            GuiDraw::Pop,
        ];
        let _ = self.draw.gui_paint(&ops);
    }

    fn sprite_src(&mut self, name: &str, src: [f32; 4], dst: [f32; 4], tint: u32) {
        self.draw.sprite(name, src, dst, Some(tint));
    }

    fn push_clip(&mut self, rect: [f32; 4]) {
        let ok = self.draw.push_clip(rect).is_ok();
        self.clips.push(ok);
    }

    fn pop_clip(&mut self) {
        if self.clips.pop() == Some(true) {
            let _ = self.draw.pop_clip();
        }
    }
}

#[mod_api::horizon_mod]
impl Hooks for ClientMod {
    fn register(&self, r: &mut dyn Registrar, _cx: &RegistrationContext) {
        let mut key = KeyRegistration::new(OPEN_KEY, "key.keyboard.right.shift");
        key.label = "key.client.open_gui".to_owned();
        key.category = "key.categories.client".to_owned();
        r.register_key_mapping(key);
        r.item_part_tag("minecraft:shulker_boxes", shulker::ShulkerPreview);
    }

    fn client_tick_pre(&self, host: &mut dyn ClientHost, _event: &mut ClientTickPreEvent) {
        let mut clicked = false;
        while host.consume_click(ID, OPEN_KEY) {
            clicked = true;
        }
        if clicked && !Self::ours(host) {
            self.lock().gui.reopen();
            let opened = host.mod_screen_open(ID);
            self.lock().open = opened;
        }
        let finished = {
            let st = self.lock();
            st.open && st.gui.finished_closing()
        };
        if finished {
            host.mod_screen_close(ID);
            let mut st = self.lock();
            st.gui.reopen();
            st.open = false;
            let State {
                configs,
                registry,
                hud,
                ..
            } = &mut *st;
            configs.autosave(registry, hud, host, false);
        }
        let ours = Self::ours(host);
        let playing = host.playing();
        let mut st = self.lock();
        if st.open && !ours {
            st.gui.reopen();
            st.open = false;
        }
        let State { registry, held, .. } = &mut *st;
        for (i, m) in registry.modules.iter_mut().enumerate() {
            let Some(key) = m.key.as_deref().filter(|_| m.role == registry::Role::Normal) else {
                held.remove(&i);
                continue;
            };
            let down = playing && !key.starts_with("key.mouse.") && host.is_key_down(key);
            if down && held.insert(i) {
                m.enabled = !m.enabled;
            } else if !down {
                held.remove(&i);
            }
        }
        let State {
            registry,
            respawn,
            third,
            mouse_down,
            ..
        } = &mut *st;
        respawn.tick(registry, host);
        third.tick(registry, host, mouse_down);
        let State {
            registry,
            auto_tool,
            auto_eat,
            elytra,
            configs,
            hud,
            mouse_down,
            inventory,
            zoom,
            ..
        } = &mut *st;
        auto_eat.tick(registry, host);
        if !auto_eat.busy() {
            auto_tool.tick(registry, host);
        }
        elytra.tick(registry, host, mouse_down, inventory);
        inventory.tick(host);
        configs.tick(registry, hud, host);
        zoom.tick(registry, host, mouse_down);
    }

    fn client_tick_post(&self, host: &mut dyn ClientHost, _event: &mut ClientTickPostEvent) {
        let mut st = self.lock();
        st.third.tick_post(host);
        st.china_hat.tick_post(host);
        st.inventory.tick_post();
        let pos = host.local_player().and_then(|p| p.entity().position(host));
        st.bps = match (st.last_pos, pos) {
            (Some(a), Some(b)) => ((b[0] - a[0]).powi(2) + (b[2] - a[2]).powi(2)).sqrt() * 20.0,
            _ => 0.0,
        };
        st.last_pos = pos;
        let shulker_on = st.registry.modules[st.shulker].enabled;
        shulker::ENABLED.store(shulker_on, std::sync::atomic::Ordering::Relaxed);
        st.sync_theme();
    }

    fn register_entity_layers(
        &self,
        _cx: &RegistrationContext,
        event: &mut RegisterEntityLayersEvent<'_>,
    ) {
        for (model, slim) in [(PlayerModel::Wide, false), (PlayerModel::Slim, true)] {
            let _ = event
                .layers
                .add(LayerTarget::Player(model), Arc::new(visuals::SkinProbe { slim }));
        }
    }

    fn standalone_models(&self, _cx: &RegistrationContext, event: &mut StandaloneModelsEvent<'_>) {
        event.models.push("client:hand_overlay".to_owned());
    }

    fn client_setup(&self, host: &mut dyn ClientHost, _event: &mut ClientSetupEvent) {
        self.lock().hands.resolve(host);
    }

    fn model_baking_completed(&self, host: &mut dyn ClientHost, _event: &mut ModelBakingCompletedEvent) {
        self.lock().hands.resolve(host);
    }

    fn client_resources_reloaded(
        &self,
        host: &mut dyn ClientHost,
        _event: &mut ClientResourcesReloadedEvent,
    ) {
        self.lock().hands.resolve(host);
    }

    fn render_hand(&self, host: &dyn ClientHost, event: &mut RenderHandEvent<'_, '_>) -> Flow {
        let st = self.lock();
        let hand = event.render.hand;
        let offset = st.view_model.offset(&st.registry, hand);
        let size = st.view_model.size(&st.registry, hand);
        if st.swing.render_hand(host, &st.registry, offset, size, event) {
            return Flow::Cancel;
        }
        let uniform = size[0] == size[1] && size[1] == size[2];
        if !uniform && st.view_model.redraw_plain(host, offset, size, event) {
            return Flow::Cancel;
        }
        for (t, o) in event.render.translation.iter_mut().zip(offset) {
            *t += o;
        }
        event.render.scale *= if uniform {
            size[0]
        } else {
            (size[0] * size[1] * size[2]).cbrt()
        };
        Flow::Continue
    }

    fn render_arm(&self, host: &dyn ClientHost, event: &mut RenderArmEvent<'_>) -> Flow {
        let mut st = self.lock();
        let State {
            hands,
            registry,
            theme,
            ..
        } = &mut *st;
        hands.render_arm(host, registry, theme, event);
        Flow::Continue
    }

    fn compute_fog_color(&self, _host: &dyn ClientHost, event: &mut ComputeFogColorEvent<'_>) {
        let st = self.lock();
        st.ambience.fog_color(&st.registry, event.rgb);
    }

    fn gui_layer_pre(&self, _host: &dyn ClientHost, event: &mut GuiLayerPreEvent<'_>) -> Flow {
        let st = self.lock();
        st.removals.gui_layer(&st.registry, event.layer)
    }

    fn extract_block_screen_effect(
        &self,
        _host: &dyn ClientHost,
        event: &mut ExtractBlockScreenEffectEvent,
    ) -> Flow {
        let st = self.lock();
        st.removals.screen_effect(&st.registry, event)
    }

    fn player_heart_type(&self, _host: &mut dyn ClientHost, event: &mut PlayerHeartTypeEvent<'_>) {
        let st = self.lock();
        st.removals.heart(&st.registry, event.kind);
    }

    fn entity_outline(&self, _host: &dyn ClientHost, event: &mut EntityOutlineEvent<'_>) {
        let st = self.lock();
        st.removals.outline(&st.registry, event.outline);
    }

    fn movement_input(&self, host: &dyn ClientHost, event: &mut MovementInputEvent) {
        let st = self.lock();
        st.sprint.movement_input(&st.registry, event);
        st.elytra.movement_input(host, event);
        st.inventory.movement_input(event);
    }

    fn zoom(&self, _host: &dyn ClientHost, _event: &mut ZoomEvent) -> Option<mod_api::Zoom> {
        let st = self.lock();
        st.zoom.view(&st.registry)
    }

    fn calculate_player_turn(&self, _host: &dyn ClientHost, event: &mut CalculatePlayerTurnEvent) {
        let st = self.lock();
        st.zoom.player_turn(&st.registry, event);
    }

    fn mouse_scrolling(&self, _host: &mut dyn ClientHost, event: &mut MouseScrollingEvent) -> Flow {
        let mut st = self.lock();
        let State { zoom, registry, .. } = &mut *st;
        if zoom.scroll(registry, event) {
            event.cancelled = true;
            return Flow::Cancel;
        }
        Flow::Continue
    }

    fn submit_custom_geometry(&self, host: &dyn ClientHost, event: &mut SubmitCustomGeometryEvent<'_>) {
        let st = self.lock();
        st.china_hat
            .draw(&st.registry, host, event.view.partial_tick, &mut *event.draw);
    }

    fn camera_pose(&self, host: &dyn ClientHost, event: &mut CameraPoseEvent<'_>) {
        let st = self.lock();
        let (mut yaw, mut pitch) = (event.yaw, event.pitch);
        st.third
            .camera_pose(host, event.partial, event.position, &mut yaw, &mut pitch);
        event.yaw = yaw;
        event.pitch = pitch;
    }

    fn camera_type_cycle(&self, _host: &mut dyn ClientHost, event: &mut CameraTypeCycleEvent<'_>) {
        if self.lock().third.active {
            *event.next = event.previous;
        }
    }

    fn gui_render_post(&self, host: &dyn ClientHost, event: &mut GuiRenderPostEvent<'_>) {
        if event.gui_hidden || host.local_player().is_none() {
            return;
        }
        let bps = self.lock().bps;
        let data = hud_data(host, bps);
        let chat = host
            .observed_screen_id()
            .is_some_and(|s| s.id.starts_with("chat"));
        let frame = HudFrame {
            size: event.batch.size(),
            mouse: host.cursor(),
            gui_scale: host.gui_scale().unwrap_or(2) as f32,
            chat,
        };
        let mut st = self.lock();
        st.sync_theme();
        let mut canvas = HorizonCanvas {
            draw: &mut *event.batch,
            clips: Vec::new(),
        };
        let State {
            hud,
            registry,
            theme,
            ..
        } = &mut *st;
        hud.render(&mut canvas, registry, theme, &data, &frame);
        while !canvas.clips.is_empty() {
            canvas.pop_clip();
        }
        if registry.modules[hud.ids.module].enabled && registry.get_bool(hud.ids.armor_on) {
            draw_armor(host, &mut *event.batch);
        }
    }

    fn mouse_button_pre(&self, host: &mut dyn ClientHost, event: &mut MouseButtonPreEvent) -> Flow {
        let name = gui::mouse_key(event.button);
        let mut st = self.lock();
        if event.pressed {
            st.mouse_down.insert(name.clone());
        } else {
            st.mouse_down.remove(&name);
        }
        if !event.pressed || !host.playing() || host.mod_screen().is_some() {
            return Flow::Continue;
        }
        for m in st.registry.modules.iter_mut() {
            if m.role == registry::Role::Normal && m.key.as_deref() == Some(name.as_str()) {
                m.enabled = !m.enabled;
            }
        }
        Flow::Continue
    }

    fn screen_transparent_background(
        &self,
        host: &dyn ClientHost,
        _event: &mut ScreenTransparentBackgroundEvent<'_>,
    ) -> Flow {
        if !Self::ours(host) {
            return Flow::Continue;
        }
        if self.lock().theme.blur {
            host.apply_screen_blur();
        }
        Flow::Cancel
    }

    fn screen_render_post(&self, host: &dyn ClientHost, event: &mut ScreenRenderPostEvent<'_>) {
        if !Self::ours(host) {
            return;
        }
        let mut st = self.lock();
        st.sync_theme();
        let size = event.draw.size();
        if st.theme.blur {
            host.apply_screen_blur();
        }
        event
            .draw
            .draw_menu_background([0.0, 0.0, size[0], size[1]], true);
        let frame = Frame {
            size,
            mouse: event.screen.mouse,
            gui_scale: host.gui_scale().unwrap_or(2) as f32,
            dt: None,
            millis: gui::now_millis(),
        };
        let mut canvas = HorizonCanvas {
            draw: &mut *event.draw,
            clips: Vec::new(),
        };
        let State {
            gui,
            registry,
            theme,
            ..
        } = &mut *st;
        gui.render(&mut canvas, registry, theme, &frame);
        while !canvas.clips.is_empty() {
            canvas.pop_clip();
        }
    }

    fn screen_mouse_button_pressed_pre(
        &self,
        host: &dyn ClientHost,
        event: &mut ScreenMouseButtonPressedPreEvent<'_>,
    ) -> Flow {
        if !Self::ours(host) {
            if event.button == 0
                && event.screen.id.starts_with("chat")
                && self.lock().hud.mouse_pressed(event.screen.mouse)
            {
                return Flow::Cancel;
            }
            return Flow::Continue;
        }
        let mut st = self.lock();
        let State { gui, registry, .. } = &mut *st;
        gui.mouse_pressed(registry, event.screen.mouse, event.button);
        Flow::Cancel
    }

    fn screen_mouse_button_released_pre(
        &self,
        host: &dyn ClientHost,
        _event: &mut ScreenMouseButtonReleasedPreEvent<'_>,
    ) -> Flow {
        if !Self::ours(host) {
            if self.lock().hud.mouse_released() {
                return Flow::Cancel;
            }
            return Flow::Continue;
        }
        self.lock().gui.mouse_released();
        Flow::Cancel
    }

    fn screen_mouse_dragged_pre(
        &self,
        host: &dyn ClientHost,
        event: &mut ScreenMouseDraggedPreEvent<'_>,
    ) -> Flow {
        if !Self::ours(host) {
            return Flow::Continue;
        }
        let mut st = self.lock();
        let State { gui, registry, .. } = &mut *st;
        gui.mouse_dragged(registry, event.screen.mouse);
        Flow::Cancel
    }

    fn screen_mouse_scrolled_pre(
        &self,
        host: &dyn ClientHost,
        event: &mut ScreenMouseScrolledPreEvent<'_>,
    ) -> Flow {
        if !Self::ours(host) {
            return Flow::Continue;
        }
        let mut st = self.lock();
        let State { gui, registry, .. } = &mut *st;
        gui.mouse_scrolled(registry, event.screen.mouse, event.scroll[1] as f32);
        Flow::Cancel
    }

    fn screen_key_pressed_pre(
        &self,
        host: &dyn ClientHost,
        event: &mut ScreenKeyPressedPreEvent<'_>,
    ) -> Flow {
        if !Self::ours(host) {
            return Flow::Continue;
        }
        let mut st = self.lock();
        let State { gui, registry, .. } = &mut *st;
        match gui.key_pressed(registry, event.key) {
            KeyResult::Consumed | KeyResult::Ignored => Flow::Cancel,
        }
    }

    fn screen_char_typed_pre(
        &self,
        host: &dyn ClientHost,
        event: &mut ScreenCharTypedPreEvent<'_>,
    ) -> Flow {
        if !Self::ours(host) {
            return Flow::Continue;
        }
        let mut st = self.lock();
        let State { gui, registry, .. } = &mut *st;
        if gui.char_typed_reg(registry, event.c) {
            Flow::Cancel
        } else {
            Flow::Continue
        }
    }

    fn screen_closing(&self, host: &dyn ClientHost, _event: &mut ScreenClosingEvent<'_>) {
        if !Self::ours(host) {
            return;
        }
        let mut st = match self.state.try_lock() {
            Ok(st) => st,
            Err(TryLockError::Poisoned(e)) => e.into_inner(),
            Err(TryLockError::WouldBlock) => return,
        };
        if st.open && !st.gui.is_closing() {
            st.gui.reopen();
            st.open = false;
        }
    }
}
