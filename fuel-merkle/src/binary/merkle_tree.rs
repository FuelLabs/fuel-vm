use crate::binary::node::Node;
use crate::digest::Digest;
use std::convert::TryFrom;
use std::marker::PhantomData;

const NODE: [u8; 1] = [0x01];
const LEAF: [u8; 1] = [0x00];

type Data = [u8; 32];
type DataRef<'a> = &'a [u8];
type DataNode = Node<Data>;

pub struct MerkleTree<D: Digest> {
    head: Option<Box<DataNode>>,
    leaves_count: u64,
    phantom: PhantomData<D>,
}

impl<D: Digest> Default for MerkleTree<D> {
    fn default() -> MerkleTree<D> {
        Self {
            head: None,
            leaves_count: 0,
            phantom: PhantomData,
        }
    }
}

impl<D: Digest> MerkleTree<D> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn root(&self) -> Data {
        match self.head() {
            None => Self::empty_sum(),
            Some(ref head) => {
                let mut current = head.clone();
                loop {
                    if current.next().is_none() {
                        break;
                    }

                    let mut node = current;
                    let mut next_node = node.next_mut().take().expect("Cannot take next!");
                    current = Self::join_subtrees(&mut next_node, &node)
                }
                *current.data()
            }
        }
    }

    pub fn leaves_count(&self) -> u64 {
        self.leaves_count
    }

    pub fn push(&mut self, data: DataRef) {
        let node = Self::create_node(self.head.take(), 0, Self::leaf_sum(data));
        self.head = Some(node);
        self.join_all_subtrees();

        self.leaves_count += 1;
    }

    pub fn prove(/**/) {
        todo!();
    }

    //
    // PRIVATE
    //

    fn head(&self) -> &Option<Box<DataNode>> {
        &self.head
    }

    fn join_all_subtrees(&mut self) {
        loop {
            let head = self.head.as_ref().expect("Cannot get head!");
            let head_next = head.next();
            if !(head_next.is_some()
                && head.height() == head_next.as_ref().expect("Cannot get head next!").height())
            {
                break;
            }

            // Merge the two front nodes of the list into a single node
            let mut node = self.head.take().expect("Cannot take head!");
            let mut next_node = node.next_mut().take().expect("Cannot take head next!");
            let joined_node = Self::join_subtrees(&mut next_node, &node);

            self.head = Some(joined_node);
        }
    }

    // Merkle Tree hash of an empty list
    // MTH({}) = Hash()
    fn empty_sum() -> Data {
        let hash = D::new();
        let data = hash.finalize();

        <Data>::try_from(data.as_slice()).unwrap()
    }

    // Merkle tree hash of an n-element list D[n]
    // MTH(D[n]) = Hash(0x01 || MTH(D[0:k]) || MTH(D[k:n])
    fn node_sum(lhs_data: DataRef, rhs_data: DataRef) -> Data {
        let mut hash = D::new();

        hash.update(&NODE);
        hash.update(&lhs_data);
        hash.update(&rhs_data);
        let data = hash.finalize();

        <Data>::try_from(data.as_slice()).unwrap()
    }

    // Merkle tree hash of a list with one entry
    // MTH({d(0)}) = Hash(0x00 || d(0))
    fn leaf_sum(data: DataRef) -> Data {
        let mut hash = D::new();

        hash.update(&LEAF);
        hash.update(&data);
        let data = hash.finalize();

        <Data>::try_from(data.as_slice()).unwrap()
    }

    fn join_subtrees(a: &mut Box<DataNode>, b: &DataNode) -> Box<DataNode> {
        let next = a.next_mut().take();
        let height = a.height() + 1;
        let data = Self::node_sum(a.data(), b.data());
        Self::create_node(next, height, data)
    }

    fn create_node(next: Option<Box<DataNode>>, height: u32, data: Data) -> Box<DataNode> {
        Box::new(DataNode::new(next, height, data))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::sha::Sha256 as Hash;

    type MT = MerkleTree<Hash>;

    fn empty_data() -> Data {
        let hash = Hash::new();
        <Data>::try_from(hash.finalize()).unwrap()
    }

    fn leaf_data(data: DataRef) -> Data {
        let mut hash = Hash::new();
        hash.update(&LEAF);
        hash.update(&data);
        <Data>::try_from(hash.finalize()).unwrap()
    }
    fn node_data(lhs_data: DataRef, rhs_data: DataRef) -> Data {
        let mut hash = Hash::new();
        hash.update(&NODE);
        hash.update(&lhs_data);
        hash.update(&rhs_data);
        <Data>::try_from(hash.finalize()).unwrap()
    }

    #[test]
    fn root_returns_the_hash_of_the_empty_string_when_no_leaves_are_pushed() {
        let mt = MT::new();
        let root = mt.root();

        let expected = empty_data();
        assert_eq!(root, expected);
    }

    #[test]
    fn root_returns_the_hash_of_the_leaf_when_one_leaf_is_pushed() {
        let mut mt = MT::new();

        let data = [1u8; 16];
        mt.push(&data);
        let root = mt.root();

        let expected = leaf_data(&data);
        assert_eq!(root, expected);
    }

    #[test]
    fn root_returns_the_hash_of_the_head_when_4_leaves_are_pushed() {
        let mut mt = MT::new();

        let leaves = [
            "Hello, World!".as_bytes(),
            "Making banana pancakes".as_bytes(),
            "What is love?".as_bytes(),
            "Bob Ross".as_bytes(),
        ];
        for leaf in leaves.iter() {
            mt.push(leaf);
        }
        let root = mt.root();

        //       N3
        //      /  \
        //     /    \
        //   N1      N2
        //  /  \    /  \
        // L1  L2  L3  L4

        let leaf_1 = leaf_data(&leaves[0]);
        let leaf_2 = leaf_data(&leaves[1]);
        let leaf_3 = leaf_data(&leaves[2]);
        let leaf_4 = leaf_data(&leaves[3]);

        let node_1 = node_data(&leaf_1, &leaf_2);
        let node_2 = node_data(&leaf_3, &leaf_4);
        let node_3 = node_data(&node_1, &node_2);

        let expected = node_3;
        assert_eq!(root, expected);
    }

    #[test]
    fn root_returns_the_hash_of_the_head_when_5_leaves_are_pushed() {
        let mut mt = MT::new();

        let leaves = [
            "Hello, World!".as_bytes(),
            "Making banana pancakes".as_bytes(),
            "What is love?".as_bytes(),
            "Bob Ross".as_bytes(),
            "The smell of napalm in the morning".as_bytes(),
        ];
        for leaf in leaves.iter() {
            mt.push(leaf);
        }
        let root = mt.root();

        //          N4
        //         /  \
        //       N3    \
        //      /  \    \
        //     /    \    \
        //   N1      N2   \
        //  /  \    /  \   \
        // L1  L2  L3  L4  L5

        let leaf_1 = leaf_data(&leaves[0]);
        let leaf_2 = leaf_data(&leaves[1]);
        let leaf_3 = leaf_data(&leaves[2]);
        let leaf_4 = leaf_data(&leaves[3]);
        let leaf_5 = leaf_data(&leaves[4]);

        let node_1 = node_data(&leaf_1, &leaf_2);
        let node_2 = node_data(&leaf_3, &leaf_4);
        let node_3 = node_data(&node_1, &node_2);
        let node_4 = node_data(&node_3, &leaf_5);

        let expected = node_4;
        assert_eq!(root, expected);
    }

    #[test]
    fn root_returns_the_hash_of_the_head_when_7_leaves_are_pushed() {
        let mut mt = MT::new();

        let leaves = [
            "Hello, World!".as_bytes(),
            "Making banana pancakes".as_bytes(),
            "What is love?".as_bytes(),
            "Bob Ross".as_bytes(),
            "The smell of napalm in the morning".as_bytes(),
            "Frankly, my dear, I don't give a damn.".as_bytes(),
            "Say hello to my little friend".as_bytes(),
        ];
        for leaf in leaves.iter() {
            mt.push(leaf);
        }
        let root = mt.root();

        //              N6
        //          /        \
        //         /          \
        //       N4            N5
        //      /  \           /\
        //     /    \         /  \
        //   N1      N2      N3   \
        //  /  \    /  \    /  \   \
        // L1  L2  L3  L4  L5  L6  L7

        let leaf_1 = leaf_data(&leaves[0]);
        let leaf_2 = leaf_data(&leaves[1]);
        let leaf_3 = leaf_data(&leaves[2]);
        let leaf_4 = leaf_data(&leaves[3]);
        let leaf_5 = leaf_data(&leaves[4]);
        let leaf_6 = leaf_data(&leaves[5]);
        let leaf_7 = leaf_data(&leaves[6]);

        let node_1 = node_data(&leaf_1, &leaf_2);
        let node_2 = node_data(&leaf_3, &leaf_4);
        let node_3 = node_data(&leaf_5, &leaf_6);
        let node_4 = node_data(&node_1, &node_2);
        let node_5 = node_data(&node_3, &leaf_7);
        let node_6 = node_data(&node_4, &node_5);

        let expected = node_6;
        assert_eq!(root, expected);
    }

    #[test]
    fn leaves_count_returns_the_number_of_leaves_pushed_to_the_tree() {
        let mut mt = MT::new();

        let leaves = [
            "Hello, World!".as_bytes(),
            "Making banana pancakes".as_bytes(),
            "What is love?".as_bytes(),
            "Bob Ross".as_bytes(),
        ];
        for leaf in leaves.iter() {
            mt.push(leaf);
        }

        assert_eq!(mt.leaves_count(), leaves.len() as u64);
    }
}
