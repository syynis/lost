use bevy::prelude::*;

pub const CARDINALS: [IVec2; 4] = [IVec2::Y, IVec2::X, IVec2::NEG_Y, IVec2::NEG_X];

pub const ORDINALS: [IVec2; 4] = [
    IVec2::ONE,
    IVec2::new(1, -1),
    IVec2::NEG_ONE,
    IVec2::new(-1, 1),
];

pub const DIRS: [IVec2; 8] = [
    IVec2::Y,
    IVec2::ONE,
    IVec2::X,
    IVec2::new(1, -1),
    IVec2::NEG_Y,
    IVec2::NEG_ONE,
    IVec2::NEG_X,
    IVec2::new(-1, 1),
];
