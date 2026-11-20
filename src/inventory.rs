use crate::player::component;
use mod_api::{
    ClientHost, Identifier, MenuClick, MouseButton, MovementInputEvent, ServerboundPlayPacket,
    SwapWith,
};
use std::collections::VecDeque;

pub const OFFHAND: usize = 45;

#[derive(Clone, Copy)]
struct Task {
    from: usize,
    to: usize,
    stop: i32,
}

#[derive(Default)]
pub struct Inventory {
    queue: VecDeque<Task>,
    stop: i32,
    started: bool,
}

fn swap_with(slot: usize) -> Option<SwapWith> {
    match slot {
        OFFHAND => Some(SwapWith::Offhand),
        36..=44 => Some(SwapWith::Hotbar((slot - 36) as u8)),
        _ => None,
    }
}

fn pickup(slot: usize, button: MouseButton) -> MenuClick {
    MenuClick::Pickup {
        slot: Some(slot),
        button,
    }
}

fn swap(slot: usize, with: SwapWith) -> MenuClick {
    MenuClick::Swap { slot, with }
}

fn close_inventory(host: &mut dyn ClientHost) {
    if let Ok(id) = Identifier::parse("minecraft:container_close") {
        let _ = host.send_packet(ServerboundPlayPacket::new(id, vec![0]));
    }
}

pub fn move_now(host: &mut dyn ClientHost, from: usize, to: usize) -> bool {
    let Some(player) = host.local_player() else {
        return false;
    };
    let Some(menu) = player.container_menu(host) else {
        return false;
    };
    let from_stack = menu.slot(host, from).flatten().filter(|s| s.count() > 0);
    let bundle = from_stack
        .as_ref()
        .is_some_and(|s| component(host, s, "minecraft:bundle_contents").is_some());
    let ok = if bundle {
        let first = menu.click(
            host,
            &[pickup(from, MouseButton::Right), pickup(to, MouseButton::Left)],
        );
        let carried = menu
            .carried(host)
            .flatten()
            .is_some_and(|s| s.count() > 0);
        if first && carried {
            menu.click(host, &[pickup(from, MouseButton::Left)]);
        }
        first
    } else if let Some(w) = swap_with(to) {
        menu.click(host, &[swap(from, w)])
    } else if let Some(w) = swap_with(from) {
        menu.click(host, &[swap(to, w)])
    } else if from != to {
        let h = SwapWith::Hotbar(0);
        menu.click(host, &[swap(from, h), swap(to, h), swap(from, h)])
    } else {
        false
    };
    close_inventory(host);
    ok
}

impl Inventory {
    pub fn busy(&self) -> bool {
        !self.queue.is_empty()
    }

    pub fn queue_move(&mut self, from: usize, to: usize, stop_ticks: i32) {
        let task = Task {
            from,
            to,
            stop: stop_ticks,
        };
        if self.queue.is_empty() && task.stop > 0 {
            self.stop = task.stop;
            self.started = true;
        }
        self.queue.push_back(task);
    }

    pub fn tick(&mut self, host: &mut dyn ClientHost) {
        if host.local_player().is_none() {
            self.queue.clear();
            self.started = false;
            return;
        }
        let Some(&task) = self.queue.front() else {
            return;
        };
        let current = if self.started { self.stop } else { -1 };
        if current < task.stop {
            move_now(host, task.from, task.to);
            self.queue.pop_front();
            if let Some(next) = self.queue.front() {
                self.stop = next.stop;
                self.started = true;
            }
        }
    }

    pub fn tick_post(&mut self) {
        if !self.started {
            return;
        }
        if self.stop > 0 {
            self.stop -= 1;
        } else {
            self.started = false;
            self.stop = 0;
        }
    }

    pub fn movement_input(&self, event: &mut MovementInputEvent) {
        if !self.started {
            return;
        }
        event.forward = 0.0;
        event.strafe = 0.0;
        event.jump = false;
        event.forward_key = false;
        event.backward_key = false;
        event.left_key = false;
        event.right_key = false;
    }
}
