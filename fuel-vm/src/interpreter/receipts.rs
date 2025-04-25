use alloc::vec::Vec;
use core::{
    mem,
    ops::Index,
};
use fuel_asm::PanicReason;

use fuel_merkle::binary::root_calculator::MerkleRootCalculator as MerkleTree;
use fuel_tx::Receipt;
use fuel_types::{
    Bytes32,
    canonical::Serialize,
};

use crate::{
    error::SimpleResult,
    prelude::{
        Bug,
        BugVariant,
    },
};

/// Receipts and the associated Merkle tree
#[derive(Debug, Default, Clone)]
pub struct ReceiptsCtx {
    receipts: Vec<Receipt>,
    receipts_tree: MerkleTree,
}

impl ReceiptsCtx {
    /// The maximum number of receipts that can be stored in a single context.
    /// https://github.com/FuelLabs/fuel-specs/blob/master/src/fuel-vm/instruction-set.md#Receipts
    pub const MAX_RECEIPTS: usize = u16::MAX as usize;

    /// Add a new receipt, updating the Merkle tree as well.
    /// Returns a panic if the context is full.
    pub fn push(&mut self, receipt: Receipt) -> SimpleResult<()> {
        if self.receipts.len() == Self::MAX_RECEIPTS {
            return Err(Bug::new(BugVariant::ReceiptsCtxFull).into())
        }

        // Last two slots can be only used for ending the script,
        // with a script result optinally preceded by a panic
        if (self.receipts.len() == Self::MAX_RECEIPTS - 1
            && !matches!(receipt, Receipt::ScriptResult { .. }))
            || (self.receipts.len() == Self::MAX_RECEIPTS - 2
                && !matches!(
                    receipt,
                    Receipt::ScriptResult { .. } | Receipt::Panic { .. }
                ))
        {
            return Err(PanicReason::TooManyReceipts.into())
        }

        self.receipts_tree.push(receipt.to_bytes().as_slice());
        self.receipts.push(receipt);
        Ok(())
    }

    /// Reset the context to an empty state
    pub fn clear(&mut self) {
        self.receipts_tree = MerkleTree::new();
        self.receipts.clear();
    }

    /// Return how many receipts are in this context
    pub fn len(&self) -> usize {
        self.receipts.len()
    }

    /// Returns `true` if the context has no receipts.
    pub fn is_empty(&self) -> bool {
        self.receipts.len() == 0
    }

    /// Return current Merkle root of the receipts
    pub fn root(&self) -> Bytes32 {
        self.receipts_tree.clone().root().into()
    }

    /// Get a mutable lock on this context
    pub fn lock(&mut self) -> ReceiptsCtxMut {
        ReceiptsCtxMut::new(self)
    }

    /// Recalculates the Merkle root of the receipts from scratch. This should
    /// only be used when the list of receipts has been mutated externally.
    fn recalculate_root(&mut self) {
        self.receipts_tree = MerkleTree::new();
        for receipt in &self.receipts {
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

#[cfg(any(test, feature = "test-helpers"))]
impl From<Vec<Receipt>> for ReceiptsCtx {
    fn from(receipts: Vec<Receipt>) -> Self {
        let mut ctx = Self::default();
        for receipt in receipts {
            ctx.push(receipt).expect("Too many receipts");
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
pub struct ReceiptsCtxMut<'a> {
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

impl Drop for ReceiptsCtxMut<'_> {
    fn drop(&mut self) {
        // The receipts may have been modified; recalculate the root
        self.receipts_ctx.recalculate_root()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        crypto::ephemeral_merkle_root,
        interpreter::receipts::ReceiptsCtx,
    };
    use core::iter;
    use fuel_tx::Receipt;
    use fuel_types::canonical::Serialize;

    use alloc::vec::Vec;

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
            ctx.push(receipt).expect("context not full");
        }

        let root = ctx.root();

        let leaves = receipts
            .map(|receipt| receipt.to_bytes())
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
            .map(|receipt| receipt.to_bytes())
            .collect::<Vec<_>>()
            .into_iter();
        let expected_root = ephemeral_merkle_root(leaves);
        assert_eq!(root, expected_root)
    }
}
