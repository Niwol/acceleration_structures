use std::collections::HashMap;

use crate::rect::Rect;

pub struct Quadtree<T> {
    max_node_capacity: usize,
    root: Node,
    elements: HashMap<u64, (T, Rect)>,
    next_id: u64,
}

pub struct NodeIter<'a> {
    nodes_to_process: Vec<&'a Node>,
}

pub struct Node {
    region: Rect,
    elements: HashMap<u64, Rect>,
    children: Option<Box<[Node; 4]>>,
    depth: u32,
    size: usize,
}

pub struct Entry<'a, T> {
    id: u64,
    owner: &'a Quadtree<T>,
}

pub struct EntryMut<'a, T> {
    id: u64,
    owner: &'a mut Quadtree<T>,
}

impl<'a, T> Entry<'a, T> {
    pub fn value(&self) -> &T {
        &self.owner.elements[&self.id].0
    }

    pub fn id(&self) -> u64 {
        self.id
    }
}

impl<'a, T> EntryMut<'a, T> {
    pub fn value(&self) -> &T {
        &self.owner.elements[&self.id].0
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn move_entry(&mut self, new_region: Rect) {
        self.owner
            .move_element(self.id, self.owner.elements[&self.id].1, new_region);
    }
}

impl Node {
    pub fn is_leaf(&self) -> bool {
        self.children.is_none()
    }

    pub fn is_node(&self) -> bool {
        self.children.is_some()
    }

    pub fn region(&self) -> Rect {
        self.region
    }

    pub fn elements(&self) -> &HashMap<u64, Rect> {
        &self.elements
    }

    pub fn depth(&self) -> u32 {
        self.depth
    }

    pub fn size(&self) -> usize {
        self.size
    }

    fn new(region: Rect) -> Self {
        Self {
            region,
            elements: HashMap::new(),
            children: None,
            depth: 0,
            size: 0,
        }
    }

    fn insert(&mut self, id: u64, region: Rect, max_node_capacity: usize) {
        assert!(self.region.contains(&region));

        if self.is_leaf() && self.elements.len() < max_node_capacity {
            self.elements.insert(id, region);
            self.size += 1;
            return;
        }

        if self.is_leaf() && self.elements.len() == max_node_capacity {
            self.subdivide(max_node_capacity);
        }
        self.size += 1;

        for child in self.children.as_mut().unwrap().iter_mut() {
            if child.region.contains(&region) {
                child.insert(id, region, max_node_capacity);
                return;
            }
        }

        self.elements.insert(id, region);
    }

    fn subdivide(&mut self, max_node_capacity: usize) {
        let mut new_self = Node::new(self.region);

        let children_w = self.region.w / 2.0;
        let children_h = self.region.h / 2.0;

        #[rustfmt::skip]
        let mut children = [
            // Top left
            Node::new(Rect::new(self.region.x, self.region.y, children_w, children_h)),

            // Top right
            Node::new(Rect::new(self.region.x + children_w, self.region.y, children_w, children_h)),
            
            // Bottom left
            Node::new(Rect::new(self.region.x, self.region.y + children_h, children_w, children_h)),
            
            // Bottom right
            Node::new(Rect::new(self.region.x + children_w, self.region.y + children_h, children_w, children_h)),
        ];

        for child in children.iter_mut() {
            child.depth = self.depth + 1;
        }

        new_self.children = Some(Box::new(children));

        for (id, region) in self.elements.iter() {
            new_self.insert(*id, *region, max_node_capacity);
        }

        *self = new_self;
    }

    fn get_all(&self) -> Vec<u64> {
        let mut result = Vec::new();

        for (id, _) in self.elements.iter() {
            result.push(*id);
        }

        if let Some(children) = &self.children {
            for child in children.as_ref() {
                result.extend(child.get_all());
            }
        }

        result
    }

    fn get_contained(&self, region: Rect) -> Vec<u64> {
        let mut result = Vec::new();

        for (id, element_region) in self.elements.iter() {
            if region.contains(element_region) {
                result.push(*id);
            }
        }

        if let Some(children) = &self.children {
            for child in children.as_ref() {
                if region.contains(&child.region) {
                    result.extend(child.get_all());
                } else if region.overlapps(&child.region) {
                    result.extend(child.get_contained(region));
                }
            }
        }

        result
    }

    fn get_overlapped(&self, region: Rect) -> Vec<u64> {
        let mut result = Vec::new();

        for (id, element_region) in self.elements.iter() {
            if region.overlapps(&element_region) {
                result.push(*id);
            }
        }

        if let Some(children) = &self.children {
            for child in children.as_ref() {
                if region.contains(&child.region) {
                    result.extend(child.get_all());
                } else if region.overlapps(&child.region) {
                    result.extend(child.get_overlapped(region));
                }
            }
        }

        result
    }

    fn remove(&mut self, id: u64, region: Rect, max_node_capacity: usize) {
        self.size -= 1;

        if let Some(children) = &mut self.children {
            for child in children.as_mut() {
                if child.region.contains(&region) {
                    child.remove(id, region, max_node_capacity);
                    break;
                }
            }
        }

        self.elements.remove(&id);

        if self.size == max_node_capacity {
            self.fuse();
        }
    }

    fn fuse(&mut self) {
        debug_assert!(!self.is_leaf());
        let mut children_elements = HashMap::new();

        let children = self.children.take();

        for child in children.unwrap().into_iter() {
            debug_assert!(child.is_leaf());

            children_elements.extend(child.elements);
        }

        self.elements.extend(children_elements);
    }

    fn move_element(
        &mut self,
        id: u64,
        old_region: Rect,
        new_region: Rect,
        max_node_capacity: usize,
    ) {
        if let Some(children) = &mut self.children {
            for child in children.as_mut() {
                if child.region.contains(&old_region) && child.region.contains(&new_region) {
                    child.move_element(id, old_region, new_region, max_node_capacity);
                    return;
                }

                if child.region.contains(&old_region) {
                    child.remove(id, old_region, max_node_capacity);
                    self.size -= 1;
                    self.insert(id, new_region, max_node_capacity);
                    return;
                }
            }
        }

        self.elements.remove(&id);
        self.size -= 1;
        self.insert(id, new_region, max_node_capacity);
    }
}

impl<T> Quadtree<T> {
    pub fn new(region: Rect, max_node_capacity: usize) -> Self {
        let root = Node::new(region);
        Self {
            max_node_capacity,
            root,
            elements: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        true
    }

    pub fn size(&self) -> usize {
        self.elements.len()
    }

    pub fn insert(&mut self, element: T, region: Rect) -> u64 {
        let id = self.next_id;
        self.elements.insert(id, (element, region));

        self.root.insert(id, region, self.max_node_capacity);

        self.next_id += 1;

        id
    }

    pub fn get_contained(&self, region: Rect) -> Vec<&T> {
        let ids = self.root.get_contained(region);
        ids.into_iter().map(|id| &self.elements[&id].0).collect()
    }

    pub fn get_contained_mut(&mut self, region: Rect) -> Vec<&mut T> {
        let ids = self.root.get_contained(region);
        let mut result = Vec::new();
        unsafe {
            for id in ids {
                let map_ptr = &mut self.elements as *mut HashMap<u64, (T, Rect)>;
                result.push(&mut map_ptr.as_mut().unwrap().get_mut(&id).unwrap().0);
            }
        }

        result
    }

    pub fn get_overlapped(&self, region: Rect) -> Vec<&T> {
        let ids = self.root.get_overlapped(region);
        ids.into_iter().map(|id| &self.elements[&id].0).collect()
    }

    pub fn get_overlapped_mut(&mut self, region: Rect) -> Vec<&mut T> {
        let ids = self.root.get_overlapped(region);
        let mut result = Vec::new();
        unsafe {
            for id in ids {
                let map_ptr = &mut self.elements as *mut HashMap<u64, (T, Rect)>;
                result.push(&mut map_ptr.as_mut().unwrap().get_mut(&id).unwrap().0);
            }
        }

        result
    }

    pub fn entry<'a>(&'a mut self, id: u64) -> Entry<'a, T> {
        debug_assert!(self.elements.get(&id).is_some());

        Entry { id, owner: self }
    }

    pub fn entry_mut<'a>(&'a mut self, id: u64) -> EntryMut<'a, T> {
        debug_assert!(self.elements.get(&id).is_some());

        EntryMut { id, owner: self }
    }

    pub fn remove(&mut self, id: u64) -> Option<(T, Rect)> {
        let element = self.elements.remove(&id);

        if let Some((element, region)) = element {
            self.root.remove(id, region, self.max_node_capacity);
            Some((element, region))
        } else {
            None
        }
    }

    pub fn entries<'a>(&'a self) -> impl Iterator<Item = Entry<'a, T>> {
        let iter = self.elements.keys().map(|id| Entry {
            id: *id,
            owner: self,
        });

        iter
    }

    pub fn entries_mut<'a>(&'a mut self) -> impl Iterator<Item = EntryMut<'a, T>> {
        unsafe {
            let self_ptr = self as *mut Self;
            self.elements.keys().map(move |id| EntryMut {
                id: *id,
                owner: &mut *self_ptr,
            })
        }
    }

    pub fn nodes<'a>(&'a self) -> NodeIter<'a> {
        NodeIter {
            nodes_to_process: vec![&self.root],
        }
    }

    fn move_element(&mut self, id: u64, old_region: Rect, new_region: Rect) {
        self.root
            .move_element(id, old_region, new_region, self.max_node_capacity);
    }
}

impl<T> Quadtree<T>
where
    T: PartialEq,
{
    pub fn contains(&self, element: &T) -> bool {
        self.elements.values().any(|(e, _)| e == element)
    }
}

impl<T> Default for Quadtree<T> {
    fn default() -> Self {
        Self {
            max_node_capacity: 5,
            root: Node::new(Rect::new(-100.0, -100.0, 200.0, 200.0)),
            elements: HashMap::new(),
            next_id: 0,
        }
    }
}

impl<'a> Iterator for NodeIter<'a> {
    type Item = &'a Node;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(node) = self.nodes_to_process.pop() {
            if let Some(children) = &node.children {
                for child in children.as_ref() {
                    self.nodes_to_process.push(child);
                }
            }

            return Some(node);
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use crate::rect::Rect;

    use super::*;

    #[test]
    fn create_empty() {
        let quadtree: Quadtree<i32> = Quadtree::default();

        assert!(quadtree.is_empty());
        assert_eq!(quadtree.size(), 0);
        assert!(quadtree.root.is_leaf());
    }

    // Insertion
    #[test]
    fn insert_one_element() {
        let mut quadtree = Quadtree::default();
        quadtree.insert(42, Rect::new(10.0, 10.0, 10.0, 10.0));

        assert!(quadtree.contains(&42));
        assert_eq!(quadtree.size(), 1);
    }

    #[test]
    fn insert_two_elements() {
        let mut quadtree = Quadtree::default();
        quadtree.insert(42, Rect::new(10.0, 10.0, 10.0, 10.0));
        quadtree.insert(5, Rect::new(50.0, 20.0, 40.0, 10.0));

        assert!(quadtree.contains(&42));
        assert!(quadtree.contains(&5));
        assert_eq!(quadtree.size(), 2);
    }

    #[test]
    fn not_contains_not_inserted_element() {
        let quadtree = Quadtree::default();
        assert!(!quadtree.contains(&666));
    }

    // Element access
    #[test]
    fn get_no_element_in_empty() {
        let quadtree: Quadtree<i32> = Quadtree::default();
        assert_eq!(
            quadtree.get_contained(Rect::new(10.0, 10.0, 10.0, 10.0)),
            Vec::<&i32>::new()
        )
    }

    // Element access contained
    #[test]
    fn get_contained_after_one_insertion() {
        let mut quadtree: Quadtree<i32> = Quadtree::default();
        quadtree.insert(42, Rect::new(10.0, 10.0, 10.0, 10.0));

        assert_eq!(
            quadtree.get_contained(Rect::new(10.0, 10.0, 10.0, 10.0)),
            vec![&42]
        )
    }

    #[test]
    fn get_two_contained_element_after_two_insertions() {
        let mut quadtree: Quadtree<i32> = Quadtree::default();
        quadtree.insert(42, Rect::new(10.0, 10.0, 10.0, 10.0));
        quadtree.insert(5, Rect::new(11.0, 14.0, 5.0, 2.0));

        let elements = quadtree.get_contained(Rect::new(10.0, 10.0, 10.0, 10.0));

        assert!(elements.contains(&&42));
        assert!(elements.contains(&&5));
    }

    #[test]
    fn get_only_one_contained_element_after_two_insertions() {
        let mut quadtree: Quadtree<i32> = Quadtree::default();
        quadtree.insert(42, Rect::new(10.0, 10.0, 10.0, 10.0));
        quadtree.insert(5, Rect::new(15.0, 10.0, 10.0, 10.0));

        assert_eq!(
            quadtree.get_contained(Rect::new(10.0, 10.0, 10.0, 10.0)),
            vec![&42]
        )
    }

    // Element access overlapped
    #[test]
    fn get_overlapped_after_one_insertion() {
        let mut quadtree: Quadtree<i32> = Quadtree::default();
        quadtree.insert(42, Rect::new(10.0, 10.0, 10.0, 10.0));

        assert_eq!(
            quadtree.get_overlapped(Rect::new(15.0, 10.0, 10.0, 10.0)),
            vec![&42]
        )
    }

    #[test]
    fn get_two_overlapped_element_after_two_insertions() {
        let mut quadtree: Quadtree<i32> = Quadtree::default();
        quadtree.insert(42, Rect::new(10.0, 10.0, 10.0, 10.0));
        quadtree.insert(5, Rect::new(15.0, 14.0, 15.0, 2.0));

        let elements = quadtree.get_overlapped(Rect::new(10.0, 10.0, 10.0, 10.0));

        assert!(elements.contains(&&42));
        assert!(elements.contains(&&5));
    }

    #[test]
    fn get_only_one_overlapped_element_after_two_insertions() {
        let mut quadtree: Quadtree<i32> = Quadtree::default();
        quadtree.insert(42, Rect::new(10.0, 10.0, 10.0, 10.0));
        quadtree.insert(5, Rect::new(35.0, 10.0, 10.0, 10.0));

        assert_eq!(
            quadtree.get_overlapped(Rect::new(10.0, 10.0, 10.0, 10.0)),
            vec![&42]
        )
    }

    // Removing
    #[test]
    fn remove_one_element() {
        let mut quadtree: Quadtree<i32> = Quadtree::default();
        let value = 42;
        let region = Rect::new(10.0, 10.0, 10.0, 10.0);
        let id = quadtree.insert(value, region);

        assert_eq!(quadtree.remove(id).unwrap(), (value, region));
    }

    // Entries
    #[test]
    fn entry() {
        let mut quadtree = Quadtree::default();
        let entry_id = quadtree.insert(42, Rect::new(10.0, 10.0, 10.0, 10.0));

        let entry = quadtree.entry(entry_id);

        assert_eq!(entry.value(), &42);
        assert_eq!(entry.id(), entry_id);
    }

    #[test]
    fn move_entry() {
        let mut quadtree = Quadtree::default();
        let entry_id = quadtree.insert(42, Rect::new(10.0, 10.0, 10.0, 10.0));

        let mut entry = quadtree.entry_mut(entry_id);
        entry.move_entry(Rect::new(20.0, 20.0, 5.0, 5.0));

        assert_eq!(
            quadtree.get_contained(Rect::new(20.0, 20.0, 5.0, 5.0)),
            vec![&42]
        );
    }
}

#[cfg(test)]
mod node_tests {
    use super::*;

    #[test]
    fn create_empty() {
        let node = Node::new(Rect::new(0.0, 0.0, 50.0, 50.0));

        assert!(node.is_leaf());
        assert_eq!(node.size, 0);
        assert!(node.elements.is_empty());
    }

    // Adding elements
    #[test]
    fn add_one_element() {
        let mut node = Node::new(Rect::new(0.0, 0.0, 50.0, 50.0));
        let id = 0;
        let region = Rect::new(10.0, 10.0, 10.0, 10.0);
        node.insert(id, region, 5);

        assert!(node.is_leaf());
        assert!(!node.elements.is_empty());
        assert_eq!(node.size, 1);
        assert_eq!(node.elements.iter().next().unwrap(), (&id, &region));
    }

    #[test]
    #[should_panic]
    fn add_one_element_outside_node_region() {
        let mut node = Node::new(Rect::new(0.0, 0.0, 50.0, 50.0));
        node.insert(0, Rect::new(-10.0, -10.0, 10.0, 10.0), 5);
    }

    #[test]
    fn add_elements_until_subdivision() {
        let mut node = Node::new(Rect::new(0.0, 0.0, 50.0, 50.0));
        let max_node_capacity = 3;
        node.insert(0, Rect::new(10.0, 10.0, 10.0, 10.0), max_node_capacity);
        node.insert(1, Rect::new(20.0, 20.0, 10.0, 10.0), max_node_capacity);
        node.insert(2, Rect::new(30.0, 10.0, 10.0, 20.0), max_node_capacity);

        assert!(node.is_leaf());

        node.insert(3, Rect::new(10.0, 15.0, 20.0, 20.0), max_node_capacity);

        assert!(!node.is_leaf());
        assert!(node.elements.contains_key(&1));
        assert!(node.elements.contains_key(&2));
        assert!(node.elements.contains_key(&3));

        assert!(!node.elements.contains_key(&0));
        assert!(node.children.unwrap()[0].elements.contains_key(&0));

        assert_eq!(node.size, 4);
    }

    // Removing elements
    #[test]
    fn remove_one_element() {
        let mut node = Node::new(Rect::new(0.0, 0.0, 50.0, 50.0));
        let id = 0;
        let region = Rect::new(10.0, 10.0, 10.0, 10.0);
        node.insert(id, region, 5);

        node.remove(id, region, 5);

        assert_eq!(node.size, 0);
        assert!(node.elements.is_empty());
    }

    #[test]
    fn after_subdivision_remove_child_element_to_fuse() {
        let mut node = Node::new(Rect::new(0.0, 0.0, 50.0, 50.0));
        let max_node_capacity = 3;
        node.insert(0, Rect::new(10.0, 10.0, 10.0, 10.0), max_node_capacity);
        node.insert(1, Rect::new(20.0, 20.0, 10.0, 10.0), max_node_capacity);
        node.insert(2, Rect::new(30.0, 10.0, 10.0, 20.0), max_node_capacity);
        node.insert(3, Rect::new(10.0, 15.0, 20.0, 20.0), max_node_capacity);

        node.remove(0, Rect::new(10.0, 10.0, 10.0, 10.0), max_node_capacity);

        assert_eq!(node.size, 3);
        assert!(node.is_leaf());
    }

    // Moving elements
    #[test]
    fn moving_element_to_parent_node() {
        let mut node = Node::new(Rect::new(0.0, 0.0, 50.0, 50.0));
        let max_node_capacity = 3;
        node.insert(0, Rect::new(10.0, 10.0, 10.0, 10.0), max_node_capacity);
        node.insert(1, Rect::new(20.0, 20.0, 10.0, 10.0), max_node_capacity);
        node.insert(2, Rect::new(30.0, 10.0, 10.0, 20.0), max_node_capacity);
        node.insert(3, Rect::new(10.0, 15.0, 20.0, 20.0), max_node_capacity);

        node.move_element(
            0,
            Rect::new(10.0, 10.0, 10.0, 10.0),
            Rect::new(10.0, 20.0, 10.0, 10.0),
            max_node_capacity,
        );

        assert!(node.elements.contains_key(&0));
        assert!(node.children.unwrap()[0].elements.is_empty());

        assert_eq!(node.size, 4);
    }

    #[test]
    fn moving_element_to_other_child() {
        let mut node = Node::new(Rect::new(0.0, 0.0, 50.0, 50.0));
        let max_node_capacity = 3;
        node.insert(0, Rect::new(10.0, 10.0, 10.0, 10.0), max_node_capacity);
        node.insert(1, Rect::new(20.0, 20.0, 10.0, 10.0), max_node_capacity);
        node.insert(2, Rect::new(30.0, 10.0, 10.0, 20.0), max_node_capacity);
        node.insert(3, Rect::new(10.0, 15.0, 20.0, 20.0), max_node_capacity);

        node.move_element(
            0,
            Rect::new(10.0, 10.0, 10.0, 10.0),
            Rect::new(10.0, 30.0, 10.0, 10.0),
            max_node_capacity,
        );

        assert!(!node.elements.contains_key(&0));
        assert!(node.children.unwrap()[2].elements.contains_key(&0));

        assert_eq!(node.size, 4);
    }

    #[test]
    fn moving_element_to_child() {
        let mut node = Node::new(Rect::new(0.0, 0.0, 50.0, 50.0));
        let max_node_capacity = 3;
        node.insert(0, Rect::new(10.0, 10.0, 10.0, 10.0), max_node_capacity);
        node.insert(1, Rect::new(20.0, 20.0, 10.0, 10.0), max_node_capacity);
        node.insert(2, Rect::new(30.0, 10.0, 10.0, 20.0), max_node_capacity);
        node.insert(3, Rect::new(10.0, 15.0, 20.0, 20.0), max_node_capacity);

        node.move_element(
            1,
            Rect::new(20.0, 20.0, 10.0, 10.0),
            Rect::new(10.0, 30.0, 10.0, 10.0),
            max_node_capacity,
        );

        assert!(!node.elements.contains_key(&1));
        assert!(node.children.unwrap()[2].elements.contains_key(&1));

        assert_eq!(node.size, 4);
    }
}
