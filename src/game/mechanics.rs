use bevy::prelude::*;

use super::{EntityKind, TilePos};

pub struct MechanicsPlugin;

impl Plugin for MechanicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, despawn_on_pit);
    }
}

#[derive(Component)]
pub struct Pit;
fn despawn_on_pit(
    mut cmds: Commands,
    q: Query<(Entity, &TilePos, &EntityKind)>,
    pit: Query<&TilePos, With<Pit>>,
) {
    for (entity, pos, kind) in q.iter() {
        if matches!(kind, EntityKind::Pushable) && pit.iter().any(|pit_pos| pit_pos == pos) {
            cmds.entity(entity).despawn_recursive();
        }
    }
}
