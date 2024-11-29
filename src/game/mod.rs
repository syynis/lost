use bevy::prelude::*;
use bevy_asset_loader::prelude::*;
use leafwing_input_manager::prelude::*;

use crate::cleanup;

use self::{
    history::{History, HistoryEvent, PreviousComponent},
    level::{LevelData, LevelLoader, Levels},
    mechanics::Pit,
};

pub mod collision;
pub mod history;
pub mod level;
pub mod level_select;
pub mod level_transition;
pub mod mechanics;
pub mod player;
pub mod util;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            InputManagerPlugin::<GameAction>::default(),
            level_select::LevelSelectPlugin,
            level_transition::LevelTransitionPlugin,
            player::PlayerPlugin,
            collision::CollisionPlugin,
            level::LevelPlugin,
            history::HistoryPlugin,
            history::HistoryComponentPlugin::<TilePos>::default(),
            history::PreviousComponentPlugin::<TilePos>::default(),
            mechanics::MechanicsPlugin,
            cleanup::StateCleanupPlugin::<GameState>::default(),
        ));
        app.register_asset_loader(LevelLoader)
            .init_asset::<Levels>();
        app.register_type::<TilePos>()
            .register_type::<Dir>()
            .register_type::<History<TilePos>>()
            .register_type::<PreviousComponent<TilePos>>()
            .register_type::<EntityKind>();
        app.init_state::<GameState>()
            .add_loading_state(
                LoadingState::new(GameState::AssetLoading)
                    .continue_to_state(GameState::LevelSelect),
            )
            .configure_loading_state(
                LoadingStateConfig::new(GameState::AssetLoading).load_collection::<GameAssets>(),
            );
        app.add_systems(Startup, setup)
            .add_systems(
                Update,
                (history, navigation).run_if(in_state(GameState::Play)),
            )
            .add_systems(
                PostUpdate,
                (
                    entity_kind_components,
                    apply_deferred,
                    copy_pos_to_transform,
                )
                    .chain()
                    .run_if(in_state(GameState::Play)),
            );
    }
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum GameState {
    #[default]
    AssetLoading,
    LevelSelect,
    LevelTransition,
    Play,
}

#[derive(Resource, Default, AssetCollection, Debug)]
pub struct GameAssets {
    #[asset(path = "player.png")]
    pub player: Handle<Image>,
    #[asset(path = "pushable.png")]
    pub pushable: Handle<Image>,
    #[asset(path = "pullable.png")]
    pub pullable: Handle<Image>,
    #[asset(texture_atlas(tile_size_x = 16, tile_size_y = 16, columns = 8, rows = 3))]
    pub layout: Handle<TextureAtlasLayout>,
    #[asset(path = "tiles.png")]
    pub tiles: Handle<Image>,
    #[asset(path = "button.png")]
    pub button: Handle<Image>,
    #[asset(path = "test.levels.ron")]
    pub levels: Handle<Levels>,
}

pub fn entity_kind_components(
    mut cmds: Commands,
    query: Query<(Entity, &EntityKind), Added<EntityKind>>,
    assets: Res<GameAssets>,
) {
    for (entity, kind) in query.iter() {
        match kind {
            EntityKind::Wall => {
                cmds.entity(entity).insert(Name::new("Wall"));
            }
            EntityKind::Pit => {
                cmds.entity(entity).insert((Name::new("Pit"), Pit));
            }
            EntityKind::Platform => {
                cmds.entity(entity).insert(Name::new("Platform"));
            }
            EntityKind::Pullable => {
                cmds.entity(entity).insert((
                    Name::new("Pullable"),
                    SpriteBundle {
                        texture: assets.pullable.clone_weak(),
                        ..default()
                    },
                ));
            }
            EntityKind::Pushable => {
                cmds.entity(entity).insert((
                    Name::new("Pushable"),
                    SpriteBundle {
                        texture: assets.pushable.clone_weak(),
                        ..default()
                    },
                ));
            }
        }
    }
}
pub fn copy_pos_to_transform(
    level_data: LevelData,
    mut query: Query<(&TilePos, &mut Transform, Option<&SpriteOffset>), Changed<TilePos>>,
) {
    for (pos, mut transform, offset) in query.iter_mut() {
        let new_pos = pos.wpos() + offset.map_or(Vec2::ZERO, |offset| **offset);

        transform.translation = new_pos.extend(level_data.size().y as f32 - pos.y as f32);
    }
}

fn setup(mut cmds: Commands) {
    cmds.spawn((
        (InputManagerBundle::<GameAction> {
            input_map: game_actions(),
            ..default()
        },),
        Name::new("GameActions"),
    ));
}

fn navigation(
    actions: Query<&ActionState<GameAction>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let Ok(actions) = actions.get_single() else {
        return;
    };

    if actions.just_pressed(&GameAction::ToLevelSelect) {
        next_state.set(GameState::LevelSelect);
    }
}

fn history(
    actions: Query<&ActionState<GameAction>>,
    mut history_events: EventWriter<HistoryEvent>,
) {
    let Ok(actions) = actions.get_single() else {
        return;
    };
    if actions.just_pressed(&GameAction::Undo) {
        history_events.send(HistoryEvent::Rewind);
    } else if actions.just_pressed(&GameAction::Reset) {
        history_events.send(HistoryEvent::Reset);
    }
}

#[derive(Actionlike, Clone, Copy, Hash, Debug, PartialEq, Eq, Reflect)]
pub enum GameAction {
    Undo,
    Reset,
    ToLevelSelect,
}

fn game_actions() -> InputMap<GameAction> {
    use GameAction::*;
    let mut input_map: InputMap<GameAction> = InputMap::default();

    input_map.insert(Undo, KeyCode::KeyE);
    input_map.insert(Reset, KeyCode::KeyR);
    input_map.insert(ToLevelSelect, KeyCode::KeyG);

    input_map
}

#[derive(Component, Default, Clone, Copy, Debug, PartialEq, Eq, Deref, DerefMut, Reflect)]
pub struct TilePos(pub IVec2);

impl TilePos {
    pub fn new(x: i32, y: i32) -> Self {
        TilePos(IVec2::new(x, y))
    }

    pub fn add_dir(&mut self, dir: Dir) {
        self.0 += IVec2::from(dir);
    }

    pub fn wpos(&self) -> Vec2 {
        self.as_vec2() * 16.
    }
}

#[derive(Debug, Copy, Clone, Reflect)]
pub enum Dir {
    Up,
    Right,
    Down,
    Left,
}

impl Dir {
    pub fn opposite(&self) -> Dir {
        use Dir::*;
        match self {
            Up => Down,
            Down => Up,
            Left => Right,
            Right => Left,
        }
    }
}

impl From<Dir> for IVec2 {
    fn from(direction: Dir) -> IVec2 {
        match direction {
            Dir::Up => IVec2::Y,
            Dir::Left => IVec2::NEG_X,
            Dir::Down => IVec2::NEG_Y,
            Dir::Right => IVec2::X,
        }
    }
}

#[derive(Component, Deref, DerefMut, Default, Reflect, Clone, Copy)]
pub struct SpriteOffset(pub Vec2);

#[derive(Debug, Copy, Clone, Component, Reflect, PartialEq)]
pub enum EntityKind {
    Wall,
    Pit,
    Platform,
    Pullable,
    Pushable,
}
