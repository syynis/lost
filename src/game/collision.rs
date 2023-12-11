use bevy::{log, prelude::*};
use bevy_pile::grid::Grid;

use super::{
    level::Levels, level_select::CurrentLevel, Dir, EntityKind, GameAssets, GameState, TilePos,
};

pub struct CollisionPlugin;

impl Plugin for CollisionPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<CollisionMap>()
            .add_systems(
                OnTransition {
                    from: GameState::LevelTransition,
                    to: GameState::Play,
                },
                init_collision_map,
            )
            .add_systems(
                PostUpdate,
                sync_collision_map.run_if(in_state(GameState::Play)),
            );
    }
}

#[derive(Reflect, Clone, Default)]
pub enum CollisionEntry {
    #[default]
    Free,
    Occupied {
        entity: Entity,
        kind: EntityKind,
    },
}

#[derive(Resource, Reflect, Deref, DerefMut)]
#[reflect(Resource)]
pub struct CollisionMap(pub Grid<CollisionEntry>);

impl Default for CollisionMap {
    fn default() -> Self {
        CollisionMap(Grid::new(IVec2::new(0, 0), CollisionEntry::Free))
    }
}

pub fn init_collision_map(
    mut cmds: Commands,
    current_level: Res<CurrentLevel>,
    levels: Res<Assets<Levels>>,
    assets: Res<GameAssets>,
    sokoban_entities: Query<(Entity, &TilePos, &EntityKind)>,
) {
    // TODO Make some system param for this
    let size = levels
        .get(&assets.levels)
        .expect("Level assets should be loaded")
        .get(**current_level)
        .expect("Current level will always be a valid level index")
        .size;
    log::debug!("Initialized collision map");
    let mut map = Grid::new(
        IVec2::new(size.x as i32, size.y as i32),
        CollisionEntry::Free,
    );
    for (entity, pos, kind) in sokoban_entities.iter() {
        map.set(
            **pos,
            CollisionEntry::Occupied {
                entity,
                kind: *kind,
            },
        );
    }
    cmds.insert_resource(CollisionMap(map));
}

// TODO dont rebuild but instead only change moved entities
fn sync_collision_map(
    mut collision: ResMut<CollisionMap>,
    sokoban_entities: Query<(Entity, &TilePos, &EntityKind)>,
) {
    collision.iter_mut().for_each(|(_, elem)| {
        *elem = CollisionEntry::Free;
    });
    for (entity, pos, kind) in sokoban_entities.iter() {
        collision.0.set(
            **pos,
            CollisionEntry::Occupied {
                entity,
                kind: *kind,
            },
        );
    }
}

pub enum CollisionResult {
    Push(Vec<Entity>),
    Collision,
    OutOfBounds,
}

impl CollisionMap {
    pub fn push_collision(&self, pusher_pos: IVec2, direction: Dir) -> CollisionResult {
        let Some(CollisionEntry::Occupied {
            entity: pusher,
            kind: _,
        }) = self.get(pusher_pos)
        else {
            return CollisionResult::OutOfBounds;
        };

        let move_in_dir = |pos| -> IVec2 { pos + IVec2::from(direction) };
        let mut is_player = true;
        let mut moving_entities = Vec::new();
        let mut current_pos = pusher_pos;
        let mut dest = move_in_dir(current_pos);
        let mut pusher = pusher;
        'outer: while let Some(dest_entity) = self.get(dest) {
            match dest_entity {
                CollisionEntry::Free => {
                    moving_entities.push(*pusher);
                    break;
                }
                CollisionEntry::Occupied {
                    entity: pushed,
                    kind,
                } => match kind {
                    EntityKind::Obstacle => return CollisionResult::Collision,
                    EntityKind::ObstaclePlayer => {
                        if is_player {
                            return CollisionResult::Collision;
                        } else {
                            moving_entities.push(*pusher);
                            break 'outer;
                        }
                    }
                    EntityKind::ObstacleBlock => {
                        if is_player {
                            moving_entities.push(*pusher);
                            break 'outer;
                        } else {
                            return CollisionResult::Collision;
                        }
                    }
                    EntityKind::Pullable => {
                        moving_entities.push(*pusher);
                        if !is_player {
                            break 'outer;
                        }
                        pusher = pushed;
                        current_pos = dest;
                        dest = move_in_dir(current_pos);
                        is_player = false;
                    }
                    EntityKind::Pushable => {
                        moving_entities.push(*pusher);
                        if !is_player {
                            break 'outer;
                        }
                        pusher = pushed;
                        current_pos = dest;
                        dest = move_in_dir(current_pos);
                        is_player = false;
                    }
                },
            }
        }
        CollisionResult::Push(moving_entities)
    }
}
