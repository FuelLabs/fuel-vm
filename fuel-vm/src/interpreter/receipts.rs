use core::ops::Index;
use std::mem;

use fuel_merkle::binary;
use fuel_tx::Receipt;
use fuel_types::bytes::SerializableVec;

#[derive(Debug, Clone)]
pub(crate) struct ReceiptsCtx {
    receipts: Vec<Receipt>,
    receipts_tree: binary::in_memory::MerkleTree,
}

impl Default for ReceiptsCtx {
    fn default() -> Self {
        Self {
            receipts: Default::default(),
            receipts_tree: Default::default(),
        }
    }
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

    pub fn recalculate_root(&mut self) {
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

impl AsMut<Vec<Receipt>> for ReceiptsCtx {
    fn as_mut(&mut self) -> &mut Vec<Receipt> {
        &mut self.receipts
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
        mem::take(ctx.as_mut())
    }
}
