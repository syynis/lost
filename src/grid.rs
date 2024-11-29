use bevy::{math::*, reflect::Reflect};
use std::ops::{Index, IndexMut};

#[derive(Debug, Clone, Reflect)]
pub struct Grid<T> {
    cells: Vec<T>,
    size: IVec2, // TODO: use u32
}

impl<T> Grid<T> {
    pub fn from_raw(size: IVec2, raw: impl Into<Vec<T>>) -> Self {
        let cells = raw.into();

        Self { cells, size }
    }

    pub fn populate_from(size: IVec2, mut f: impl FnMut(IVec2) -> T) -> Self {
        Self {
            cells: (0..size.y)
                .flat_map(|y| (0..size.x).map(move |x| IVec2::new(x, y)))
                .map(&mut f)
                .collect(),

            size,
        }
    }

    pub fn new(size: IVec2, default_cell: T) -> Self
    where
        T: Clone,
    {
        Self {
            cells: vec![default_cell; (size.x * size.y) as usize],

            size,
        }
    }

    fn idx(&self, pos: IVec2) -> Option<usize> {
        if pos.x < self.size.x && pos.y < self.size.y {
            Some((pos.y * self.size.x + pos.x) as usize)
        } else {
            None
        }
    }

    pub fn size(&self) -> IVec2 {
        self.size
    }

    pub fn get(&self, pos: IVec2) -> Option<&T> {
        self.cells.get(self.idx(pos)?)
    }

    pub fn get_mut(&mut self, pos: IVec2) -> Option<&mut T> {
        let idx = self.idx(pos)?;

        self.cells.get_mut(idx)
    }

    pub fn set(&mut self, pos: IVec2, cell: T) -> Option<T> {
        let idx = self.idx(pos)?;

        self.cells.get_mut(idx).map(|c| core::mem::replace(c, cell))
    }

    pub fn iter(&self) -> impl Iterator<Item = (IVec2, &T)> + '_ {
        let w = self.size.x;

        self.cells
            .iter()
            .enumerate()
            .map(move |(i, cell)| (IVec2::new(i as i32 % w, i as i32 / w), cell))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (IVec2, &mut T)> + '_ {
        let w = self.size.x;

        self.cells
            .iter_mut()
            .enumerate()
            .map(move |(i, cell)| (IVec2::new(i as i32 % w, i as i32 / w), cell))
    }

    pub fn iter_area(&self, pos: IVec2, size: IVec2) -> impl Iterator<Item = (IVec2, &T)> + '_ {
        (0..size.x).flat_map(move |x| {
            (0..size.y).flat_map(move |y| {
                Some((
                    pos + IVec2::new(x, y),
                    &self.cells[self.idx(pos + IVec2::new(x, y))?],
                ))
            })
        })
    }

    pub fn raw(&self) -> &[T] {
        &self.cells
    }
}

impl<T> Index<IVec2> for Grid<T> {
    type Output = T;

    fn index(&self, index: IVec2) -> &Self::Output {
        self.get(index).unwrap_or_else(|| {
            panic!(
                "Attempted to index grid of size {:?} with index {:?}",
                self.size(),
                index
            )
        })
    }
}

impl<T> IndexMut<IVec2> for Grid<T> {
    fn index_mut(&mut self, index: IVec2) -> &mut Self::Output {
        let size = self.size();

        self.get_mut(index).unwrap_or_else(|| {
            panic!(
                "Attempted to index grid of size {:?} with index {:?}",
                size, index
            )
        })
    }
}
