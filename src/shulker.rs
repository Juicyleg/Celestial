use crate::player::{get, num};
use mod_api::{
    ClientHost, ItemPart, ItemStack, Tag, TooltipComponentDrawEvent, TooltipComponentEvent,
};
use std::sync::atomic::{AtomicBool, Ordering};

pub static ENABLED: AtomicBool = AtomicBool::new(true);

const W: f32 = 175.0;
const H: f32 = 70.0;
const SPRITE: &str = "client:gui/shulker_3x9";

const DYES: [(&str, u32); 16] = [
    ("white", 0xFFFFFF),
    ("orange", 0xD87F33),
    ("magenta", 0xB24CD8),
    ("light_blue", 0x6699D8),
    ("yellow", 0xE5E533),
    ("lime", 0x7FCC19),
    ("pink", 0xF27FA5),
    ("gray", 0x4C4C4C),
    ("light_gray", 0x999999),
    ("cyan", 0x4C7F99),
    ("purple", 0x7F3FB2),
    ("blue", 0x334CB2),
    ("brown", 0x664C33),
    ("green", 0x667F33),
    ("red", 0x993333),
    ("black", 0x191919),
];

fn tint(host: &dyn ClientHost, stack: &ItemStack) -> u32 {
    let id = stack.item().id(host);
    let path = id.split_once(':').map_or(id.as_str(), |(_, p)| p);
    let dye = path.strip_suffix("_shulker_box").unwrap_or("");
    let rgb = DYES
        .iter()
        .find(|(n, _)| *n == dye)
        .map_or(0x7F3FB2, |(_, c)| *c);
    0xFF00_0000 | rgb
}

fn contents(host: &dyn ClientHost, stack: &ItemStack) -> Vec<(usize, ItemStack)> {
    let Some(c) = stack.components() else {
        return Vec::new();
    };
    let Some(Tag::List(list)) = get(c.as_nbt(), "minecraft:container") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in list {
        let Tag::Compound(entry) = entry else {
            continue;
        };
        let Some(index) = get(entry, "slot").and_then(num) else {
            continue;
        };
        let Some(Tag::Compound(item)) = get(entry, "item") else {
            continue;
        };
        if let Some(s) = ItemStack::from_nbt(host, item)
            && (0.0..27.0).contains(&index)
        {
            out.push((index as usize, s));
        }
    }
    out
}

#[derive(Clone)]
pub struct ShulkerPreview;

#[mod_api::horizon_part]
impl ItemPart for ShulkerPreview {
    fn tooltip_component(
        &self,
        host: &dyn ClientHost,
        event: &mut TooltipComponentEvent<'_>,
    ) -> Option<[f32; 2]> {
        if !ENABLED.load(Ordering::Relaxed) || contents(host, event.stack).is_empty() {
            return None;
        }
        Some([W, H])
    }

    fn tooltip_component_draw(&self, host: &dyn ClientHost, event: &mut TooltipComponentDrawEvent<'_>) {
        let [x, y] = event.origin;
        let draw = &mut *event.draw;
        draw.sprite(SPRITE, [0.0, 0.0, W, H], [x, y, W, H], Some(tint(host, event.stack)));
        for (i, s) in contents(host, event.stack) {
            let (col, row) = ((i % 9) as f32, (i / 9) as f32);
            let at = [x + 8.0 + col * 18.0, y + 8.0 + row * 18.0];
            draw.gui_draw_item_with(&s, at, None);
        }
    }
}

pub fn register(reg: &mut crate::registry::Registry) -> usize {
    reg.add_module(
        2,
        "Shulker Preview",
        "Shows shulker box contents in the item tooltip",
        true,
    )
}
