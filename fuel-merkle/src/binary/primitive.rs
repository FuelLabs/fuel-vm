use crate::{
    binary::Node,
    common::{
        Bytes32,
        Position,
    },
};

pub type Primitive = (u64, Bytes32);

pub trait PrimitiveView {
    fn position(&self) -> Position;
    fn hash(&self) -> &Bytes32;
}

impl PrimitiveView for Primitive {
    fn position(&self) -> Position {
        Position::from_in_order_index(self.0)
    }

    fn hash(&self) -> &Bytes32 {
        &self.1
    }
}

impl From<&Node> for Primitive {
    fn from(node: &Node) -> Self {
        (node.position().in_order_index(), *node.hash())
    }
}

impl From<Primitive> for Node {
    fn from(primitive: Primitive) -> Self {
        let position = primitive.position();
        let hash = *primitive.hash();
        Node::new(position, hash)
    }
}
