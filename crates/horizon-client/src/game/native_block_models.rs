//! Block models a client mod changes (`native_client_block_model`): what the loaded terrain draws a
//! block state as, for a recoloured ore, a hidden block or a stand-in, changed while the game runs.

use super::Game;
use horizon_world::registry::BlockStateId;
use std::sync::Arc;

// Native mods call it; nothing in Horizon does.
#[allow(dead_code)]
impl Game<'_> {
    /// Asks `native_client_block_model` again for every block state and, when the answers differ
    /// from those in use, rebuilds every loaded section with them (`LevelRenderer.allChanged`, as
    /// changing Biome Blend does). Call it when your answers change (a toggle, an area entered)
    /// and once when the player joins, not every frame: a rebuild takes the workers a few seconds.
    /// The answers survive resource reloads and world changes. API @API:client_block_model@.
    pub(super) fn native_refresh_block_models(&mut self) {
        let count = self.pack.assets.registry.len();
        let mut models: Vec<u32> = (0..count as u32).collect();
        let mut changed = false;
        // Air stays air: the mesher skips it before any lookup.
        for id in 1..count as u32 {
            let mut model = BlockStateId(id);
            self.native_client_block_model(BlockStateId(id), &mut model);
            if (model.0 as usize) < count && model.0 != id {
                models[id as usize] = model.0;
                changed = true;
            }
        }
        // No mod changes anything: the empty table, which the mesher skips whole.
        let models: Arc<[u32]> = if changed {
            models.into()
        } else {
            Arc::default()
        };
        if *models == *self.pack.context.block_models {
            return;
        }
        self.pack.context = self.pack.context_with_block_models(models);
        let context = self.worker_context();
        self.streaming.reload_resources(&mut self.world, context);
    }
}
