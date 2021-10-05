#[derive(Debug, Clone)]
pub struct Subtree<T> {
    node: T,
    next: Option<Box<Subtree<T>>>,
}

impl<T> Subtree<T> {
    pub fn new(node: T, next: Option<Box<Subtree<T>>>) -> Self {
        Self { node, next }
    }

    pub fn next(&self) -> &Option<Box<Subtree<T>>> {
        &self.next
    }

    pub fn next_mut(&mut self) -> &mut Option<Box<Subtree<T>>> {
        &mut self.next
    }

    pub fn take_next(&mut self) -> Option<Box<Subtree<T>>> {
        self.next_mut().take()
    }

    pub fn node(&self) -> &T {
        &self.node
    }

    pub fn node_mut(&mut self) -> &mut T {
        &mut self.node
    }

    pub fn next_node(&self) -> Option<&T> {
        self.next().as_ref().map(|next| next.node())
    }
}
