use bevy::{ecs::system::Command, prelude::*};
use bevy_nine_slice_ui::NineSliceTexture;

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
                    NodeBundle {
                        style: self.style,
                        focus_policy: bevy::ui::FocusPolicy::Block,
                        ..default()
                    },
                    Interaction::default(),
                    self.button.clone(),
                    NineSliceTexture::new(self.texture.clone_weak()),
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
