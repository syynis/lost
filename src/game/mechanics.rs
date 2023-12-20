use bevy::{ecs::system::Command, log, prelude::*, utils::hashbrown::HashSet};
use bevy_simple_tilemap::TileMap;

use super::{
    collision::CollisionMap,
    history::{CurrentTime, HandleHistoryEvents, History, HistoryEvent, PreviousComponent},
    level::LevelRoot,
    player::Player,
    util::{CARDINALS, CARDINALS_DIR},
    EntityKind, GameState, TilePos,
};

pub struct MechanicsPlugin;

impl Plugin for MechanicsPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<HistoryStore>()
            .register_type::<DespawnHistory>();
        app.init_resource::<DespawnHistory>();
        app.add_systems(
            Update,
            (
                despawn_on_pit,
                check_win,
                (rewind, apply_deferred).chain().before(HandleHistoryEvents),
            )
                .run_if(in_state(GameState::Play)),
        );
    }
}

#[derive(Component)]
pub struct Pit;
fn despawn_on_pit(
    mut cmds: Commands,
    q: Query<(Entity, &TilePos, &EntityKind)>,
    pit: Query<(Entity, &TilePos), With<Pit>>,
    mut tilemap: Query<&mut TileMap>,
) {
    let Ok(mut tilemap) = tilemap.get_single_mut() else {
        return;
    };
    for (entity, pos, kind) in q.iter() {
        if matches!(kind, EntityKind::Pushable) {
            if let Some((pit_entity, _)) = pit.iter().find(|(_, pit_pos)| *pit_pos == pos) {
                cmds.add(DespawnSokobanEntityCommand(entity));
                cmds.entity(pit_entity).despawn_recursive();
                tilemap.set_tile(
                    pos.extend(1),
                    Some(bevy_simple_tilemap::Tile {
                        sprite_index: 16,
                        ..default()
                    }),
                );
            }
        }
    }
}

pub struct DespawnSokobanEntityCommand(pub Entity);

impl Command for DespawnSokobanEntityCommand {
    fn apply(self, world: &mut World) {
        let (pos, history, previous, kind) = world
            .query::<(
                &TilePos,
                &History<TilePos>,
                &PreviousComponent<TilePos>,
                &EntityKind,
            )>()
            .get(world, self.0)
            .expect("test");
        let (pos, history, previous, kind) = (*pos, history.clone(), previous.clone(), *kind);

        let level_entity = world
            .query_filtered::<Entity, With<LevelRoot>>()
            .get_single(world)
            .expect("Level should exist");

        let current_time = *world.resource::<CurrentTime>();
        world.resource_scope(|world, mut despawn_history: Mut<DespawnHistory>| {
            let despawn = HistoryStore {
                pos,
                history,
                kind,
                previous,
                level_entity,
            };
            despawn_history.push((*current_time, despawn));
            world.despawn(self.0);
        });
    }
}

pub fn rewind(
    mut cmds: Commands,
    mut history_events: EventReader<HistoryEvent>,
    mut command_history: ResMut<DespawnHistory>,
    current_time: Res<CurrentTime>,
    mut tilemap: Query<&mut TileMap>,
) {
    let Ok(mut tilemap) = tilemap.get_single_mut() else {
        return;
    };
    for ev in history_events.read() {
        match ev {
            HistoryEvent::Record => {}
            HistoryEvent::Rewind => {
                while let Some((time, despawn)) = command_history.pop() {
                    if time == **current_time {
                        cmds.entity(despawn.level_entity).with_children(|parent| {
                            parent.spawn((
                                despawn.pos,
                                despawn.history,
                                despawn.previous,
                                despawn.kind,
                            ));
                        });
                        cmds.entity(despawn.level_entity).with_children(|parent| {
                            parent.spawn((despawn.pos, EntityKind::Pit));
                        });
                        tilemap.set_tile(
                            despawn.pos.extend(1),
                            Some(bevy_simple_tilemap::Tile {
                                sprite_index: 18,
                                ..default()
                            }),
                        );
                    } else {
                        command_history.push((time, despawn));
                        break;
                    }
                }
            }
            HistoryEvent::Reset => {}
        }
    }
}

#[derive(Reflect)]
pub struct HistoryStore {
    pub pos: TilePos,
    pub history: History<TilePos>,
    pub previous: PreviousComponent<TilePos>,
    pub kind: EntityKind,
    pub level_entity: Entity,
}

#[derive(Resource, Default, Reflect, Deref, DerefMut)]
#[reflect(Resource)]
pub struct DespawnHistory(Vec<(usize, HistoryStore)>);

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
                    if !(collision.is_blocked(dest, true)
                        || collision.is_blocked(dest2, true)
                        || collision.is_blocked(dest, false))
                    {
                        check_for_reachable.push(dest);
                    }
                }
            }
            EntityKind::Pushable => {
                for dir in CARDINALS_DIR.iter() {
                    let dest = entity_pos + IVec2::from(*dir);
                    let opp = entity_pos + IVec2::from(dir.opposite());
                    if !(collision.is_blocked(dest, true) || collision.is_blocked(opp, false)) {
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
