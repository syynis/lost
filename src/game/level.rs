use std::future::Future;

use bevy::{
    asset::{AssetLoader, AsyncReadExt},
    ecs::system::SystemParam,
    prelude::*,
    reflect::TypePath,
    utils::ConditionalSendFuture,
};
use bevy_simple_tilemap::prelude::*;
use bevy_simple_tilemap::TileFlags;
use serde::Deserialize;
use thiserror::Error;

use crate::{cleanup::DependOnState, grid::Grid};

use super::{
    collision::init_collision_map, history::HistoryBundle, level_select::CurrentLevel,
    player::SpawnPlayer, util::DIRS, EntityKind, GameAssets, GameState, TilePos,
};

pub struct LevelPlugin;

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(SimpleTileMapPlugin);
        app.register_type::<Level>();
        app.add_systems(
            OnTransition {
                exited: GameState::LevelTransition,
                entered: GameState::Play,
            },
            (spawn_level, apply_deferred)
                .chain()
                .before(init_collision_map),
        )
        .add_systems(
            Update,
            reload_on_change
                .run_if(in_state(GameState::Play))
                .run_if(on_event::<AssetEvent<Levels>>()),
        );
    }
}

#[derive(SystemParam)]
pub struct LevelData<'w> {
    current_level: Res<'w, CurrentLevel>,
    levels: Res<'w, Assets<Levels>>,
    assets: Res<'w, GameAssets>,
}

impl<'w> LevelData<'w> {
    pub fn levels(&self) -> &Levels {
        self.levels
            .get(&self.assets.levels)
            .expect("Level handle should be loaded")
    }

    pub fn current_level_data(&self) -> &Level {
        self.levels()
            .get(self.current_level())
            .expect("Current level should only ever be set to a valid level")
    }

    pub fn current_level(&self) -> usize {
        **self.current_level
    }

    pub fn size(&self) -> UVec2 {
        self.current_level_data().size
    }

    pub fn amount_levels(&self) -> usize {
        self.levels().len()
    }
}

#[derive(Component)]
pub struct LevelRoot;

fn spawn_level(mut cmds: Commands, level_data: LevelData, assets: Res<GameAssets>) {
    let level = level_data.current_level_data();

    let level_root = cmds
        .spawn((
            SpatialBundle::default(),
            // DependOnState(vec![GameState::Play, GameState::Pause]),
            DependOnState::single(GameState::Play),
            Name::new("Level Root"),
            LevelRoot,
        ))
        .id();
    let tilemap_entity = cmds.spawn_empty().id();
    let wall_entity = cmds.spawn_empty().id();
    let sub_wall_entity = cmds.spawn_empty().id();

    let mut tiles = Vec::new();
    let mut wall_tiles = Vec::new();
    let mut sub_wall_tiles = Vec::new();
    for (idx, tile) in level.tiles.iter().enumerate() {
        let x = idx as i32 % level.size.x as i32;
        let y = idx as i32 / level.size.x as i32;
        let pos = TilePos(IVec2::new(x, y));

        let (sprite_index, flags) = tile.index_flip(&pos, level);

        // Tile is not walkable and above us is static tile
        if !tile.is_static() && level.tiles[idx + level.size.x as usize].is_static() {
            sub_wall_tiles.push((
                IVec3::new(x, y, y),
                Some(Tile {
                    sprite_index: 14,
                    ..default()
                }),
            ));
        }

        let elem = (
            IVec3::new(x, y, y),
            Some(Tile {
                sprite_index,
                flags,
                ..default()
            }),
        );
        if tile.is_static() {
            wall_tiles.push(elem);
        } else {
            tiles.push(elem);
        }

        match tile {
            TileKind::Wall | TileKind::Platform | TileKind::Pit => {
                cmds.entity(wall_entity).with_children(|parent| {
                    parent.spawn((pos, tile.entity_kind().unwrap()));
                });
            }
            TileKind::Floor => {}
            TileKind::Player => cmds.add(SpawnPlayer {
                pos,
                tilemap_entity: level_root,
            }),
            TileKind::Pushable | TileKind::Pullable => {
                cmds.entity(level_root).with_children(|parent| {
                    parent.spawn((
                        pos,
                        tile.entity_kind().unwrap(),
                        HistoryBundle::<TilePos>::default(),
                    ));
                });
            }
        }
    }

    let mut tilemap = TileMap::default();
    let mut walls = TileMap::default();
    let mut sub_walls = TileMap::default();
    tilemap.set_tiles(tiles);
    walls.set_tiles(wall_tiles);
    sub_walls.set_tiles(sub_wall_tiles);
    let tilemap_entity = cmds
        .entity(tilemap_entity)
        .insert((
            TileMapBundle {
                tilemap,
                atlas: assets.layout.clone_weak().into(),
                texture: assets.tiles.clone_weak(),
                transform: Transform::from_translation(Vec3::NEG_Z),
                ..default()
            },
            Name::new(format!("Level {}", level_data.current_level())),
        ))
        .id();

    let walls_entity = cmds
        .entity(wall_entity)
        .insert((
            TileMapBundle {
                tilemap: walls,
                atlas: assets.layout.clone_weak().into(),
                texture: assets.tiles.clone_weak(),
                transform: Transform::from_translation(
                    8. * Vec3::Y + level.size.y as f32 * Vec3::Z,
                ),
                ..default()
            },
            Name::new("Walls"),
        ))
        .id();
    let sub_walls_entity = cmds
        .entity(sub_wall_entity)
        .insert((
            TileMapBundle {
                tilemap: sub_walls,
                atlas: assets.layout.clone_weak().into(),
                texture: assets.tiles.clone_weak(),
                transform: Transform::from_translation(8. * Vec3::Y + Vec3::NEG_Z * 0.5),
                ..default()
            },
            Name::new("Walls"),
        ))
        .id();

    cmds.entity(level_root).add_child(tilemap_entity);
    cmds.entity(level_root).add_child(walls_entity);
    cmds.entity(level_root).add_child(sub_walls_entity);
}

fn calculate_wall_index(pos: IVec2, level: &Level) -> (u32, TileFlags) {
    let level_grid = Grid::from_raw(level.size.as_ivec2(), level.tiles.clone());
    let [n, ne, e, se, s, sw, w, nw]: [bool; 8] = DIRS
        .iter()
        .map(|dir| {
            let npos = pos + *dir;
            level_grid.get(npos).map_or(false, |tile| !tile.is_static())
        })
        .collect::<Vec<bool>>()
        .try_into()
        .unwrap();

    let diag_c: usize = [ne, se, sw, nw].iter().map(|x| *x as usize).sum();
    let card_c: usize = [n, e, s, w].iter().map(|x| *x as usize).sum();

    #[derive(Default, Debug)]
    struct TileFlip {
        pub x: bool,
        pub y: bool,
        pub d: bool,
    }

    let flip_to_flag = |tileflip: TileFlip| -> TileFlags {
        let mut empty = TileFlags::empty();
        if tileflip.x {
            empty.insert(TileFlags::FLIP_X);
        }
        if tileflip.y {
            empty.insert(TileFlags::FLIP_Y);
        }
        if tileflip.d {
            empty.insert(TileFlags::FLIP_D);
        }
        empty
    };
    let flip = TileFlip {
        x: e,
        y: s,
        d: e || w,
    };
    let flip_inv = TileFlip {
        x: e,
        y: s,
        d: !e || !w,
    };
    let two_diag = TileFlip {
        x: ne,
        y: sw,
        d: (nw && sw) || (ne && se), // Both vertical
    };
    let zero_flip_diag = TileFlip {
        x: ne || se,
        y: sw || se,
        ..default()
    };
    let three_diag_flip = TileFlip {
        x: e || n && se || s && ne,
        y: s || e && sw || w && se,
        d: e || w,
    };
    let (id, flip) = match card_c {
        0 => match diag_c {
            0 => (8, TileFlip::default()),
            1 => (9, zero_flip_diag),
            2 => {
                if (nw && se) || (ne && sw) {
                    (11, two_diag)
                } else {
                    (10, two_diag)
                }
            }
            3 => (12, zero_flip_diag),
            4 => (13, TileFlip::default()),
            _ => unreachable!(),
        },
        1 => {
            if diag_c == 4 {
                (2, flip)
            } else if diag_c == 3 {
                (1, three_diag_flip)
            } else {
                (0, flip)
            }
        }
        2 => {
            if (n && s) || (w && e) {
                (5, flip_inv)
            } else if (n && w && se) || (n && e && sw) || (s && w && ne) || (s && e && nw) {
                (4, flip)
            } else {
                (3, flip)
            }
        }
        3 => (6, flip_inv),
        4 => (7, TileFlip::default()),
        _ => unreachable!(),
    };

    (id, flip_to_flag(flip))
}

fn reload_on_change(
    mut asset_events: EventReader<AssetEvent<Levels>>,
    mut game_state: ResMut<NextState<GameState>>,
) {
    for ev in asset_events.read() {
        match ev {
            AssetEvent::Modified { id: _ } => {
                game_state.set(GameState::LevelTransition);
            }
            AssetEvent::Added { id: _ } => {}
            AssetEvent::Removed { id: _ } => {}
            AssetEvent::LoadedWithDependencies { id: _ } => {}
            _ => {}
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Reflect)]
pub enum TileKind {
    Wall,
    Floor,
    Player,
    Pushable,
    Pullable,
    Platform,
    Pit,
}

impl TileKind {
    pub fn is_static(&self) -> bool {
        matches!(self, TileKind::Wall)
    }

    pub fn index_flip(&self, pos: &TilePos, level: &Level) -> (u32, TileFlags) {
        match self {
            TileKind::Floor | TileKind::Player | TileKind::Pushable | TileKind::Pullable => {
                (16, TileFlags::empty())
            }
            TileKind::Wall => calculate_wall_index(**pos, level),
            TileKind::Platform => (17, TileFlags::empty()),
            TileKind::Pit => (18, TileFlags::empty()),
        }
    }

    pub fn entity_kind(&self) -> Option<EntityKind> {
        match self {
            TileKind::Wall => Some(EntityKind::Wall),
            TileKind::Floor => None,
            TileKind::Player => None,
            TileKind::Pushable => Some(EntityKind::Pushable),
            TileKind::Pullable => Some(EntityKind::Pullable),
            TileKind::Platform => Some(EntityKind::Platform),
            TileKind::Pit => Some(EntityKind::Pit),
        }
    }
}

impl From<u8> for TileKind {
    fn from(value: u8) -> Self {
        use TileKind::*;
        match value {
            b'#' => Wall,
            b'_' => Floor,
            b'@' => Player,
            b'b' => Pushable,
            b'p' => Pullable,
            b'-' => Platform,
            b'O' => Pit,
            _ => {
                bevy::log::warn!("Couldnt parse tile kind defaulting to wall tile");
                Wall
            }
        }
    }
}

#[derive(Deserialize, Debug, Reflect, Deref)]
pub struct StringLevel(pub String);
#[derive(Deserialize, Debug, Deref)]
pub struct StringLevels(pub Vec<StringLevel>);

#[derive(TypePath, Debug, Deserialize, Deref, DerefMut, Asset)]
pub struct Levels(pub Vec<Level>);

#[derive(Deserialize, Debug, Reflect)]
pub struct Level {
    pub tiles: Vec<TileKind>,
    pub size: UVec2,
}

#[derive(Default)]
pub struct LevelLoader;

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum LevelLoaderError {
    #[error("Could not read the file: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not parse the ron: {0}")]
    RonError(#[from] ron::error::SpannedError),
}

impl AssetLoader for LevelLoader {
    type Asset = Levels;
    type Settings = ();
    type Error = LevelLoaderError;

    fn load<'a>(
        &'a self,
        reader: &'a mut bevy::asset::io::Reader,
        _settings: &'a Self::Settings,
        _load_context: &'a mut bevy::asset::LoadContext,
    ) -> impl ConditionalSendFuture
           + Future<Output = Result<<Self as AssetLoader>::Asset, <Self as AssetLoader>::Error>>
    {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            let string_levels = ron::de::from_bytes::<StringLevels>(&bytes)?;
            let levels = string_levels
                .iter()
                .map(|string_level| {
                    let height = string_level.split('\n').count() as u32;
                    let tiles = string_level
                        .replace(['\n', ' '], "")
                        .as_bytes()
                        .iter()
                        .map(|byte| TileKind::from(*byte))
                        .collect::<Vec<TileKind>>();
                    let width = tiles.len() as u32 / height;
                    let tiles = tiles
                        .chunks_exact(width as usize)
                        .rev()
                        .flat_map(|chunk| chunk.to_vec())
                        .collect::<Vec<TileKind>>();
                    Level {
                        tiles,
                        size: UVec2::new(width, height),
                    }
                })
                .collect::<Vec<Level>>();

            Ok(Levels(levels))
        })
    }

    fn extensions(&self) -> &[&str] {
        &["levels.ron"]
    }
}
