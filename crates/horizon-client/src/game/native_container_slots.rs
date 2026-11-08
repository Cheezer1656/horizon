//! The open container screen's slots for native client mods (docs/native-mods.md,
//! API @API:container_screen_render@): where the screen draws each slot, by the
//! menu's slot number, for the container render hooks and for the hovered slot
//! and slot rectangle helpers. The layout is `Game::slots` and
//! `Game::slot_position`, the one the screen draws and clicks by, so a mod does
//! not derive it again.
use super::{Game, Screen};
use horizon_client::hud::HudBatch;
use horizon_client::items::ItemStack;
use horizon_client::mod_api::{self, NativeSlot};

/// A slot of the open menu for one frame, owned: `NativeSlot` borrows its stack.
struct SlotSnapshot {
    index: usize,
    rect: [f32; 4],
    stack: Option<ItemStack>,
}

impl Game<'_> {
    /// The menu type whose slot numbers the open inventory screen uses: the
    /// one the server opened, `inventory` for the window's own inventory
    /// (and `minecraft:crafting` for a crafting table the client runs
    /// itself). None for a screen no menu numbers: creative, a stock
    /// container the client runs itself, a station or a mount it runs itself.
    fn native_slot_menu(&self) -> Option<&str> {
        if self.screen != Screen::Inventory {
            return None;
        }
        if let Some(menu) = self.remote_menu_type().filter(|m| !m.is_empty()) {
            return Some(menu);
        }
        // The server's own inventory menu, or the client's: no container, no
        // workstation and no mount is up.
        let plain = self.container.is_none()
            && self.station_slot_count().is_none()
            && self.mount_slot_count().is_none();
        match (plain, self.remote.is_some(), self.craft_size) {
            (false, ..) => None,
            (true, false, 3) => Some("minecraft:crafting"),
            (true, ..) => Some("inventory"),
        }
    }
    /// The open screen's slots as `(menu slot number, Game slot id)`, in menu
    /// order: the slots the screen shows, which is `Game::slots` less any slot
    /// the menu has and the screen hides (a mod screen's hidden slots).
    fn native_slot_layout(&self) -> Vec<(usize, usize)> {
        let Some(menu) = self.native_slot_menu() else {
            return Vec::new();
        };
        let shown = self.slots();
        (0..)
            .map_while(|n| Self::menu_slot(menu, n as i32).map(|slot| (n, slot)))
            .filter(|(_, slot)| shown.contains(slot))
            .collect()
    }
    /// The window as `screen_view` and the input hooks size it, for the
    /// helpers that have no `HudBatch` of their own. None without a renderer
    /// (no window) and on any screen but a container's.
    fn native_slot_batch(&self) -> Option<HudBatch> {
        if self.screen != Screen::Inventory {
            return None;
        }
        let renderer = self.renderer.as_ref()?;
        Some(HudBatch::new([
            renderer.config.width,
            renderer.config.height,
        ]))
    }
    /// `native_container_screen_render_background` (`foreground` false) or
    /// `_foreground` for the open container screen, with the screen's origin
    /// `(ox, oy)` as the screen draws it. Called where the screen draws its
    /// slots' highlights: the background before the first, the foreground
    /// after the second. Free with no native mod; with one, the frame's slots
    /// are listed and their stacks cloned, once per call.
    pub(super) fn fire_container_screen_render(
        &mut self,
        batch: &mut HudBatch,
        foreground: bool,
        ox: f32,
        oy: f32,
    ) {
        if mod_api::mods().is_empty() || self.screen != Screen::Inventory {
            return;
        }
        let layout = self.native_slot_layout();
        let snapshot: Vec<SlotSnapshot> = layout
            .into_iter()
            .map(|(index, slot)| {
                let (sx, sy) = self.slot_position(slot);
                SlotSnapshot {
                    index,
                    rect: [ox + sx, oy + sy, 16.0, 16.0],
                    stack: self.slot_view(slot),
                }
            })
            .collect();
        let slots: Vec<NativeSlot<'_>> = snapshot
            .iter()
            .map(|s| NativeSlot {
                index: s.index,
                rect: s.rect,
                stack: s.stack.as_ref(),
            })
            .collect();
        let view = self.screen_view(batch, self.screen);
        if foreground {
            self.native_container_screen_render_foreground(batch, &view, &slots);
        } else {
            self.native_container_screen_render_background(batch, &view, &slots);
        }
    }
    /// The menu slot number of the slot under the pointer on the open
    /// container screen, by the same hit test as its clicks and tooltips
    /// (the slot grows a pixel on every side, so the gaps hover one). None
    /// off every slot, on a screen with no menu numbering, and with no
    /// window. API @API:container_screen_render@.
    pub(super) fn native_hovered_slot(&self) -> Option<usize> {
        let menu = self.native_slot_menu()?;
        let batch = self.native_slot_batch()?;
        let slot = self.slot_at(&batch, self.cursor.0, self.cursor.1)?;
        Self::menu_slot_index(menu, slot)
    }
    /// Where the open container screen draws menu slot `slot`: its 16 x 16
    /// item box `[x, y, width, height]` in GUI pixels. None when the screen
    /// does not show that slot (a mod screen hides some), is not a container
    /// screen, has no menu numbering, or there is no window.
    /// API @API:container_screen_render@.
    pub(super) fn native_slot_rect(&self, slot: usize) -> Option<[f32; 4]> {
        let menu = self.native_slot_menu()?;
        let batch = self.native_slot_batch()?;
        let slot = Self::menu_slot(menu, i32::try_from(slot).ok()?)?;
        if !self.slots().contains(&slot) {
            return None;
        }
        let (ox, oy) = self.inventory_origin(&batch);
        let (sx, sy) = self.slot_position(slot);
        Some([ox + sx, oy + sy, 16.0, 16.0])
    }
}
