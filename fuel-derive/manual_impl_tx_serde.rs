impl fuel_types::canonical::SerializedSizeVariable for Transaction {
    fn size_static(&self) -> usize {
        match self {
            Transaction::Script(_) => Script::SIZE_STATIC,
            Transaction::Create(_) => Create::SIZE_STATIC,
            Transaction::Mint(_) => Mint::SIZE_STATIC,
        }
    }
}

impl fuel_types::canonical::Serialize for Transaction {
    const SIZE_NO_DYNAMIC: bool = false;

    fn size_dynamic(&self) -> usize {
        match self {
            Transaction::Script(t) => t.size_dynamic(),
            Transaction::Create(t) => t.size_dynamic(),
            Transaction::Mint(t) => t.size_dynamic(),
        }
    }

    fn encode_static<O: fuel_types::canonical::Output + ?Sized>(
        &self,
        buffer: &mut O,
    ) -> Result<(), fuel_types::canonical::Error> {
        // Since the variants are prefixed with discriminant already, just encode them
        // directlu
        match self {
            Transaction::Script(t) => t.encode_static(buffer),
            Transaction::Create(t) => t.encode_static(buffer),
            Transaction::Mint(t) => t.encode_static(buffer),
        }
    }

    fn encode_dynamic<O: fuel_types::canonical::Output + ?Sized>(
        &self,
        buffer: &mut O,
    ) -> Result<(), fuel_types::canonical::Error> {
        match self {
            Transaction::Script(t) => t.encode_dynamic(buffer),
            Transaction::Create(t) => t.encode_dynamic(buffer),
            Transaction::Mint(t) => t.encode_dynamic(buffer),
        }
    }
}

impl fuel_types::canonical::Deserialize for Transaction {
    fn decode_static<I: fuel_types::canonical::Input + ?Sized>(
        buffer: &mut I,
    ) -> Result<Self, fuel_types::canonical::Error> {
        // The variants are all prefixed with discriminant
        let buf = buffer.clone();
        let raw_discr =
            <::core::primitive::u64 as fuel_types::canonical::Deserialize>::decode(
                buffer,
            )?;
        *buffer = buf; // Restore buffer position
        let repr = TransactionRepr::try_from_primitive(raw_discr)
            .map_err(|_| fuel_types::canonical::Error::UnknownDiscriminant)?;
        match repr {
            TransactionRepr::Script => Ok(Self::Script(Script::decode_static(buffer)?)),
            TransactionRepr::Create => Ok(Self::Create(Create::decode_static(buffer)?)),
            TransactionRepr::Mint => Ok(Self::Mint(Mint::decode_static(buffer)?)),
        }
    }

    fn decode_dynamic<I: fuel_types::canonical::Input + ?Sized>(
        &mut self,
        buffer: &mut I,
    ) -> Result<(), fuel_types::canonical::Error> {
        match self {
            Transaction::Script(t) => t.decode_dynamic(buffer),
            Transaction::Create(t) => t.decode_dynamic(buffer),
            Transaction::Mint(t) => t.decode_dynamic(buffer),
        }
    }
}
