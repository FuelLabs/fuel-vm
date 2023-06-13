use alloc::boxed::Box;

#[derive(Debug, Clone)]
pub struct Subtree<T> {
    node: T,
    next: Option<Box<Subtree<T>>>,
}

impl<T> Subtree<T> {
    pub fn new(node: T, next: Option<Subtree<T>>) -> Self {
        Self {
            node,
            next: next.map(Box::new),
        }
    }

    pub fn next(&self) -> Option<&Subtree<T>> {
        self.next.as_ref().map(AsRef::as_ref)
    }

    pub fn next_mut(&mut self) -> Option<&mut Subtree<T>> {
        self.next.as_mut().map(AsMut::as_mut)
    }

    pub fn take_next(&mut self) -> Option<Subtree<T>> {
        self.next.take().map(|next| *next)
    }

    pub fn node(&self) -> &T {
        &self.node
    }

    pub fn node_mut(&mut self) -> &mut T {
        &mut self.node
    }

    pub fn next_node(&self) -> Option<&T> {
        self.next().map(|next| next.node())
    }
}
