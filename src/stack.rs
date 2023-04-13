use std::collections::HashMap;

use crate::Widget;

pub struct Stack {
    layers: HashMap<usize, Vec<Box<dyn Widget>>>,
}

impl Stack {
    pub fn new() -> Self {
        todo!("Figure this out. Layers need to be merged BEFORE being handed to termwiz for diffs.")
    }
}
