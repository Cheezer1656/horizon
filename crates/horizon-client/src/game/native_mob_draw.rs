//! Mobs drawn by native mods on the client: `native_mob_submit` (vanilla's
//! `EntityRenderer.submit` for a mob, NeoForge's `RenderLivingEvent.Pre` when
//! it cancels) gets the mob's render state and a [`BlockEntityDraw`] posed at
//! its feet and turned with its body, so a mod draws a model of its own with
//! the helpers block entities draw with (`native_draw_cube`,
//! `native_draw_quad`, `native_draw_block`, `native_draw_item`). A player
//! has `native_player_submit` (`RenderPlayerEvent.Pre`), and every mob and
//! player in view gets `native_mob_layer` after its vanilla model
//! (`EntityRenderersEvent.AddLayers`), in the same frame.

use super::{Dim, Game, native_block_entity_draw::BlockEntityDraw};
use glam::{DVec3, Mat4};
use horizon_client::{
    block_entity_render::diffuse_lights,
    mesher::Vertex,
    mobs::{Mob, anim},
    mod_api,
};

/// One mob's frame, as `EntityRenderer.extractRenderState` fills a
/// `LivingEntityRenderState`: what a model reads to pose and animate itself.
/// Angles are in degrees, interpolated to the frame's partial tick.
// Native mods read it; nothing in Horizon does.
#[derive(Clone, Debug)]
pub struct NativeMobRender<'a> {
    /// The mob's network id, as the mob helpers and mod messages name it.
    pub id: u32,
    /// Its entity type (`minecraft:villager`).
    pub kind: &'a str,
    /// Its `CustomName` as plain text, when it has one.
    pub custom_name: Option<&'a str>,
    /// `isBaby`.
    pub baby: bool,
    /// The frame's partial tick, 0 to 1.
    pub partial: f32,
    /// `ageInTicks`: ticks lived plus the partial tick, for idle motion.
    pub age: f32,
    /// `bodyRot`: the way the body faces (0 is south, 90 west).
    pub body_yaw: f32,
    /// `yRot`: the head's turn from the body, -180 to 180.
    pub head_yaw: f32,
    /// `xRot`: the head's pitch, positive looking down.
    pub pitch: f32,
    /// `walkAnimationPos`: how far the legs have swung, for a walk cycle
    /// (a vanilla quadruped's legs swing `cos(pos * 0.6662) * 1.4 * speed` radians).
    pub walk_position: f32,
    /// `walkAnimationSpeed`: how fast it walks, 0 standing to about 1.
    pub walk_speed: f32,
    /// Ticks left of the red hurt flash (10 when hit).
    pub hurt_time: i32,
    /// Ticks since it died, 0 while alive (vanilla tips the body over 20).
    pub death_time: i32,
    pub on_ground: bool,
}

impl<'a> NativeMobRender<'a> {
    /// The render state of `m` at `partial` (`LivingEntityRenderer.extractRenderState`).
    pub(super) fn of(m: &'a Mob, partial: f32) -> Self {
        let body = anim::body_rot(m, partial);
        // The walk cycle, head turn and age as the vanilla models read them:
        // no walking while riding or dead, a baby's legs three times as far
        // (`WalkAnimationState.position(partial)`, `positionScale`).
        let pose = anim::state(m, partial);
        Self {
            id: m.id,
            kind: &m.kind.id,
            custom_name: m.custom_name.as_deref(),
            baby: m.is_baby(),
            partial,
            age: pose.age,
            body_yaw: body,
            head_yaw: pose.y_rot,
            pitch: pose.x_rot,
            walk_position: pose.pos,
            walk_speed: pose.speed,
            hurt_time: m.hurt_time,
            death_time: m.death_time,
            on_ground: m.on_ground,
        }
    }
}

/// Which player a [`NativeMobRender`] is, for `native_player_submit` and
/// `native_mob_layer`.
// Native mods read it; nothing in Horizon does.
#[derive(Clone, Copy, Debug)]
pub struct NativePlayerRender<'a> {
    /// The player's profile name.
    pub name: &'a str,
    /// Whether it is the player of this window, drawn in third person.
    pub local: bool,
}

/// The pose a mod's mob model starts from: at the feet, `at` (relative to the
/// camera), turned with the body so +z is the way it faces, +x its left and +y
/// up, in blocks.
pub(super) fn mob_pose(at: DVec3, body_yaw: f32) -> Mat4 {
    Mat4::from_translation(at.as_vec3()) * Mat4::from_rotation_y((-body_yaw).to_radians())
}

impl Game<'_> {
    /// `native_mob_submit` for `mob`, lit by `light` (packed as the entity
    /// pass packs it): true when a mod drew it, and its vanilla model, armour
    /// and held items are left out. The name tag, shadow, fire and a glowing
    /// outline still draw as for any mob.
    pub(super) fn mod_mob_geometry(
        &self,
        mob: &Mob,
        partial: f32,
        camera: DVec3,
        light: u32,
        out: &mut Vec<Vertex>,
    ) -> bool {
        self.mod_entity_draw(mob, partial, camera, light, out, |render, draw| {
            let mut drawn = false;
            self.native_mob_submit(render, draw, &mut drawn);
            drawn
        })
    }
    /// `native_player_submit` for the player `mob` (`who` says which): true
    /// when a mod drew it and the vanilla model, armour, held items, cape and
    /// other layers are left out.
    pub(super) fn mod_player_geometry(
        &self,
        mob: &Mob,
        who: NativePlayerRender<'_>,
        partial: f32,
        camera: DVec3,
        light: u32,
        out: &mut Vec<Vertex>,
    ) -> bool {
        // Posing a player for the mods costs, each frame: only when one listens.
        let mut listened = false;
        self.native_player_render_listened(&mut listened);
        if !listened {
            return false;
        }
        self.mod_entity_draw(mob, partial, camera, light, out, |render, draw| {
            let mut drawn = false;
            self.native_player_submit(render, &who, draw, &mut drawn);
            drawn
        })
    }
    /// `native_mob_layer` for `mob` (`who` set for a player): what a mod adds
    /// is kept, drawn after the vanilla model and its layers.
    pub(super) fn mod_mob_layer_geometry(
        &self,
        mob: &Mob,
        who: Option<NativePlayerRender<'_>>,
        partial: f32,
        camera: DVec3,
        light: u32,
        out: &mut Vec<Vertex>,
    ) {
        // Every mob in view each frame: only when a mod listens.
        let mut listened = false;
        self.native_player_render_listened(&mut listened);
        if !listened {
            return;
        }
        self.mod_entity_draw(mob, partial, camera, light, out, |render, draw| {
            self.native_mob_layer(render, who.as_ref(), draw);
            true
        });
    }
    /// Gives `hook` the frame of `mob` and a draw posed at its feet; what it
    /// added to `out` stays when it answers true.
    fn mod_entity_draw(
        &self,
        mob: &Mob,
        partial: f32,
        camera: DVec3,
        light: u32,
        out: &mut Vec<Vertex>,
        hook: impl FnOnce(&NativeMobRender<'_>, &mut BlockEntityDraw) -> bool,
    ) -> bool {
        let render = NativeMobRender::of(mob, partial);
        let a = f64::from(partial);
        let feet = mob.previous.lerp(mob.position, a);
        let feet = DVec3::new(feet.x, feet.y, feet.z);
        let start = out.len();
        let mut draw = BlockEntityDraw::posed(
            partial,
            self.world.game_time,
            light,
            [feet.x, feet.y, feet.z].map(|c| c.floor() as i32),
            mob_pose(feet - camera, render.body_yaw),
            diffuse_lights(self.dim == Dim::Nether),
            std::mem::take(out),
        );
        let keep = hook(&render, &mut draw);
        *out = draw.into_vertices();
        if !keep {
            out.truncate(start);
        }
        keep
    }
}

// How a client entity draws, as native mods decide it (`native_client_entity_render`): hidden,
// translucent, or without its armour. Drawing only; the entity is unchanged.

/// What a mod made of an entity's drawing, for an entity that is drawn at all.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct EntityLook {
    /// 1.0 opaque; below it the entity's quads blend (in `(0.0, 1.0]`).
    pub(super) alpha: f32,
    /// Leave out the worn armour and the head item.
    pub(super) hide_armor: bool,
}

impl EntityLook {
    /// An entity no mod touched.
    pub(super) const VANILLA: Self = Self {
        alpha: 1.0,
        hide_armor: false,
    };

    /// Whether the entity's models can be posed on the GPU: they cannot once their
    /// vertices are faded on the CPU.
    pub(super) fn gpu_posable(self) -> bool {
        self.alpha >= 1.0
    }

    /// Fades the quads an entity drew (`out[start..]`).
    pub(super) fn apply(self, vertices: &mut [Vertex]) {
        fade(vertices, self.alpha);
    }
}

/// What the mods made of the drawing of entity `entity_id` (type `kind`): `None` to skip it.
fn look_of(
    entity_id: u32,
    kind: &str,
    ask: impl FnOnce(u32, &str, &mut f32, &mut bool, &mut bool),
) -> Option<EntityLook> {
    let (mut alpha, mut hide_armor, mut cancel) = (1.0, false, false);
    ask(entity_id, kind, &mut alpha, &mut hide_armor, &mut cancel);
    // NaN counts as untouched; the shader drops anything under 0.1 anyway.
    let alpha = if alpha.is_nan() { 1.0 } else { alpha.min(1.0) };
    (!cancel && alpha > 0.0).then_some(EntityLook { alpha, hide_armor })
}

/// `alpha` times each quad's colour alpha, in the translucent entity pass
/// (`RenderTypes.entityTranslucent`, as a spectator's body is drawn).
pub(super) fn fade(vertices: &mut [Vertex], alpha: f32) {
    if alpha >= 1.0 {
        return;
    }
    for vertex in vertices.iter_mut() {
        let a = ((vertex.color >> 24) as f32 * alpha).round() as u32;
        vertex.color = vertex.color & 0x00FF_FFFF | a << 24;
    }
    horizon_client::gfx::mark_oit(vertices);
}

impl Game<'_> {
    /// `native_client_entity_render` for an entity about to draw: how to draw it, or `None`
    /// to skip it. With no native mod installed, nothing is asked.
    pub(super) fn entity_look(&self, entity_id: u32, kind: &str) -> Option<EntityLook> {
        if horizon_client::mod_api::mods().is_empty() {
            return Some(EntityLook::VANILLA);
        }
        look_of(entity_id, kind, |id, kind, alpha, hide_armor, cancel| {
            self.native_client_entity_render(id, kind, alpha, hide_armor, cancel);
        })
    }
}

/// The client entity render hook (API @API:client_entity_render@).
impl Game<'_> {
    /// Before a client entity is drawn: whether it draws at all, how opaque, and whether
    /// its armour shows (NeoForge's `RenderLivingEvent.Pre` and `EntityRenderer.shouldRender`).
    /// `alpha` comes in as 1.0 and `hide_armor` as false; set `alpha` below 1.0 to draw the
    /// entity translucent (at or below 0.1 nothing of it shows, as the entity shader's alpha
    /// cutoff), `hide_armor` to leave out its worn armour and head item (`HumanoidArmorLayer`,
    /// `CustomHeadLayer`), or `cancel` to skip it altogether: its model, layers, held items and
    /// glowing outline. It changes only what this client draws; the entity stays in the world,
    /// and its name tag, fire and shadow are decided apart (`native_render_name_tag`).
    /// Asked each frame for every entity that is about to draw, after the distance and view
    /// checks: mobs, other players and yourself in third person, armor stands, dropped items,
    /// projectiles, and block, item and text displays. `kind` is the entity type
    /// (`minecraft:armor_stand`). `alpha` and `hide_armor` apply where the entity has something
    /// to fade or armour to hide; a text display ignores `alpha`. API @API:client_entity_render@.
    pub(super) fn native_client_entity_render(
        &self,
        entity_id: u32,
        kind: &str,
        alpha: &mut f32,
        hide_armor: &mut bool,
        cancel: &mut bool,
    ) {
        mod_api::dispatch_client_entity_render(
            self,
            mod_api::mods(),
            entity_id,
            kind,
            alpha,
            hide_armor,
            cancel,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    fn close(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < 1e-5
    }

    #[test]
    fn the_pose_faces_where_the_body_faces() {
        // Entity yaw 0 faces south (+z), 90 west (-x), 180 north, 270 east.
        for (yaw, ahead, left) in [
            (0.0, Vec3::Z, Vec3::X),
            (90.0, Vec3::NEG_X, Vec3::Z),
            (180.0, Vec3::NEG_Z, Vec3::NEG_X),
            (270.0, Vec3::X, Vec3::NEG_Z),
        ] {
            let pose = mob_pose(DVec3::ZERO, yaw);
            assert!(close(pose.transform_vector3(Vec3::Z), ahead), "{yaw}");
            assert!(close(pose.transform_vector3(Vec3::X), left), "{yaw}");
            assert!(close(pose.transform_vector3(Vec3::Y), Vec3::Y), "{yaw}");
        }
    }

    #[test]
    fn the_pose_starts_at_the_feet() {
        let at = DVec3::new(3.0, -1.5, 0.25);
        let pose = mob_pose(at, 37.0);
        assert!(close(pose.transform_point3(Vec3::ZERO), at.as_vec3()));
    }
}

#[cfg(test)]
mod look_tests {
    use super::*;

    #[test]
    fn untouched_entities_draw_as_vanilla() {
        assert_eq!(
            look_of(7, "minecraft:zombie", |_, _, _, _, _| {}),
            Some(EntityLook::VANILLA)
        );
    }

    #[test]
    fn a_mod_can_fade_hide_armour_or_cancel() {
        let faded = look_of(7, "minecraft:armor_stand", |id, kind, alpha, hide, _| {
            assert_eq!((id, kind), (7, "minecraft:armor_stand"));
            *alpha = 0.4;
            *hide = true;
        });
        assert_eq!(
            faded,
            Some(EntityLook {
                alpha: 0.4,
                hide_armor: true
            })
        );
        assert_eq!(look_of(7, "", |_, _, _, _, cancel| *cancel = true), None);
        // Fully transparent is not drawn; more than opaque and NaN are opaque.
        assert_eq!(look_of(7, "", |_, _, alpha, _, _| *alpha = 0.0), None);
        assert_eq!(look_of(7, "", |_, _, alpha, _, _| *alpha = -1.0), None);
        let vanilla = |a: f32| look_of(7, "", move |_, _, alpha, _, _| *alpha = a);
        assert_eq!(vanilla(3.0), Some(EntityLook::VANILLA));
        assert_eq!(vanilla(f32::NAN), Some(EntityLook::VANILLA));
    }

    #[test]
    fn fading_scales_alpha_and_keeps_colour() {
        let mut quads = [Vertex {
            color: 0xFF80_4020,
            ..bytemuck::Zeroable::zeroed()
        }];
        fade(&mut quads, 0.5);
        assert_eq!(quads[0].color, 0x8080_4020);
        assert_eq!(
            quads[0].layer & horizon_client::gfx::OIT_ENTITY,
            horizon_client::gfx::OIT_ENTITY
        );
        // Opaque leaves the quads alone, without the translucent tag.
        let mut opaque = [Vertex {
            color: 0xFF80_4020,
            ..bytemuck::Zeroable::zeroed()
        }];
        fade(&mut opaque, 1.0);
        assert_eq!((opaque[0].color, opaque[0].layer), (0xFF80_4020, 0));
    }
}
