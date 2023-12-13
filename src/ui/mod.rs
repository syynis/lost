use bevy::{ecs::system::Command, prelude::*};
use bevy_nine_slice_ui::{NineSliceUiMaterialBundle, NineSliceUiTexture};

#[derive(Component)]
pub struct NineSliceButtonText<T: Component + Into<String> + Clone> {
    pub button: T,
    pub style: Style,
    pub texture: Handle<Image>,
    pub parent: Entity,
}

impl<T: Component + Into<String> + Clone> Command for NineSliceButtonText<T> {
    fn apply(self, world: &mut World) {
        world.entity_mut(self.parent).with_children(|parent| {
            parent
                .spawn((
                    NineSliceUiMaterialBundle {
                        style: self.style,
                        nine_slice_texture: NineSliceUiTexture::from_image(
                            self.texture.clone_weak(),
                        ),
                        focus_policy: bevy::ui::FocusPolicy::Block,
                        ..default()
                    },
                    Interaction::default(),
                    self.button.clone(),
                ))
                .with_children(|parent| {
                    parent.spawn(TextBundle::from_section(
                        self.button.into(),
                        TextStyle {
                            font_size: 20.,
                            color: Color::WHITE,
                            ..default()
                        },
                    ));
                });
        });
    }
}
