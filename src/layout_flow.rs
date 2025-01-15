use std::{
    cmp,
    ops::{Deref, DerefMut},
};

#[derive(Clone, Debug)]
pub struct LayoutElement<Data> {
    pub offset: f32,
    pub height: f32,
    pub data: Data,
}

// TODO: Rename this thing...
#[derive(Clone, Default, Debug)]
pub struct LayoutFlow<Data> {
    pub(super) flow: Vec<LayoutElement<Data>>,
    height: f32,
}

pub trait LayoutData {
    fn height(&self) -> f32;
}

pub struct MutableData<'a, Data: LayoutData> {
    index: usize,
    layout_flow: &'a mut LayoutFlow<Data>,
}

impl<Data> Deref for MutableData<'_, Data>
where
    Data: LayoutData,
{
    type Target = Data;

    fn deref(&self) -> &Self::Target {
        &self.layout_flow.flow[self.index].data
    }
}

impl<Data> DerefMut for MutableData<'_, Data>
where
    Data: LayoutData,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.layout_flow.flow[self.index].data
    }
}

impl<Data> Drop for MutableData<'_, Data>
where
    Data: LayoutData,
{
    fn drop(&mut self) {
        let new_height = self.layout_flow.flow[self.index].data.height();
        let height_diff = new_height - self.layout_flow.flow[self.index].height;
        if height_diff.abs() > f32::EPSILON {
            self.layout_flow.recompute_from_index(self.index);
        }
    }
}

impl<Data: LayoutData> LayoutFlow<Data> {
    pub fn new() -> Self {
        Self {
            flow: Vec::new(),
            height: 0.0,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            flow: Vec::with_capacity(capacity),
            height: 0.0,
        }
    }

    // TODO: Think about making it a `Result`
    pub fn get_visible_parts(
        &self,
        offset: f32,
        height: f32,
    ) -> &[LayoutElement<Data>] {
        let bottom = offset + height;
        if let Ok(index) = self.flow.binary_search_by(|v| {
            // TODO: This comparison should probably use epsilon
            if v.offset <= offset && v.offset + v.height >= offset {
                cmp::Ordering::Equal
            } else if v.offset < offset {
                cmp::Ordering::Less
            } else {
                cmp::Ordering::Greater
            }
        }) {
            let last_index = self.flow[index..]
                .iter()
                .position(|v| v.offset <= bottom && v.offset + v.height >= bottom)
                .map(|index| index + self.flow[index..].len())
                // TODO: Maybe this should return an error???
                .unwrap_or(self.flow.len());
            &self.flow[index..last_index]
        } else {
            &[]
        }
    }

    pub fn push(&mut self, element: Data) {
        let offset = self.flow.last().map(|v| v.offset + v.height).unwrap_or(0.0);
        let elem = LayoutElement {
            offset,
            height: element.height(),
            data: element,
        };
        self.height += elem.height;
        self.flow.push(elem);
    }

    pub fn insert(&mut self, index: usize, element: Data) {
        let mut offset = self.flow[index].offset;
        let elem = LayoutElement {
            offset,
            height: element.height(),
            data: element,
        };
        offset += elem.height;
        self.height += elem.height;
        self.flow.insert(index, elem);
        for e in self.flow[index + 1..].iter_mut() {
            e.offset = offset;
            offset += e.height;
        }
    }

    pub fn get_mutable(&mut self, index: usize) -> MutableData<'_, Data> {
        MutableData {
            index,
            layout_flow: self,
        }
    }

    /// This return an element with correlated coordinates within the element
    pub fn get_element_at_offset(&self, offset: f32) -> Option<(&Data, f32)> {
        let res = self
            .flow
            .binary_search_by(|v| {
                // TODO: This comparison should probably use epsilon
                if v.offset <= offset && v.offset + v.height >= offset {
                    cmp::Ordering::Equal
                } else if v.offset < offset {
                    cmp::Ordering::Less
                } else {
                    cmp::Ordering::Greater
                }
            })
            .ok();
        res.map(|index| {
            let element = &self.flow[index];
            let corelated_offset = offset - element.offset;
            (&element.data, corelated_offset)
        })
    }

    pub fn recopute_all(&mut self) {
        self.recompute_from_index(0);
    }

    /// This return an element with correlated coordinates within the element
    pub fn recompute_from_index(&mut self, index: usize) {
        let mut offset = self.flow[index].offset;
        for element in self.flow[index..].iter_mut() {
            element.height = element.data.height();
            element.offset = offset;
            offset += element.height;
        }
        self.height = offset;
    }

    pub fn apply_to_all<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut Data),
    {
        for e in self.flow.iter_mut() {
            f(&mut e.data)
        }
        self.recopute_all();
    }
}