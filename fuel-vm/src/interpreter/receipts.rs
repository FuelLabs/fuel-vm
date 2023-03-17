use core::ops::Index;
use std::mem;

use fuel_merkle::binary;
use fuel_tx::Receipt;
use fuel_types::bytes::SerializableVec;

#[derive(Debug, Default, Clone)]
pub(crate) struct ReceiptsCtx {
    receipts: Vec<Receipt>,
    receipts_tree: binary::in_memory::MerkleTree,
}

impl ReceiptsCtx {
    pub fn push(&mut self, receipt: Receipt) {
        self.receipts_tree.push(receipt.clone().to_bytes().as_slice());
        self.receipts.push(receipt)
    }

    pub fn clear(&mut self) {
        self.receipts_tree.reset();
        self.receipts.clear();
    }

    pub fn root(&self) -> [u8; 32] {
        self.receipts_tree.root()
    }

    /// Get a mutable lock on this context
    pub fn lock(&mut self) -> ReceiptsCtxMut {
        ReceiptsCtxMut::new(self)
    }

    /// Recalculates the Merkle root of the receipts from scratch. This should
    /// only be used when the list of receipts has been mutated externally.
    fn recalculate_root(&mut self) {
        self.receipts_tree.reset();
        let receipts = self.as_ref().clone();
        for mut receipt in receipts {
            self.receipts_tree.push(receipt.to_bytes().as_slice())
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
