use std::ops::AddAssign;

use bevy::{ecs::world::Command, log, prelude::*};
use leafwing_input_manager::prelude::*;

use super::{
    collision::CollisionMap,
    history::{HandleHistoryEvents, HistoryBundle, HistoryEvent},
    Dir, GameAssets, GameState, SpriteOffset, TilePos,
};

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(InputManagerPlugin::<PlayerAction>::default())
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                player_movement
                    .before(HandleHistoryEvents)
                    .run_if(in_state(GameState::Play)),
            );
    }
}

#[derive(Component, Clone)]
pub struct Player;

#[derive(Actionlike, Clone, Copy, Hash, Debug, PartialEq, Eq, Reflect)]
pub enum PlayerAction {
    Up,
    Right,
    Down,
    Left,
}

impl From<PlayerAction> for Dir {
    fn from(value: PlayerAction) -> Dir {
        match value {
            PlayerAction::Up => Dir::Up,
            PlayerAction::Left => Dir::Left,
            PlayerAction::Down => Dir::Down,
            PlayerAction::Right => Dir::Right,
        }
    }
}

pub struct SpawnPlayer {
    pub pos: TilePos,
    pub tilemap_entity: Entity,
}

impl SpawnPlayer {
    pub fn new(pos: TilePos, tilemap_entity: Entity) -> Self {
        Self {
            pos,
            tilemap_entity,
        }
    }
}

impl Command for SpawnPlayer {
    fn apply(self, world: &mut World) {
        let texture = world.resource::<GameAssets>().player.clone();
        world
            .entity_mut(self.tilemap_entity)
            .with_children(|child_builder| {
                child_builder.spawn((
                    Name::new("Player"),
                    Player,
                    self.pos,
                    HistoryBundle::<TilePos>::default(),
                    SpriteBundle {
                        texture,
                        ..default()
                    },
                    SpriteOffset(Vec2::Y * 4.),
                    MovementTimer::default(),
                ));
            });
    }
}

fn setup(mut cmds: Commands) {
    cmds.spawn((
        (InputManagerBundle::<PlayerAction> {
            input_map: player_actions(),
            ..default()
        },),
        Name::new("PlayerActions"),
    ));
}

fn player_actions() -> InputMap<PlayerAction> {
    use PlayerAction::*;
    let mut input_map = InputMap::default();

    input_map.insert(Up, KeyCode::KeyW);
    input_map.insert(Right, KeyCode::KeyD);
    input_map.insert(Down, KeyCode::KeyS);
    input_map.insert(Left, KeyCode::KeyA);

    input_map
}

#[derive(Clone, Debug, Component, Deref, DerefMut)]
pub struct MovementTimer(pub Timer);

impl Default for MovementTimer {
    fn default() -> MovementTimer {
        MovementTimer(Timer::from_seconds(0.2, TimerMode::Once))
    }
}

pub fn player_movement(
    mut player_q: Query<(Entity, &mut MovementTimer), With<Player>>,
    mut dynamic_entities: Query<&mut TilePos>,
    mut history_events: EventWriter<HistoryEvent>,
    player_actions: Query<&ActionState<PlayerAction>>,
    time: Res<Time>,
    collision: Res<CollisionMap>,
) {
    let Ok((player_entity, mut movement_timer)) = player_q.get_single_mut() else {
        return;
    };

    let player_actions = player_actions
        .get_single()
        .expect("Player input map should exist");

    movement_timer.tick(time.delta());

    if !movement_timer.finished() {
        return;
    }

    let player_pos = dynamic_entities
        .get(player_entity)
        .expect("Player always has tile pos")
        .0;

    let mut moved = false;
    for direction in player_actions
        .get_pressed()
        .iter()
        .map(|action| Dir::from(*action))
    {
        movement_timer.reset();

        match collision.player_push_collision(player_entity, player_pos, direction) {
            super::collision::CollisionResult::Push(push) => {
                moved = true;
                let dir_vec = IVec2::from(direction);
                for e in push {
                    dynamic_entities
                        .get_mut(e)
                        .expect("Every entity in collision map has tile pos")
                        .0
                        .add_assign(dir_vec);
                }
            }
            super::collision::CollisionResult::Collision => {
                log::debug!("Can't move");
            }
        }
    }

    if moved {
        history_events.send(HistoryEvent::Record);
    }
}
