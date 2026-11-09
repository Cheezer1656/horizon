//! Where blocks' textures are in the block sprite arrays, for a client mod drawing blocks with the player's own
//! textures (resource packs included) in its own pass: with `WorldDrawn::block_sprites`, the arrays themselves.
//! Also every block state's facts and looks, and the resources' files, for a mod working on them off the main thread.
use super::Game;
use horizon_client::block_sprites::{BlockSprites, Lookup, SpriteRef};
use horizon_client::block_states::BlockStates;
use horizon_client::client_resources::ClientResources;
use horizon_client::mod_api;
use horizon_core::geometry::Direction;
use horizon_world::registry::BlockStateId;
use std::sync::Arc;

impl Game<'_> {
    /// For a client mod: what each block (`minecraft:grass_block`, `oak_log[axis=x]`; properties left out take the
    /// block's default) draws on each of its six sides, as the mesher bakes its model: each face's sprite (array and
    /// layer, as `WorldDrawn::block_sprites` cuts them), where it lies on the side, how its texture runs across it,
    /// its tint source and whether it is cut out, translucent or animated; its particle sprite; whether it is a plain
    /// cube. None for a block the game does not know. The model is the state's first random variant. API
    /// 380.
    pub(super) fn native_block_sprites(&self, blocks: &[&str]) -> Vec<Option<BlockSprites>> {
        let lookup = self.native_sprite_lookup();
        blocks.iter().map(|block| lookup.block(block)).collect()
    }
    /// For a client mod: each texture's sprite by name (`minecraft:block/stone`, `block/stone`), none for one no
    /// model uses. API 380.
    pub(super) fn native_texture_sprites(&self, textures: &[&str]) -> Vec<Option<SpriteRef>> {
        let lookup = self.native_sprite_lookup();
        textures
            .iter()
            .map(|texture| lookup.texture(texture))
            .collect()
    }
    /// For a client mod drawing a block entity: the texture block state `state`'s model shows on its `side`
    /// (`block/oak_planks`: vanilla's without their namespace, a mod's with it), by the name `native_draw_cube` and
    /// `native_draw_quad` take, and that face's
    /// tint index (-1 for none): a copycat block drawing a box of its material's textures with `native_draw_cube`,
    /// kept with `BlockEntityDraw::keep`. The face culled against that side comes first, else the first facing it;
    /// None when the model has no face that way or isn't baked. API @API:block_face_sprite@.
    pub(super) fn native_block_face_sprite(
        &self,
        state: BlockStateId,
        side: Direction,
    ) -> Option<(String, i32)> {
        let wanted = horizon_assets::model::Direction::ALL[side as usize];
        let models = self
            .pack
            .render
            .model_catalog
            .collect_seeded(state, 42)
            .ok()?;
        let faces = || models.iter().flat_map(|(_, model)| &model.faces);
        let face = faces()
            .find(|f| f.quad.cull == Some(wanted))
            .or_else(|| faces().find(|f| f.quad.facing == wanted))?;
        let name = self.pack.layer_names.get(&face.layer)?.clone();
        Some((name, face.quad.tint_index))
    }
    /// For a client mod: changes each time the resources load (a resource pack turned on or off can move every
    /// sprite): ask `native_block_sprites` again then. API 380.
    pub(super) fn native_block_sprites_version(&self) -> u64 {
        self.pack.context.render.generation
    }
    /// For a client mod: every block state by its id, with its facts, what its model draws on each side and the
    /// texture behind each sprite, as the resources loaded now; a copy to keep and use on any thread. Ask again when
    /// `native_block_sprites_version` changes. API @API:block_appearance@.
    pub(super) fn native_block_states(&self) -> BlockStates {
        BlockStates::new(
            self.res.facts(),
            self.res.collision(),
            self.pack.context.clone(),
            self.native_sprite_split(),
        )
    }
    /// For a client mod: the files of the resources loaded now, resource packs' over vanilla's, to read textures
    /// and other resources on any thread. Ask again when `native_block_sprites_version` changes. API
    /// @API:block_appearance@.
    pub(super) fn native_client_resources(&self) -> ClientResources {
        ClientResources::new(self.options.resources.clone())
    }
    fn native_sprite_lookup(&self) -> Lookup<'_> {
        Lookup::new(&self.pack.context, self.native_sprite_split())
    }
    fn native_sprite_split(&self) -> u32 {
        // (no renderer, no arrays: every layer counts as array 0's)
        self.renderer
            .as_ref()
            .map_or(u32::MAX, |r| r.sprite_split())
    }
}

// Block models a client mod changes (`native_client_block_model`): what the loaded terrain draws a
// block state as, for a recoloured ore, a hidden block or a stand-in, changed while the game runs.

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
        if self.pack.context.tables.set_block_models(models) {
            let context = self.worker_context();
            self.streaming.reload_resources(&mut self.world, context);
        }
    }
}

/// The client block model hook (API @API:client_block_model@).
impl Game<'_> {
    /// The block state this client draws `state` as in the loaded terrain
    /// (Fabric's and NeoForge's block model overrides; the client's
    /// `BlockStateModelSet.get`). `model` comes in as `state` itself; set it
    /// to another state to draw that state's model, culling and light in its
    /// place: a recoloured ore, a hidden block (air), a plain stand-in. Drawing
    /// only: collision, outline, what the server and other players see and
    /// the world's blocks stay. Asked for every block state (air excepted)
    /// when a mod calls `native_refresh_block_models`, not per block or per
    /// frame, so the answer depends on the state alone and on what the mod
    /// keeps (an area, a toggle); an id past the registry is ignored. API
    /// @API:client_block_model@.
    pub(super) fn native_client_block_model(&self, state: BlockStateId, model: &mut BlockStateId) {
        mod_api::dispatch_client_block_model(self, mod_api::mods(), state, model);
    }
}
