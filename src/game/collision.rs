use bevy::{log, prelude::*};

use crate::grid::Grid;

use super::{
    level::Levels, level_select::CurrentLevel, Dir, EntityKind, GameAssets, GameState, TilePos,
};

pub struct CollisionPlugin;

impl Plugin for CollisionPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<CollisionMap>()
            .add_systems(
                OnTransition {
                    exited: GameState::LevelTransition,
                    entered: GameState::Play,
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
}

impl CollisionMap {
    pub fn is_blocked(&self, pos: IVec2, player: bool) -> bool {
        self.get(pos).map_or(true, |entry| match entry {
            CollisionEntry::Free => false,
            CollisionEntry::Occupied { entity: _, kind } => {
                matches!(
                    kind,
                    EntityKind::Wall | EntityKind::Pullable | EntityKind::Pushable
                ) || if player {
                    matches!(kind, EntityKind::Pit)
                } else {
                    matches!(kind, EntityKind::Platform)
                }
            }
        })
    }

    pub fn player_push_collision(
        &self,
        pusher: Entity,
        pusher_pos: IVec2,
        direction: Dir,
    ) -> CollisionResult {
        let move_in_dir = |pos, dir| -> IVec2 { pos + IVec2::from(dir) };
        let dest = move_in_dir(pusher_pos, direction);
        let mut moving_entities = Vec::new();
        if let Some(CollisionEntry::Occupied {
            entity: pushed,
            kind,
        }) = self.get(dest)
        {
            match kind {
                EntityKind::Pushable => {
                    if self.is_blocked(move_in_dir(dest, direction), false) {
                        return CollisionResult::Collision;
                    } else {
                        moving_entities.push(*pushed)
                    }
                }
                EntityKind::Wall | EntityKind::Pullable | EntityKind::Pit => {
                    return CollisionResult::Collision
                }
                EntityKind::Platform => {}
            }
        }
        let opp = move_in_dir(pusher_pos, direction.opposite());
        if !self.is_blocked(pusher_pos, false) {
            if let Some(CollisionEntry::Occupied { entity, kind }) = self.get(opp) {
                if matches!(kind, EntityKind::Pullable) {
                    moving_entities.push(*entity);
                }
            }
        }
        moving_entities.push(pusher);
        CollisionResult::Push(moving_entities)
    }
}
