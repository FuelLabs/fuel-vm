use fuel_tx::Address;

pub struct BundleMetadata {
    coinbase: Address,
}

impl BundleMetadata {
    pub fn new(coinbase: Address) -> Self {
        Self { coinbase }
    }
}
