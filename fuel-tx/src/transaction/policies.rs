use core::ops::Deref;
use fuel_types::{
    canonical::{
        Deserialize,
        Error,
        Input,
        Output,
        Serialize,
    },
    BlockHeight,
    Word,
};

#[cfg(feature = "random")]
use rand::{
    distributions::{
        Distribution,
        Standard,
    },
    Rng,
};

bitflags::bitflags! {
    /// See https://github.com/FuelLabs/fuel-specs/blob/master/src/tx-format/policy.md#policy
    #[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Hash)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct PoliciesBits: u32 {
        /// If set, the gas price is present in the policies.
        const GasPrice = 1 << 0;
        /// If set, the witness limit is present in the policies.
        const WitnessLimit = 1 << 1;
        /// If set, the maturity is present in the policies.
        const Maturity = 1 << 2;
        /// If set, the max fee is present in the policies.
        const MaxFee = 1 << 3;
    }
}

/// The helper enum to make user-friendly API for [`Policies::set`] and [`Policies::get`]
/// methods.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    strum_macros::EnumCount,
    strum_macros::EnumIter,
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PolicyType {
    GasPrice,
    WitnessLimit,
    Maturity,
    MaxFee,
}

impl PolicyType {
    pub const fn index(&self) -> usize {
        match self {
            PolicyType::GasPrice => 0,
            PolicyType::WitnessLimit => 1,
            PolicyType::Maturity => 2,
            PolicyType::MaxFee => 3,
        }
    }

    pub const fn bit(&self) -> PoliciesBits {
        match self {
            PolicyType::GasPrice => PoliciesBits::GasPrice,
            PolicyType::WitnessLimit => PoliciesBits::WitnessLimit,
            PolicyType::Maturity => PoliciesBits::Maturity,
            PolicyType::MaxFee => PoliciesBits::MaxFee,
        }
    }
}

/// The total number of policies.
pub const POLICIES_NUMBER: usize = PoliciesBits::all().bits().count_ones() as usize;

/// Container for managing policies.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "typescript", wasm_bindgen::prelude::wasm_bindgen)]
pub struct Policies {
    /// A bitmask that indicates what policies are set.
    bits: PoliciesBits,
    /// The array of policy values.
    values: [Word; POLICIES_NUMBER],
}

impl Policies {
    /// Creates an empty `Self`.
    pub const fn new() -> Self {
        Self {
            bits: PoliciesBits::empty(),
            values: [0; POLICIES_NUMBER],
        }
    }

    /// Returns `true` if no policies are set.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the number of set policies.
    pub fn len(&self) -> usize {
        self.bits.bits().count_ones() as usize
    }

    /// Returns the bit mask of the policies.
    pub fn bits(&self) -> u32 {
        self.bits.bits()
    }

    /// Sets the `gas_price` policy.
    pub fn with_gas_price(mut self, gas_price: Word) -> Self {
        self.set(PolicyType::GasPrice, Some(gas_price));
        self
    }

    /// Sets the `witness_limit` policy.
    pub fn with_witness_limit(mut self, witness_limit: Word) -> Self {
        self.set(PolicyType::WitnessLimit, Some(witness_limit));
        self
    }

    /// Sets the `maturity` policy.
    pub fn with_maturity(mut self, maturity: BlockHeight) -> Self {
        self.set(PolicyType::Maturity, Some(*maturity.deref() as u64));
        self
    }

    /// Sets the `max_fee` policy.
    pub fn with_max_fee(mut self, max_fee: Word) -> Self {
        self.set(PolicyType::MaxFee, Some(max_fee));
        self
    }

    /// Returns a policy's value if the corresponding bit is set.
    pub fn get(&self, policy_type: PolicyType) -> Option<Word> {
        if self.bits.contains(policy_type.bit()) {
            Some(self.values[policy_type.index()])
        } else {
            None
        }
    }

    /// Returns a policy's type by the `index`.
    pub fn get_type_by_index(&self, index: usize) -> Option<u32> {
        self.bits.iter().nth(index).map(|bit| bit.bits())
    }

    /// Sets a policy's value if the `value` is `Some`, otherwise, unset it.
    pub fn set(&mut self, policy_type: PolicyType, value: Option<Word>) {
        if let Some(value) = value {
            self.bits.insert(policy_type.bit());
            self.values[policy_type.index()] = value;
        } else {
            self.bits.remove(policy_type.bit());
            self.values[policy_type.index()] = 0;
        }
    }

    /// Returns `true` if the `Self` follows all rules from the specification.
    pub fn is_valid(&self) -> bool {
        let expected_values = Self::values_for_bitmask(self.bits, self.values);

        if self.bits.bits() > PoliciesBits::all().bits() {
            return false
        }

        if self.values != expected_values {
            return false
        }

        if let Some(maturity) = self.get(PolicyType::Maturity) {
            if maturity > u32::MAX as u64 {
                return false
            }
        }

        true
    }

    /// Helper function to generate values arrays based on the `PoliciesBits`.
    fn values_for_bitmask(
        bits: PoliciesBits,
        default_values: [Word; POLICIES_NUMBER],
    ) -> [Word; POLICIES_NUMBER] {
        use strum::IntoEnumIterator;
        let mut values = [0; POLICIES_NUMBER];
        for policy_type in PolicyType::iter() {
            if bits.contains(policy_type.bit()) {
                values[policy_type.index()] = default_values[policy_type.index()];
            }
        }
        values
    }
}

impl Serialize for Policies {
    fn size_static(&self) -> usize {
        self.bits.bits().size_static()
    }

    fn size_dynamic(&self) -> usize {
        self.bits.bits().count_ones() as usize * Word::MIN.size()
    }

    fn encode_static<O: Output + ?Sized>(&self, buffer: &mut O) -> Result<(), Error> {
        self.bits.bits().encode_static(buffer)
    }

    fn encode_dynamic<O: Output + ?Sized>(&self, buffer: &mut O) -> Result<(), Error> {
        for (value, bit) in self.values.iter().zip(PoliciesBits::all().iter()) {
            if self.bits.contains(bit) {
                value.encode(buffer)?;
            }
        }
        Ok(())
    }
}

impl Deserialize for Policies {
    fn decode_static<I: Input + ?Sized>(buffer: &mut I) -> Result<Self, Error> {
        let bits = u32::decode(buffer)?;
        let bits = PoliciesBits::from_bits(bits)
            .ok_or(Error::Unknown("Invalid policies bits"))?;
        Ok(Self {
            bits,
            values: Default::default(),
        })
    }

    fn decode_dynamic<I: Input + ?Sized>(&mut self, buffer: &mut I) -> Result<(), Error> {
        for (index, bit) in PoliciesBits::all().iter().enumerate() {
            if self.bits.contains(bit) {
                self.values[index] = Word::decode(buffer)?;
            }
        }

        if let Some(maturity) = self.get(PolicyType::Maturity) {
            if maturity > u32::MAX as u64 {
                return Err(Error::Unknown("The maturity in more than `u32::MAX`"))
            }
        }

        Ok(())
    }
}

#[cfg(feature = "random")]
impl Distribution<Policies> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Policies {
        let bits: u32 = rng.gen();
        let bits = bits & PoliciesBits::all().bits();
        let bits = PoliciesBits::from_bits(bits).expect("We checked that above");
        let values = rng.gen();
        let mut policies = Policies {
            bits,
            values: Policies::values_for_bitmask(bits, values),
        };

        if policies.get(PolicyType::Maturity).is_some() {
            let maturity: u32 = rng.gen();
            policies.set(PolicyType::Maturity, Some(maturity as u64));
        }

        policies
    }
}

#[cfg(feature = "typescript")]
pub mod typescript {
    use wasm_bindgen::prelude::*;

    use crate::transaction::Policies;
    use alloc::{
        format,
        string::String,
        vec::Vec,
    };

    #[wasm_bindgen]
    impl Policies {
        #[wasm_bindgen(constructor)]
        pub fn typescript_new() -> Policies {
            Policies::default()
        }

        #[cfg(feature = "serde")]
        #[wasm_bindgen(js_name = toJSON)]
        pub fn to_json(&self) -> String {
            serde_json::to_string(&self).expect("unable to json format")
        }

        #[wasm_bindgen(js_name = toString)]
        pub fn typescript_to_string(&self) -> String {
            format!("{:?}", self)
        }

        #[wasm_bindgen(js_name = to_bytes)]
        pub fn typescript_to_bytes(&self) -> Vec<u8> {
            use fuel_types::canonical::Serialize;
            <Self as Serialize>::to_bytes(self)
        }

        #[wasm_bindgen(js_name = from_bytes)]
        pub fn typescript_from_bytes(value: &[u8]) -> Result<Policies, js_sys::Error> {
            use fuel_types::canonical::Deserialize;
            <Self as Deserialize>::from_bytes(value)
                .map_err(|e| js_sys::Error::new(&format!("{:?}", e)))
        }
    }
}

#[test]
fn values_for_bitmask_produces_expected_values() {
    const MAX_BITMASK: u32 = 1 << POLICIES_NUMBER;
    const VALUES: [Word; POLICIES_NUMBER] = [0x1000001, 0x2000001, 0x3000001, 0x4000001];

    // Given
    let mut set = hashbrown::HashSet::new();

    // When
    for bitmask in 0..MAX_BITMASK {
        let bits =
            PoliciesBits::from_bits(bitmask).expect("Should construct a valid bits");
        set.insert(Policies::values_for_bitmask(bits, VALUES));
    }

    // Then
    assert_eq!(set.len(), MAX_BITMASK as usize);
}

#[test]
fn canonical_serialization_deserialization_for_any_combination_of_values_works() {
    const MAX_BITMASK: u32 = 1 << POLICIES_NUMBER;
    const VALUES: [Word; POLICIES_NUMBER] = [0x1000001, 0x2000001, 0x3000001, 0x4000001];

    for bitmask in 0..MAX_BITMASK {
        let bits =
            PoliciesBits::from_bits(bitmask).expect("Should construct a valid bits");
        let policies = Policies {
            bits,
            values: Policies::values_for_bitmask(bits, VALUES),
        };

        let size = policies.size();
        let mut buffer = vec![0u8; size];
        policies
            .encode(&mut buffer.as_mut_slice())
            .expect("Should encode without error");

        let new_policies = Policies::decode(&mut buffer.as_slice())
            .expect("Should decode without error");

        assert_eq!(policies, new_policies);
        assert_eq!(new_policies.bits.bits(), bitmask);

        for (index, bit) in PoliciesBits::all().iter().enumerate() {
            if policies.bits.contains(bit) {
                assert_eq!(VALUES[index], new_policies.values[index]);
            } else {
                assert_eq!(0, new_policies.values[index]);
            }
        }

        assert_eq!(new_policies.size(), size);
        // `bitmask.count_ones()` - the number of serialized values
        assert_eq!(
            size,
            (policies.bits.bits().size()
                + bitmask.count_ones() as usize * Word::MIN.size())
        );
    }
}
