use bevy::{log, prelude::*, utils::hashbrown::HashSet};

use super::{
    collision::CollisionMap,
    player::Player,
    util::{CARDINALS, CARDINALS_DIR},
    EntityKind, GameState, TilePos,
};

pub struct MechanicsPlugin;

impl Plugin for MechanicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (despawn_on_pit, check_win).run_if(in_state(GameState::Play)),
        );
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

fn check_win(
    player_q: Query<&TilePos, With<Player>>,
    entity_q: Query<(&TilePos, &EntityKind), Without<Player>>,
    collision: Res<CollisionMap>,
) {
    let Ok(player_pos) = player_q.get_single() else {
        return;
    };

    let mut check_for_reachable = Vec::new();
    for (entity_pos, kind) in entity_q.iter() {
        let entity_pos = **entity_pos;
        match kind {
            EntityKind::Pullable => {
                for dir in CARDINALS_DIR.iter() {
                    let dest = entity_pos + IVec2::from(*dir);
                    let dest2 = dest + IVec2::from(*dir);
                    if !collision.is_blocked(dest, true)
                        && !collision.is_blocked(dest2, true)
                        && !collision.is_blocked(dest, false)
                    {
                        check_for_reachable.push(dest);
                    }
                }
            }
            EntityKind::Pushable => {
                for dir in CARDINALS_DIR.iter() {
                    let dest = entity_pos + IVec2::from(*dir);
                    let opp = entity_pos + IVec2::from(dir.opposite());
                    if !collision.is_blocked(dest, true) && !collision.is_blocked(opp, false) {
                        check_for_reachable.push(dest);
                    }
                }
            }
            _ => {}
        }
    }

    let mut queue = Vec::new();
    queue.push(**player_pos);
    let mut visited = HashSet::new();

    while let Some(next) = queue.pop() {
        visited.insert(next);
        for dir in CARDINALS.iter() {
            let dest = next + *dir;
            if !collision.is_blocked(dest, true) && !visited.contains(&dest) {
                queue.push(dest);
            }
        }
    }

    if check_for_reachable
        .iter()
        .all(|check| !visited.contains(check))
    {
        log::info!("WIN!");
    }
}
