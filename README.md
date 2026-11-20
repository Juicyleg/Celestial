# Celestial Client

A native Rust client mod for the [Horizon](https://github.com/horizon-rust/horizon) Minecraft engine. It brings the Celestial ClickGUI and HUD to Horizon, together with a set of visual, movement and quality of life modules, packaged as a single `client.hmod`.

The whole menu and HUD are drawn by the mod itself through Horizon's GUI drawing API, with animated theme gradients, soft glows and smooth open and close transitions.

## Compatibility

| | |
|---|---|
| Game | Horizon 26.3 |
| Horizon commit | `0b14f8acfd97fa2c750e42795249dcc981178ca4` |
| Mixins commit | `e59da9094f0c2fdda6c627fe9eff22dec6a906f0` |
| Mod type | Native, client side |

Native mods are compiled by the Horizon launcher against the exact game build, so the commit pins above must match your installed Horizon Nightly.

## Features

### ClickGUI

- Opens with **Right Shift**.
- Categories: Combat, Movement, Render, Player, Util, Configs and Themes.
- Left click a module card to toggle it, right click (or the gear icon) to open its settings panel, middle click to bind it to a key or mouse button.
- Settings panels can be dragged, stacked and closed. Setting types include toggles, sliders, dropdowns, groups, buttons, key binds and text fields.
- Search bar that filters modules across every category.
- Reopens on the tab you last used.

### HUD

Every element can be dragged while the chat is open, and the whole HUD has a size slider.

- **Watermark** with player name and a clock with seconds.
- **Info panel** with FPS (real time), ping, coordinates and blocks per second. Shown as one bar or as separate boxes, at the top centre or on the side. Digits use a fixed width so the boxes do not jitter.
- **Module list** of enabled modules.
- **Potions** with level and remaining time.
- **Keybinds** of bound, enabled modules.
- **Armor** strip next to the hotbar.

### Modules

| Category | Module | What it does |
|---|---|---|
| Movement | Sprint | Sprints automatically while moving forward |
| Render | HUD | Watermark, info panel, module list, potions, keybinds and armor |
| Render | View Model | Moves and resizes the items in your hands, per hand and per axis |
| Render | Swing Animation | Five custom swing animations with angle, tilt and intensity |
| Render | Hands Shader | Animated, theme coloured overlay on the first person arm (wide and slim models) |
| Render | Ambience | Custom fog colour |
| Render | Removals | Hides the scoreboard, boss bar, fire and underwater overlays, black hearts and glowing outlines |
| Render | Shulker Preview | Shows shulker box contents in the tooltip, tinted to the box colour |
| Render | China Hat | Theme coloured cone hat above your own head in third person |
| Player | Auto Respawn | Respawns automatically after death |
| Player | Third Person | Free look in third person without changing movement direction |
| Player | Auto Tool | Switches to the fastest tool while you break a block, then switches back |
| Player | Auto Eat | Eats the best food when hunger drops below a set level |
| Player | Elytra Helper | Swaps between your elytra and best chestplate on its own key, with optional auto start |
| Util | Zoom | Hold or toggle zoom with adjustable amount, smooth transition, reduced or cinematic mouse and scroll to adjust |
| Themes | ClickGUI | Theme colour, gradient speed, blur and glow for the menu and HUD |

### Configs

- **New Config** saves the current setup as a new config card.
- Click a config card to load it and make it active.
- Each config has a Name field (renames the file), Load, Save and Remove.
- Configs are plain JSON files in `config/client/configs/`.
- The current state is also saved automatically to `config/client/auto.json` and restored when you join a world.

### Shared inventory handling

Item moves (Auto Tool, Auto Eat, Elytra Helper) go through one queue that uses hotbar swaps where possible, handles bundles, closes the inventory afterwards and can briefly stop movement during a swap to avoid desync.

## Performance

- Glows are drawn from small pre-made textures instead of thousands of computed shapes.
- Rounded boxes, borders and gradient fill geometry are built once and reused while nothing changes. Only colours are recalculated each frame, so the theme gradient stays smooth.
- HUD text is drawn on its own, so changing values like FPS and BPS do not rebuild their backgrounds.

## Installing

1. Download `client.hmod` from the releases page.
2. Put it in your Horizon instance's mods folder (`horizon-mods/` or `horizon-native-mods/`, depending on your launcher version).
3. Enable it in the launcher. The launcher compiles native mods for your game build on first launch.
4. In game, press **Right Shift** to open the menu.

## Building from source

Requirements: Rust (edition 2024) and the official Horizon mod tool.

```sh
cargo binstall --git https://github.com/horizon-rust/horizon cargo-horizon-mod
```

Fetch the SDK for the pinned Horizon build into `.horizon/sdk` (the tool does this when you build), then:

```sh
cargo horizon-mod build
cargo horizon-mod check target/horizon-mod/client.hmod
```

The generated package is written to `target/horizon-mod/client.hmod`.

### Regenerating textures

The category icons, exit icon and glow textures are generated by scripts in `tools/`:

```sh
python3 tools/modern_icons.py
python3 tools/glow_tex.py
```

`modern_icons.py` needs `rsvg-convert`; `glow_tex.py` needs `numpy` and `Pillow`.

## Project layout

```
src/
  lib.rs         hook wiring and shared state
  gui.rs         ClickGUI
  hud.rs         HUD elements
  paint.rs       vector painter, glow sprites and geometry cache
  registry.rs    modules, settings and events
  config.rs      config files and autosave
  inventory.rs   shared inventory swap queue
  modules.rs     Sprint, Auto Respawn, Third Person, clock
  player.rs      Auto Tool, Auto Eat, Elytra Helper
  visuals.rs     View Model, Swing Animation, Hands Shader, Ambience, Removals, theme
  cosmetics.rs   China Hat
  shulker.rs     Shulker Preview
  zoom.rs        Zoom
  metrics.rs     font metrics
assets/client/   textures, fonts, models and language files
tools/           texture generators
```

## Credits

The menu and HUD design follow the Celestial client. Several module behaviours follow the Delta client. This project is a reimplementation for Horizon in Rust.
