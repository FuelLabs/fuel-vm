use core::ops::Index;
use fuel_crypto::Hasher;
use std::mem;

use fuel_merkle::binary;
use fuel_tx::Receipt;
use fuel_types::{bytes::SerializableVec, Bytes32};

#[derive(Debug, Default, Clone)]
pub(crate) struct ReceiptsCtx {
    receipts: Vec<Receipt>,
    inner: Inner,
}

impl ReceiptsCtx {
    pub fn new(accum: bool) -> Self {
        let inner = if accum {
            Inner::Accumulator {
                current_root: Bytes32::zeroed(),
            }
        } else {
            Inner::Merkle {
                receipts_tree: Default::default(),
            }
        };
        Self {
            receipts: vec![],
            inner,
        }
    }
}

#[derive(Debug, Clone)]
enum Inner {
    Merkle {
        receipts_tree: binary::in_memory::MerkleTree,
    },
    Accumulator {
        current_root: Bytes32,
    },
}

impl Default for Inner {
    fn default() -> Self {
        Inner::Merkle {
            receipts_tree: Default::default(),
        }
    }
}

impl ReceiptsCtx {
    pub fn push(&mut self, mut receipt: Receipt) {
        let receipt_bytes = receipt.to_bytes();
        match &mut self.inner {
            Inner::Merkle { receipts_tree } => {
                receipts_tree.push(receipt_bytes.as_slice());
            }
            Inner::Accumulator { current_root } => {
                *current_root = Hasher::default()
                    .chain(current_root.as_slice())
                    .chain(receipt_bytes.as_slice())
                    .finalize()
                    .into();
            }
        }
        self.receipts.push(receipt)
    }

    pub fn clear(&mut self) {
        match &mut self.inner {
            Inner::Merkle { receipts_tree } => receipts_tree.reset(),
            Inner::Accumulator { current_root } => *current_root = Bytes32::zeroed(),
        }
        self.receipts.clear();
    }

    pub fn len(&self) -> usize {
        self.receipts.len()
    }

    pub fn root(&self) -> Bytes32 {
        match &self.inner {
            Inner::Merkle { receipts_tree } => receipts_tree.root().into(),
            Inner::Accumulator { current_root } => *current_root,
        }
    }

    /// Get a mutable lock on this context
    pub fn lock(&mut self) -> ReceiptsCtxMut {
        ReceiptsCtxMut::new(self)
    }

    /// Recalculates the Merkle root of the receipts from scratch. This should
    /// only be used when the list of receipts has been mutated externally.
    fn recalculate_root(&mut self) {
        let receipts = self.as_ref().clone();
        self.clear();
        for receipt in receipts {
            self.push(receipt);
        }
    }
}

impl Index<usize> for ReceiptsCtx {
    type Output = Receipt;

    fn index(&self, index: usize) -> &Self::Output {
        &self.as_ref()[index]
    }
}

impl AsRef<Vec<Receipt>> for ReceiptsCtx {
    fn as_ref(&self) -> &Vec<Receipt> {
        &self.receipts
    }
}

impl PartialEq for ReceiptsCtx {
    fn eq(&self, other: &Self) -> bool {
        self.root() == other.root()
    }
}

impl Eq for ReceiptsCtx {}

impl From<Vec<Receipt>> for ReceiptsCtx {
    fn from(receipts: Vec<Receipt>) -> Self {
        let mut ctx = Self::default();
        for receipt in receipts {
            ctx.push(receipt)
        }
        ctx
    }
}

impl From<ReceiptsCtx> for Vec<Receipt> {
    fn from(mut ctx: ReceiptsCtx) -> Self {
        mem::take(&mut ctx.receipts)
    }
}

/// When modifying the list of receipts directly, it is imperative that the
/// receipts tree remain in sync with the modified list. Therefore, we lock
/// mutable access to the list of receipts behind an opaque context with
/// explicit getters and automatic root calculation. As long as the mutable
/// context is in scope, access to the original context is forbidden due to
/// borrowing semantics. When the mutable context is dropped, we recalculate the
/// root and return access to the original context.
pub(crate) struct ReceiptsCtxMut<'a> {
    receipts_ctx: &'a mut ReceiptsCtx,
}

impl<'a> ReceiptsCtxMut<'a> {
    pub fn new(receipts_ctx: &'a mut ReceiptsCtx) -> Self {
        Self { receipts_ctx }
    }

    pub fn receipts_mut(&mut self) -> &mut Vec<Receipt> {
        &mut self.receipts_ctx.receipts
    }
}

impl<'a> Drop for ReceiptsCtxMut<'a> {
    fn drop(&mut self) {
        // The receipts may have been modified; recalculate the root
        self.receipts_ctx.recalculate_root()
    }
}

#[cfg(test)]
mod tests {
    use crate::crypto::ephemeral_merkle_root;
    use crate::interpreter::receipts::ReceiptsCtx;
    use fuel_tx::Receipt;
    use fuel_types::bytes::SerializableVec;
    use std::iter;

    fn create_receipt() -> Receipt {
        Receipt::call(
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
        )
    }

    #[test]
    fn root_returns_merkle_root_of_pushed_receipts() {
        let mut ctx = ReceiptsCtx::default();
        let receipts = iter::repeat(create_receipt()).take(5);
        for receipt in receipts.clone() {
            ctx.push(receipt)
        }

        let root = ctx.root();

        let leaves = receipts
            .map(|mut receipt| receipt.to_bytes())
            .collect::<Vec<_>>()
            .into_iter();
        let expected_root = ephemeral_merkle_root(leaves);
        assert_eq!(root, expected_root)
    }

    #[test]
    fn root_returns_merkle_root_of_directly_modified_receipts() {
        let mut ctx = ReceiptsCtx::default();
        let receipts = iter::repeat(create_receipt()).take(5);

        {
            let mut ctx_mut = ctx.lock();
            *ctx_mut.receipts_mut() = receipts.clone().collect();
        }

        let root = ctx.root();

        let leaves = receipts
            .map(|mut receipt| receipt.to_bytes())
            .collect::<Vec<_>>()
            .into_iter();
        let expected_root = ephemeral_merkle_root(leaves);
        assert_eq!(root, expected_root)
    }
}
