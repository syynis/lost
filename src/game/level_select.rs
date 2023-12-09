use bevy::prelude::*;

use crate::{
    cleanup::DependOnState,
    game::{level::Levels, GameAssets, GameState},
    ui::NineSliceButtonText,
};

pub struct LevelSelectPlugin;

impl Plugin for LevelSelectPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CurrentLevel>()
            .register_type::<CurrentLevel>()
            .add_systems(OnEnter(GameState::LevelSelect), spawn_level_select)
            .add_systems(
                Update,
                handle_buttons.run_if(in_state(GameState::LevelSelect)),
            );
    }
}

#[derive(Resource, Deref, DerefMut, Reflect, Default, Debug)]
#[reflect(Resource)]
pub struct CurrentLevel(pub usize);

#[derive(Component, Deref, DerefMut, Clone)]
struct LevelButton(pub usize);

impl From<LevelButton> for String {
    fn from(value: LevelButton) -> Self {
        format!("{}", *value + 1)
    }
}

fn handle_buttons(
    mut game_state: ResMut<NextState<GameState>>,
    buttons: Query<(&LevelButton, &Interaction), Changed<Interaction>>,
    mut current_level: ResMut<CurrentLevel>,
) {
    buttons
        .iter()
        .for_each(|(level, interaction)| match interaction {
            Interaction::Pressed => {
                current_level.0 = **level;
                game_state.set(GameState::LevelTransition);
            }
            Interaction::Hovered => {}
            Interaction::None => {}
        });
}

fn spawn_level_select(mut cmds: Commands, assets: Res<GameAssets>, levels: Res<Assets<Levels>>) {
    let button_texture = assets.button.clone_weak();
    let button_style = Style {
        width: Val::Px(75.0),
        height: Val::Px(75.0),
        margin: UiRect::all(Val::Px(10.)),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        border: UiRect::all(Val::Px(2.)),
        ..default()
    };
    let amount_levels = levels
        .get(&assets.levels)
        .expect("Level assets should be loaded")
        .len();
    let cols = 5;
    let rows = (amount_levels / cols) + 1;

    let mut row_nodes = Vec::new();
    for r in 0..rows {
        let row_node = cmds
            .spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    align_content: AlignContent::Center,
                    margin: UiRect::all(Val::Auto),
                    ..default()
                },
                ..default()
            })
            .id();
        for c in 0..cols {
            let idx = c + r * cols;
            if idx >= amount_levels {
                break;
            }
            cmds.add(NineSliceButtonText {
                button: LevelButton(idx),
                style: button_style.clone(),
                texture: button_texture.clone_weak(),
                parent: row_node,
            });
        }
        row_nodes.push(row_node);
    }
    cmds.spawn((
        NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                align_content: AlignContent::Center,
                margin: UiRect::all(Val::Auto),
                ..default()
            },
            ..default()
        },
        DependOnState::single(GameState::LevelSelect),
    ))
    .push_children(&row_nodes);
}
