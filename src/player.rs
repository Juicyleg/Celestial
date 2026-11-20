use crate::registry::Registry;
use mod_api::{
    BlockState, ClientHost, Compound, CrosshairTarget, ItemStack, MenuRef,
    MovementInputEvent, PlayerRef, Tag,
};
use crate::inventory::Inventory;
use std::collections::HashSet;

pub const HOTBAR: std::ops::RangeInclusive<usize> = 36..=44;
pub const MAIN: std::ops::RangeInclusive<usize> = 9..=35;
const CHEST: usize = 6;

pub fn bind_down(reg: &Registry, s: usize, host: &dyn ClientHost, mouse: &HashSet<String>) -> bool {
    match reg.get_bind(s) {
        Some(k) if k.starts_with("key.mouse.") => mouse.contains(k),
        Some(k) => host.is_key_down(k),
        None => false,
    }
}

pub fn get<'a>(c: &'a Compound, key: &str) -> Option<&'a Tag> {
    c.iter()
        .find(|(k, _)| k.to_string_lossy() == key)
        .map(|(_, v)| v)
}

pub fn num(t: &Tag) -> Option<f64> {
    Some(match t {
        Tag::Byte(v) => *v as f64,
        Tag::Short(v) => *v as f64,
        Tag::Int(v) => *v as f64,
        Tag::Long(v) => *v as f64,
        Tag::Float(v) => f32::from_bits(*v) as f64,
        Tag::Double(v) => f64::from_bits(*v),
        _ => return None,
    })
}

pub fn text(t: &Tag) -> Option<String> {
    match t {
        Tag::String(s) => Some(s.to_string_lossy()),
        _ => None,
    }
}

pub fn component(host: &dyn ClientHost, stack: &ItemStack, key: &str) -> Option<Tag> {
    if let Some(c) = stack.components() {
        let nbt = c.as_nbt();
        if get(nbt, &format!("!{key}")).is_some() {
            return None;
        }
        if let Some(t) = get(nbt, key) {
            return Some(t.clone());
        }
    }
    host.item_default_component(stack.item(), key)
}

pub fn has_patch(stack: &ItemStack, key: &str) -> bool {
    stack
        .components()
        .is_some_and(|c| get(c.as_nbt(), key).is_some())
}

pub fn item_id(host: &dyn ClientHost, stack: &ItemStack) -> String {
    stack.item().id(host)
}

fn enchant_level(host: &dyn ClientHost, stack: &ItemStack, ench: &str) -> i32 {
    let Some(Tag::Compound(c)) = component(host, stack, "minecraft:enchantments") else {
        return 0;
    };
    let levels = match get(&c, "levels") {
        Some(Tag::Compound(l)) => l,
        _ => &c,
    };
    get(levels, ench).and_then(num).unwrap_or(0.0) as i32
}

fn menu(host: &dyn ClientHost) -> Option<(PlayerRef, MenuRef)> {
    let player = host.local_player()?;
    let menu = player.container_menu(host)?;
    Some((player, menu))
}

fn slot(host: &dyn ClientHost, menu: MenuRef, i: usize) -> Option<ItemStack> {
    menu.slot(host, i).flatten().filter(|s| s.count() > 0)
}

fn free_hands(host: &dyn ClientHost) -> bool {
    host.playing() && host.mod_screen().is_none() && host.observed_screen_id().is_none()
}

#[derive(Clone, Copy)]
enum Restore {
    Select(u8),
    Swap { slot: usize, hotbar: u8 },
}

fn take_slot(host: &mut dyn ClientHost, from: usize) -> Option<Restore> {
    let (player, _) = menu(host)?;
    let cur = player.selected_slot(host)? as u8;
    if HOTBAR.contains(&from) {
        let target = (from - 36) as u8;
        if target == cur {
            return None;
        }
        host.select_hotbar_slot(target).ok()?;
        return Some(Restore::Select(cur));
    }
    crate::inventory::move_now(host, from, 36 + cur as usize).then_some(Restore::Swap {
        slot: from,
        hotbar: cur,
    })
}

fn restore(host: &mut dyn ClientHost, r: Restore) {
    match r {
        Restore::Select(i) => {
            let _ = host.select_hotbar_slot(i);
        }
        Restore::Swap { slot, hotbar } => {
            crate::inventory::move_now(host, slot, 36 + hotbar as usize);
        }
    }
}

fn blocks_match(host: &dyn ClientHost, t: &Tag, state: BlockState, id: &str) -> bool {
    match t {
        Tag::String(s) => {
            let s = s.to_string_lossy();
            match s.strip_prefix('#') {
                Some(tag) => host.block_in_tag(state.block(), tag) == Some(true),
                None => s == id,
            }
        }
        Tag::List(l) => l.iter().any(|t| blocks_match(host, t, state, id)),
        _ => false,
    }
}

fn mining_speed(host: &dyn ClientHost, stack: Option<&ItemStack>, state: BlockState, id: &str) -> f64 {
    let Some(stack) = stack else {
        return 1.0;
    };
    let Some(Tag::Compound(tool)) = component(host, stack, "minecraft:tool") else {
        return 1.0;
    };
    let mut speed = get(&tool, "default_mining_speed").and_then(num).unwrap_or(1.0);
    if let Some(Tag::List(rules)) = get(&tool, "rules") {
        for rule in rules {
            let Tag::Compound(rule) = rule else {
                continue;
            };
            let Some(blocks) = get(rule, "blocks") else {
                continue;
            };
            if let Some(s) = get(rule, "speed").and_then(num)
                && blocks_match(host, blocks, state, id)
            {
                speed = s;
                break;
            }
        }
    }
    if speed > 1.0 {
        let lvl = enchant_level(host, stack, "minecraft:efficiency");
        if lvl > 0 {
            speed += (lvl * lvl + 1) as f64;
        }
    }
    speed
}

pub struct AutoTool {
    pub module: usize,
    restore: Option<Restore>,
    last_block: Option<[i32; 3]>,
}

impl AutoTool {
    pub fn register(reg: &mut Registry) -> AutoTool {
        let module = reg.add_module(
            3,
            "Auto Tool",
            "Switches to the fastest tool for the block you are breaking",
            false,
        );
        AutoTool {
            module,
            restore: None,
            last_block: None,
        }
    }

    pub fn tick(&mut self, reg: &Registry, host: &mut dyn ClientHost) {
        let attack = host
            .mapping("key.attack")
            .ok()
            .flatten()
            .is_some_and(|m| m.down);
        let target = match host.crosshair_target() {
            Some(CrosshairTarget::Block { pos, state, .. }) => Some((pos, state)),
            _ => None,
        };
        let active = reg.modules[self.module].enabled && attack && free_hands(host);
        let Some((pos, state)) = target.filter(|_| active) else {
            self.last_block = None;
            if let Some(r) = self.restore.take() {
                restore(host, r);
            }
            return;
        };
        let p = [pos.x, pos.y, pos.z];
        if self.last_block == Some(p) {
            return;
        }
        self.last_block = Some(p);
        let Some((player, menu)) = menu(host) else {
            return;
        };
        let Some(cur) = player.selected_slot(host) else {
            return;
        };
        let id = state.block().id(host);
        let held = slot(host, menu, 36 + cur);
        let mut best = (mining_speed(host, held.as_ref(), state, &id), None);
        let swapped = matches!(self.restore, Some(Restore::Swap { .. }));
        let range: Vec<usize> = if swapped {
            HOTBAR.collect()
        } else {
            HOTBAR.chain(MAIN).collect()
        };
        for i in range {
            if i == 36 + cur {
                continue;
            }
            let Some(s) = slot(host, menu, i) else {
                continue;
            };
            let v = mining_speed(host, Some(&s), state, &id);
            if v > best.0 {
                best = (v, Some(i));
            }
        }
        let Some(i) = best.1 else {
            return;
        };
        if let Some(Restore::Select(orig)) = self.restore
            && HOTBAR.contains(&i)
        {
            let _ = host.select_hotbar_slot((i - 36) as u8);
            self.restore = Some(Restore::Select(orig));
            return;
        }
        if self.restore.is_some() {
            return;
        }
        self.restore = take_slot(host, i);
    }
}

const EAT_BLACKLIST: [&str; 3] = [
    "minecraft:chorus_fruit",
    "minecraft:pufferfish",
    "minecraft:dried_kelp",
];
const RAW: [&str; 4] = [
    "minecraft:rotten_flesh",
    "minecraft:spider_eye",
    "minecraft:poisonous_potato",
    "minecraft:suspicious_stew",
];

fn food_rank(host: &dyn ClientHost, stack: &ItemStack) -> Option<(u8, f64)> {
    let id = item_id(host, stack);
    if EAT_BLACKLIST.contains(&id.as_str()) || has_patch(stack, "minecraft:custom_name") {
        return None;
    }
    let Some(Tag::Compound(food)) = component(host, stack, "minecraft:food") else {
        return None;
    };
    let nutrition = get(&food, "nutrition").and_then(num).unwrap_or(0.0);
    let rank = match id.as_str() {
        "minecraft:enchanted_golden_apple" => 3,
        "minecraft:golden_apple" => 2,
        s if s.contains(":raw_") || RAW.contains(&s) => 1,
        _ => 0,
    };
    Some((rank, nutrition))
}

struct EatTask {
    restore: Option<Restore>,
    ticks: u32,
    started: bool,
}

pub struct AutoEat {
    pub module: usize,
    pub hunger: usize,
    task: Option<EatTask>,
}

impl AutoEat {
    pub fn register(reg: &mut Registry) -> AutoEat {
        let module = reg.add_module(
            3,
            "Auto Eat",
            "Eats the best food in your inventory when hunger drops",
            false,
        );
        let hunger = reg.slider("Eat when hunger below", 16.0, 1.0, 20.0, 1.0, "");
        reg.attach(module, &[hunger]);
        AutoEat {
            module,
            hunger,
            task: None,
        }
    }

    pub fn busy(&self) -> bool {
        self.task.is_some()
    }

    fn finish(&mut self, host: &mut dyn ClientHost) {
        if let Some(t) = self.task.take() {
            let _ = host.set_mapping_down("key.use", false);
            if let Some(r) = t.restore {
                restore(host, r);
            }
        }
    }

    pub fn tick(&mut self, reg: &Registry, host: &mut dyn ClientHost) {
        let Some(player) = host.local_player() else {
            self.task = None;
            return;
        };
        let level = player.food(host).map(|f| f.level).unwrap_or(20);
        if let Some(t) = self.task.as_mut() {
            t.ticks += 1;
            let using = host.local_item_use().is_some();
            t.started |= using;
            let done = (t.started && !using) || t.ticks > 60 || !free_hands(host);
            if done || !reg.modules[self.module].enabled {
                self.finish(host);
            } else {
                let _ = host.set_mapping_down("key.use", true);
            }
            return;
        }
        if !reg.modules[self.module].enabled || !free_hands(host) {
            return;
        }
        if player.entity().tick_count(host).unwrap_or(0) <= 40 {
            return;
        }
        if level as f32 >= reg.get_slider(self.hunger) || host.local_item_use().is_some() {
            return;
        }
        if player.is_creative(host) == Some(true) || player.is_spectator(host) == Some(true) {
            return;
        }
        let Some(menu) = player.container_menu(host) else {
            return;
        };
        let mut best: Option<(u8, f64, usize)> = None;
        for i in HOTBAR.chain(MAIN) {
            let Some(s) = slot(host, menu, i) else {
                continue;
            };
            let Some((rank, n)) = food_rank(host, &s) else {
                continue;
            };
            let better = match best {
                None => true,
                Some((br, bn, _)) => rank < br || (rank == br && n > bn),
            };
            if better {
                best = Some((rank, n, i));
            }
        }
        let Some((_, _, i)) = best else {
            return;
        };
        let cur = player.selected_slot(host).unwrap_or(0);
        let restore = if i == 36 + cur {
            None
        } else {
            match take_slot(host, i) {
                Some(r) => Some(r),
                None => return,
            }
        };
        let _ = host.set_mapping_down("key.use", true);
        self.task = Some(EatTask {
            restore,
            ticks: 0,
            started: false,
        });
    }
}

fn is_elytra(host: &dyn ClientHost, s: &ItemStack) -> bool {
    item_id(host, s) == "minecraft:elytra" || component(host, s, "minecraft:glider").is_some()
}

fn chest_score(host: &dyn ClientHost, s: &ItemStack) -> Option<f64> {
    if is_elytra(host, s) {
        return None;
    }
    let Some(Tag::Compound(eq)) = component(host, s, "minecraft:equippable") else {
        return None;
    };
    if get(&eq, "slot").and_then(text).as_deref() != Some("chest") {
        return None;
    }
    let mut score = 0.0;
    let mods = match component(host, s, "minecraft:attribute_modifiers") {
        Some(Tag::List(l)) => l,
        Some(Tag::Compound(c)) => match get(&c, "modifiers") {
            Some(Tag::List(l)) => l.clone(),
            _ => Vec::new(),
        },
        _ => Vec::new(),
    };
    for m in mods {
        let Tag::Compound(m) = m else {
            continue;
        };
        let kind = get(&m, "type").and_then(text).unwrap_or_default();
        let amount = get(&m, "amount").and_then(num).unwrap_or(0.0);
        if kind.ends_with("armor") || kind.ends_with("armor_toughness") {
            score += amount;
        }
    }
    score += enchant_level(host, s, "minecraft:protection") as f64;
    Some(score)
}

pub struct ElytraHelper {
    pub module: usize,
    pub key: usize,
    pub auto_start: usize,
    was_down: bool,
    start_ticks: u32,
}

impl ElytraHelper {
    pub fn register(reg: &mut Registry) -> ElytraHelper {
        let module = reg.add_module(
            3,
            "Elytra Helper",
            "Swaps between your elytra and best chestplate on a key",
            false,
        );
        let key = reg.bind("Swap key", None);
        let auto_start = reg.boolean("Auto start after swap", false);
        reg.attach(module, &[key, auto_start]);
        ElytraHelper {
            module,
            key,
            auto_start,
            was_down: false,
            start_ticks: 0,
        }
    }

    fn plan(&self, host: &dyn ClientHost) -> Option<(usize, bool)> {
        let (_, menu) = menu(host)?;
        let chest = slot(host, menu, CHEST);
        let wearing = chest.as_ref().is_some_and(|s| is_elytra(host, s));
        let mut pick: Option<(f64, usize)> = None;
        for i in HOTBAR.chain(MAIN) {
            let Some(s) = slot(host, menu, i) else {
                continue;
            };
            let score = if wearing {
                chest_score(host, &s)
            } else if is_elytra(host, &s) {
                Some(0.0)
            } else {
                None
            };
            if let Some(v) = score
                && pick.is_none_or(|(b, _)| v > b)
            {
                pick = Some((v, i));
            }
        }
        pick.map(|(_, from)| (from, !wearing))
    }

    pub fn tick(
        &mut self,
        reg: &Registry,
        host: &mut dyn ClientHost,
        mouse: &HashSet<String>,
        inv: &mut Inventory,
    ) {
        let enabled = reg.modules[self.module].enabled;
        let down = enabled && free_hands(host) && bind_down(reg, self.key, host, mouse);
        if down && !self.was_down && !inv.busy()
            && let Some((from, to_elytra)) = self.plan(host)
        {
            inv.queue_move(from, CHEST, 1);
            if to_elytra && reg.get_bool(self.auto_start) {
                self.start_ticks = 12;
            }
        }
        self.was_down = down;
        if self.start_ticks > 0 {
            self.start_ticks -= 1;
            let gliding = host
                .local_player()
                .and_then(|p| host.living(p.entity()))
                .and_then(|l| l.is_gliding(host))
                .unwrap_or(false);
            if gliding || !enabled {
                self.start_ticks = 0;
            }
        }
    }

    pub fn movement_input(&self, host: &dyn ClientHost, event: &mut MovementInputEvent) {
        if self.start_ticks == 0 {
            return;
        }
        let wearing = host
            .local_player()
            .and_then(|p| host.living(p.entity()))
            .and_then(|l| l.equipment(host, mod_api::EquipmentSlot::Chest))
            .is_some_and(|s| is_elytra(host, &s));
        if !wearing {
            return;
        }
        let age = host
            .local_player()
            .and_then(|p| p.entity().tick_count(host))
            .unwrap_or(0);
        event.jump = age % 2 == 0;
    }
}
