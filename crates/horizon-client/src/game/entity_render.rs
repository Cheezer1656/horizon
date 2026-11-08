//! How a client entity draws, as native mods decide it (`native_client_entity_render`):
//! hidden, translucent, or without its armour. Drawing only; the entity is unchanged.
use super::{Game, Vertex};

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

#[cfg(test)]
mod tests {
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
