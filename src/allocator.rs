use std::{collections::VecDeque, fmt::Debug, mem::MaybeUninit, ptr::NonNull};

#[derive(Debug, PartialEq, Eq, PartialOrd, Clone, Copy, Hash)]
pub struct NodeId {
    idx: u32,
    ver: u32,
}

impl NodeId {
    fn from_raw(idx: u32, ver: u32) -> Self {
        NodeId { idx, ver }
    }

    fn into_raw(self) -> (u32, u32) {
        (self.idx, self.ver)
    }
}

struct Slot<T> {
    value: T,
    version: u32,
}

impl<T> Default for Slot<T> {
    fn default() -> Self {
        Slot {
            value: unsafe { MaybeUninit::zeroed().assume_init() },
            version: 1,
        }
    }
}

pub struct Slab<V> {
    ptr: NonNull<Slot<V>>,
    cap: usize,
    taken: usize,
    free: VecDeque<u32>,
}

#[allow(unused)]
impl<V> Slab<V> {
    pub fn new() -> Self {
        Slab {
            // Note: If capacity is zero, ptr will always be invalid
            ptr: NonNull::dangling(),
            cap: 0,
            taken: 0,
            free: VecDeque::new(),
        }
    }

    pub fn with_capacity(cap: usize) -> Self {
        assert!(cap.is_power_of_two());
        let mut s = Slab {
            ptr: NonNull::dangling(),
            cap: 0,
            taken: 0,
            free: VecDeque::new(),
        };
        let layout =
            std::alloc::Layout::array::<Slot<V>>(cap).expect("layout to have a non-zero size");
        let ptr = unsafe { std::alloc::alloc(layout) } as *mut Slot<V>;

        s.ptr = NonNull::new(ptr).expect("new allocation should not be nullptr");

        for i in 0..cap {
            unsafe {
                ptr.add(i).write(Slot::<V>::default());
            }
            s.free.push_back(i as u32);
        }
        s.cap = cap;
        s
    }

    pub fn len(&self) -> usize {
        self.taken
    }

    pub fn capacity(&self) -> usize {
        self.cap
    }

    pub fn insert(&mut self, value: V) -> NodeId {
        use std::alloc;
        let offset = loop {
            if self.cap == 0 {
                let layout =
                    alloc::Layout::array::<Slot<V>>(4).expect("layout to have a non-zero size");
                // Safety: this is safe because we know Slot<V> has a non-zero size
                let ptr = unsafe { alloc::alloc(layout) } as *mut Slot<V>;

                self.ptr = NonNull::new(ptr).expect("new allocation should not be nullptr");

                for i in 0..4 {
                    // Safety: we know self.ptr is non-null and initialized `alloc`
                    unsafe {
                        ptr.add(i).write(Slot::<V>::default());
                    }
                    self.free.push_back(i as u32);
                }
                self.cap = 4;
            } else if self.taken >= self.cap {
                let layout = alloc::Layout::array::<Slot<V>>(self.cap)
                    .expect("layout to have a non-zero size");

                // This is safe because we know Slot<V> has a size > 0, and we check for a nullptr
                // result
                let ptr = unsafe {
                    alloc::realloc(
                        self.ptr.as_ptr() as *mut u8,
                        layout,
                        std::mem::size_of::<Slot<V>>() * self.cap * 2,
                    ) as *mut Slot<V>
                };

                self.ptr = NonNull::new(ptr).expect("reallocation should not be nullptr");

                for i in self.cap..(self.cap * 2) {
                    // Safety: we have confirmed that self.ptr is non null, and that it is freshly
                    // reallocated.
                    unsafe {
                        self.ptr.as_ptr().add(i).write(Slot::default());
                    }
                    self.free.push_back(i as u32);
                }
                self.cap *= 2;
            }
            if let Some(free) = self.free.pop_front() {
                break free;
            }
        };

        // Safety: This is safe because we know all slots are pre-initialized at allocation time
        let slot = unsafe {
            self.ptr
                .as_ptr()
                .add(offset as usize)
                .as_mut()
                .expect("slot should not be nullptr")
        };

        // Just in case, assert that the slot is marked as free. If this panics, it's a bug.
        assert!(
            slot.version % 2 != 0,
            "Slot from free queue marked as occupied",
        );

        // perform the store
        slot.value = value;
        // mark the slot as taken
        slot.version += 1;
        self.taken += 1;

        NodeId::from_raw(offset, slot.version)
    }

    pub fn get(&self, key: NodeId) -> Option<&V> {
        let (idx, ver) = NodeId::into_raw(key);
        if idx as usize > self.cap {
            return None;
        }

        // Safety: This is safe because we know all slots are pre-initialized at allocation time
        let slot = unsafe { self.ptr.as_ptr().add(idx as usize).as_ref() };
        if let Some(slot) = slot {
            if slot.version % 2 != 0 || slot.version != ver {
                return None;
            }
            Some(&slot.value)
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, key: NodeId) -> Option<&mut V> {
        let (idx, ver) = NodeId::into_raw(key);
        if idx as usize > self.cap {
            return None;
        }

        // Safety: This is safe because we know all slots are pre-initialized at allocation time
        let slot = unsafe { self.ptr.as_ptr().add(idx as usize).as_mut() };
        if let Some(slot) = slot {
            if slot.version % 2 != 0 || slot.version != ver {
                return None;
            }
            Some(&mut slot.value)
        } else {
            None
        }
    }

    pub fn remove(&mut self, key: NodeId) -> Option<V> {
        let (idx, ver) = NodeId::into_raw(key);
        if idx as usize > self.cap {
            // index out out bounds, early return
            return None;
        }

        // Safety: This is safe because we know all slots are pre-initialized at allocation time
        let slot = unsafe { self.ptr.as_ptr().add(idx as usize) };
        let current = unsafe { slot.read() };

        if current.version % 2 != 0 || current.version != ver {
            // Key version mismatch - key is invalid
            return None;
        }

        unsafe {
            // Mark the slot as free
            (*slot).version += 1;
        }
        // Add the slot to the free queue
        self.free.push_back(idx);
        self.taken -= 1;

        Some(current.value)
    }

    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(NodeId, &mut V) -> bool,
    {
        for i in 0..self.cap {
            // Safety: This is safe because we know all slots are pre-initialized at allocation time
            let slot = unsafe { self.ptr.as_ptr().add(i).as_mut() };
            if let Some(slot) = slot {
                if slot.version % 2 != 0 {
                    // Slot is not occupied, so we don't need to drop the value
                    continue;
                }
                let key = NodeId::from_raw(i as u32, slot.version);
                if !f(key, &mut slot.value) {
                    // drop the slot's value
                    unsafe {
                        std::ptr::drop_in_place(&mut slot.value);
                    }
                    // mark the slot as free
                    slot.version += 1;
                    // add the slot to the free queue
                    self.free.push_back(i as u32);
                    self.taken -= 1;
                }
            }
        }
    }
}

impl<V> Drop for Slab<V> {
    fn drop(&mut self) {
        let layout = std::alloc::Layout::array::<Slot<V>>(self.cap).unwrap();

        unsafe {
            // drop inner contents
            for i in 0..self.cap {
                let slot = self.ptr.as_ptr().add(i).as_mut().unwrap();
                if slot.version % 2 != 0 {
                    // Slot is not occupied, so we don't need to drop the value
                    continue;
                }
                // drop the slot's value
                std::ptr::drop_in_place(&mut slot.value);
            }
            // drop the allocation itself
            std::alloc::dealloc(self.ptr.as_ptr() as *mut u8, layout);
        }
    }
}

impl<V: Debug> Debug for Slab<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut entries = vec![];
        for i in 0..self.cap {
            let slot = unsafe { self.ptr.as_ptr().add(i).as_ref() };
            if let Some(slot) = slot {
                if slot.version % 2 == 0 {
                    entries.push((i, &slot.value));
                }
            }
        }
        f.debug_map().entries(entries).finish()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn basic() {
        let mut slab = super::Slab::<u32>::new();

        let key1 = slab.insert(1);
        let key2 = slab.insert(2);
        slab.insert(3);
        slab.insert(4);
        slab.insert(5);
        let key3 = slab.insert(6);
        slab.remove(key3);

        assert_eq!(slab.get(key3), None);

        assert_eq!(slab.capacity(), 8);
        assert_eq!(slab.len(), 5);

        assert_eq!(slab.get(key1), Some(&1));
        assert_eq!(slab.get_mut(key2), Some(&mut 2));
    }
}
