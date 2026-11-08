//! The stacks of a server-opened menu for native client mods
//! (docs/native-mods.md, "Entity and level handles", API
//! @API:client_entity_refs@): what the window shows in each slot, as the
//! server last synced it, where `NativeScreenView` only counts the slots.
use super::Game;
use horizon_client::items::ItemStack;
use std::borrow::Cow;

impl Game<'_> {
    /// The stacks of the menu the server opened for this window, with their
    /// components, by the server's slot number (`AbstractContainerMenu.slots`,
    /// the numbers `ContainerSetSlot` and `ContainerClick` carry): the menu's
    /// own slots, then the player's inventory and hotbar below it. An empty
    /// slot is None. None on a server, while no server-opened menu is up (the
    /// window's own inventory, creative), and for a menu whose slots the
    /// client doesn't lay out (a lectern). API @API:client_entity_refs@.
    pub(super) fn native_menu_stacks(&mut self) -> Option<Vec<Option<ItemStack>>> {
        let (_, menu) = self.remote.as_ref()?.menu.clone()?;
        let slots: Vec<usize> = (0..).map_while(|s| Self::menu_slot(&menu, s)).collect();
        if slots.is_empty() {
            return None;
        }
        Some(slots.into_iter().map(|i| self.slot_view(i)).collect())
    }
    /// What the open server menu draws in game slot `internal` holding
    /// `stack`: the stack after native mods' `native_menu_slot_display`
    /// (API @API:menu_slot_display@), asked with the server's slot number.
    /// Only the picture and tooltip use this. Without mods, or while no
    /// server-opened menu is up, `stack` comes back as it is, with no
    /// clone or slot lookup.
    pub(super) fn menu_slot_shown<'a>(
        &self,
        internal: usize,
        stack: &'a Option<ItemStack>,
    ) -> Cow<'a, Option<ItemStack>> {
        if horizon_client::mod_api::mods().is_empty() {
            return Cow::Borrowed(stack);
        }
        let Some((_, menu)) = self.remote.as_ref().and_then(|r| r.menu.as_ref()) else {
            return Cow::Borrowed(stack);
        };
        let Some(slot) = Self::menu_slot_index(menu, internal) else {
            return Cow::Borrowed(stack);
        };
        let mut shown = stack.clone();
        self.native_menu_slot_display(slot, &mut shown);
        Cow::Owned(shown)
    }
}
