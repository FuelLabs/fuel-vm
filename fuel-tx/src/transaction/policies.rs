use alloc::vec::Vec;
use core::{
    fmt,
    marker::PhantomData,
    ops::Deref,
};
use fuel_types::{
    BlockHeight,
    Word,
    canonical::{
        Deserialize,
        Error,
        Input,
        Output,
        Serialize,
    },
};
use hashbrown::HashMap;

#[cfg(feature = "random")]
use rand::{
    Rng,
    distributions::{
        Distribution,
        Standard,
    },
};
use serde::ser::SerializeStruct;

bitflags::bitflags! {
    /// See https://github.com/FuelLabs/fuel-specs/blob/master/src/tx-format/policy.md#policy
    #[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Hash)]
    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct PoliciesBits: u32 {
        /// If set, the gas price is present in the policies.
        const Tip = 1 << 0;
        /// If set, the witness limit is present in the policies.
        const WitnessLimit = 1 << 1;
        /// If set, the maturity is present in the policies.
        const Maturity = 1 << 2;
        /// If set, the max fee is present in the policies.
        const MaxFee = 1 << 3;
        /// If set, the expiration is present in the policies.
        const Expiration = 1 << 4;
        /// If set, the owner is present in the policies.
        const Owner = 1 << 5;
    }
}

#[cfg(feature = "da-compression")]
impl fuel_compression::Compressible for PoliciesBits {
    type Compressed = u32;
}

#[cfg(feature = "da-compression")]
impl<Ctx> fuel_compression::CompressibleBy<Ctx> for PoliciesBits
where
    Ctx: fuel_compression::ContextError,
{
    async fn compress_with(&self, _: &mut Ctx) -> Result<Self::Compressed, Ctx::Error> {
        Ok(self.bits())
    }
}

#[cfg(feature = "da-compression")]
impl<Ctx> fuel_compression::DecompressibleBy<Ctx> for PoliciesBits
where
    Ctx: fuel_compression::ContextError,
{
    async fn decompress_with(c: Self::Compressed, _: &Ctx) -> Result<Self, Ctx::Error> {
        Ok(Self::from_bits_truncate(c))
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
    serde::Serialize,
    serde::Deserialize,
)]
pub enum PolicyType {
    Tip,
    WitnessLimit,
    Maturity,
    MaxFee,
    Expiration,
    Owner,
}

impl PolicyType {
    pub const fn index(&self) -> usize {
        match self {
            PolicyType::Tip => 0,
            PolicyType::WitnessLimit => 1,
            PolicyType::Maturity => 2,
            PolicyType::MaxFee => 3,
            PolicyType::Expiration => 4,
            PolicyType::Owner => 5,
        }
    }

    pub const fn bit(&self) -> PoliciesBits {
        match self {
            PolicyType::Tip => PoliciesBits::Tip,
            PolicyType::WitnessLimit => PoliciesBits::WitnessLimit,
            PolicyType::Maturity => PoliciesBits::Maturity,
            PolicyType::MaxFee => PoliciesBits::MaxFee,
            PolicyType::Expiration => PoliciesBits::Expiration,
            PolicyType::Owner => PoliciesBits::Owner,
        }
    }
}

/// The total number of policies.
pub const POLICIES_NUMBER: usize = PoliciesBits::all().bits().count_ones() as usize;

/// Container for managing policies.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "typescript", wasm_bindgen::prelude::wasm_bindgen)]
pub struct Policies {
    /// A bitmask that indicates what policies are set.
    bits: PoliciesBits,
    /// The array of policy values.
    values: [Word; POLICIES_NUMBER],
    /// Storage for unknown policies to enable forward compatibility and round-trip serialization.
    /// Maps from policy bit position (0-31) to policy value.
    /// This allows newer clients to preserve unknown policies when deserializing and re-serializing.
    #[cfg_attr(feature = "typescript", wasm_bindgen(skip))]
    unknown_policies: HashMap<u8, Word>,
    /// Raw bits value including unknown bits, stored during deserialization
    #[cfg_attr(feature = "typescript", wasm_bindgen(skip))]
    raw_bits: u32,
}

impl Default for Policies {
    fn default() -> Self {
        Self::new()
    }
}

// Manual Hash implementation that only hashes known fields for semantic equality
// Unknown policies are ignored for hashing to maintain backward compatibility
impl core::hash::Hash for Policies {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.bits.hash(state);
        self.values.hash(state);
        // Intentionally not hashing unknown_policies and raw_bits
        // to maintain semantic equality based on known policies only
    }
}

impl Policies {
    /// Creates an empty `Self`.
    pub fn new() -> Self {
        Self {
            bits: PoliciesBits::empty(),
            values: [0; POLICIES_NUMBER],
            unknown_policies: HashMap::new(),
            raw_bits: 0,
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
    pub fn with_tip(mut self, tip: Word) -> Self {
        self.set(PolicyType::Tip, Some(tip));
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

    /// Sets the `expiration` policy.
    pub fn with_expiration(mut self, expiration: BlockHeight) -> Self {
        self.set(PolicyType::Expiration, Some(*expiration.deref() as u64));
        self
    }

    /// Sets the `max_fee` policy.
    pub fn with_max_fee(mut self, max_fee: Word) -> Self {
        self.set(PolicyType::MaxFee, Some(max_fee));
        self
    }

    /// Sets the `owner` policy.
    pub fn with_owner(mut self, owner: Word) -> Self {
        self.set(PolicyType::Owner, Some(owner));
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

    /// Returns `true` if the policy is set.
    pub fn is_set(&self, policy_type: PolicyType) -> bool {
        self.bits.contains(policy_type.bit())
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
            self.raw_bits |= policy_type.bit().bits();
        } else {
            self.bits.remove(policy_type.bit());
            self.values[policy_type.index()] = 0;
            self.raw_bits &= !policy_type.bit().bits();
        }
    }

    /// Returns `true` if the `Self` follows all rules from the specification.
    pub fn is_valid(&self) -> bool {
        let expected_values = Self::values_for_bitmask(self.bits, self.values);

        if self.bits.bits() > PoliciesBits::all().bits() {
            return false;
        }

        if self.values != expected_values {
            return false;
        }

        if let Some(maturity) = self.get(PolicyType::Maturity)
            && maturity > u32::MAX as u64
        {
            return false;
        }

        if let Some(expiration) = self.get(PolicyType::Expiration)
            && expiration > u32::MAX as u64
        {
            return false;
        }

        if let Some(owner) = self.get(PolicyType::Owner)
            && owner > u32::MAX as u64
        {
            return false;
        }

        // Validate unknown policies don't have bit positions that conflict with known policies
        for bit_position in self.unknown_policies.keys() {
            if *bit_position < 32 && PoliciesBits::all().bits() & (1 << bit_position) != 0 {
                return false;
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

// This serde is manually implemented because of the `values` field format.
// Serialization of the `values` field :
// 1. Always write the 4 elements for the policies `Maturity, MaxFee, Tip, WitnessLimit`,
//    even if they are not set for backward compatibility.
// 2. For the remaining, write the value only if the policy is set.
impl serde::Serialize for Policies {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("Policies", 2)?;
        state.serialize_field("bits", &self.bits)?;
        // For the `values` field, we always write the 4 elements for the first 4 policies
        // and then write the value only if the policy is set.

        // Previous behavior
        if self.bits.intersection(PoliciesBits::all())
            == self.bits.intersection(
                PoliciesBits::Maturity
                    .union(PoliciesBits::MaxFee)
                    .union(PoliciesBits::Tip)
                    .union(PoliciesBits::WitnessLimit),
            )
        {
            let first_four_values: [Word; 4] =
                self.values[..4].try_into().map_err(|_| {
                    serde::ser::Error::custom("The first 4 values should be present")
                })?;
            state.serialize_field("values", &first_four_values)?;
        // New backward compatible behavior
        } else {
            let mut values = Vec::new();
            for (value, bit) in self.values.iter().zip(PoliciesBits::all().iter()) {
                if self.bits.contains(bit) {
                    values.push(*value);
                }
            }
            state.serialize_field("values", &values)?;
        }
        state.end()
    }
}

// Most of the code is copy-paste from the auto-generated code
// by `serde::Deserialize` derive macro with small modifications to support
// backward compatibility.
//
// See description of the https://github.com/FuelLabs/fuel-vm/pull/878.
impl<'de> serde::Deserialize<'de> for Policies {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        enum Field {
            Bits,
            Values,
            Ignore,
        }
        struct FieldVisitor;
        impl serde::de::Visitor<'_> for FieldVisitor {
            type Value = Field;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("field identifier")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "bits" => Ok(Field::Bits),
                    "values" => Ok(Field::Values),
                    _ => Ok(Field::Ignore),
                }
            }

            fn visit_bytes<E>(self, value: &[u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    b"bits" => Ok(Field::Bits),
                    b"values" => Ok(Field::Values),
                    _ => Ok(Field::Ignore),
                }
            }
        }
        impl<'de> serde::Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                deserializer.deserialize_identifier(FieldVisitor)
            }
        }
        struct StructVisitor<'de> {
            marker: PhantomData<Policies>,
            lifetime: PhantomData<&'de ()>,
        }
        impl<'de> serde::de::Visitor<'de> for StructVisitor<'de> {
            type Value = Policies;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct Policies")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let bits = match seq.next_element::<PoliciesBits>()? {
                    Some(bits) => bits,
                    None => {
                        return Err(serde::de::Error::invalid_length(
                            0,
                            &"struct Policies with 2 elements",
                        ))
                    }
                };
                // For the `values` field, we always write the 4 elements for the first 4
                // policies and then write the value only if the policy is
                // set.
                // Previous behavior
                if bits.intersection(PoliciesBits::all())
                    == bits.intersection(
                        PoliciesBits::Maturity
                            .union(PoliciesBits::MaxFee)
                            .union(PoliciesBits::Tip)
                            .union(PoliciesBits::WitnessLimit),
                    )
                {
                    let decoded_values: [Word; 4] =
                        match seq.next_element::<[Word; 4]>()? {
                            Some(values) => values,
                            None => {
                                return Err(serde::de::Error::invalid_length(
                                    1,
                                    &"struct Policies with 2 elements",
                                ))
                            }
                        };
                    let mut values: [Word; POLICIES_NUMBER] = [0; POLICIES_NUMBER];
                    values[..4].copy_from_slice(&decoded_values);
                    Ok(Policies { bits, values, unknown_policies: HashMap::new(), raw_bits: bits.bits() })
                // New backward compatible behavior
                } else {
                    let decoded_values = match seq.next_element::<Vec<Word>>()? {
                        Some(values) => values,
                        None => {
                            return Err(serde::de::Error::invalid_length(
                                1,
                                &"struct Policies with 2 elements",
                            ))
                        }
                    };
                    let mut values: [Word; POLICIES_NUMBER] = [0; POLICIES_NUMBER];
                    let mut decoded_index = 0;
                    for (index, bit) in PoliciesBits::all().iter().enumerate() {
                        if bits.contains(bit) {
                            values[index] = *decoded_values.get(decoded_index).ok_or(
                                serde::de::Error::custom(
                                    "The values array isn't synchronized with the bits",
                                ),
                            )?;
                            decoded_index = decoded_index.checked_add(1).ok_or(
                                serde::de::Error::custom(
                                    "Too many values in the values array",
                                ),
                            )?;
                        }
                    }
                    if decoded_index != decoded_values.len() {
                        return Err(serde::de::Error::custom(
                            "The values array isn't synchronized with the bits",
                        ));
                    }
                    Ok(Policies { bits, values, unknown_policies: HashMap::new(), raw_bits: bits.bits() })
                }
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut bits: Option<PoliciesBits> = None;
                let mut values = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Bits => {
                            if bits.is_some() {
                                return Err(serde::de::Error::duplicate_field("bits"));
                            }
                            bits = Some(map.next_value()?);
                        }
                        Field::Values => {
                            if values.is_some() {
                                return Err(serde::de::Error::duplicate_field("values"));
                            }
                            let Some(bits) = bits else {
                                return Err(serde::de::Error::custom(
                                    "bits field should be set before values",
                                ));
                            };
                            // For the `values` field, we always write the 4 elements for
                            // the first 4 policies and then
                            // write the value only if the policy is
                            // set.
                            // Previous behavior
                            if bits.intersection(PoliciesBits::all())
                                == bits.intersection(
                                    PoliciesBits::Maturity
                                        .union(PoliciesBits::MaxFee)
                                        .union(PoliciesBits::Tip)
                                        .union(PoliciesBits::WitnessLimit),
                                )
                            {
                                let decoded_values: [Word; 4] =
                                    map.next_value::<[Word; 4]>()?;
                                let mut tmp_values: [Word; POLICIES_NUMBER] =
                                    [0; POLICIES_NUMBER];
                                tmp_values[..4].copy_from_slice(&decoded_values);
                                values = Some(tmp_values);
                            // New backward compatible behavior
                            } else {
                                let decoded_values = map.next_value::<Vec<Word>>()?;
                                let mut tmp_values: [Word; POLICIES_NUMBER] =
                                    [0; POLICIES_NUMBER];
                                let mut decoded_index = 0;
                                for (index, bit) in PoliciesBits::all().iter().enumerate()
                                {
                                    if bits.contains(bit) {
                                        tmp_values[index] =
                                                *decoded_values
                                                    .get(decoded_index)
                                                    .ok_or(serde::de::Error::custom(
                                                    "The values array isn't synchronized with the bits",
                                                ))?;
                                        decoded_index = decoded_index
                                            .checked_add(1)
                                            .ok_or(serde::de::Error::custom(
                                                "Too many values in the values array",
                                            ))?;
                                    }
                                }
                                if decoded_index != decoded_values.len() {
                                    return Err(serde::de::Error::custom(
                                        "The values array isn't synchronized with the bits",
                                    ));
                                }
                                values = Some(tmp_values);
                            }
                        }
                        Field::Ignore => {
                            let _: serde::de::IgnoredAny = map.next_value()?;
                        }
                    }
                }
                let bits = bits.ok_or_else(|| serde::de::Error::missing_field("bits"))?;
                let values =
                    values.ok_or_else(|| serde::de::Error::missing_field("values"))?;
                Ok(Policies { bits, values, unknown_policies: HashMap::new(), raw_bits: bits.bits() })
            }
        }
        const FIELDS: &[&str] = &["bits", "values"];
        serde::Deserializer::deserialize_struct(
            deserializer,
            "Policies",
            FIELDS,
            StructVisitor {
                marker: PhantomData::<Policies>,
                lifetime: PhantomData,
            },
        )
    }
}

#[cfg(feature = "da-compression")]
impl fuel_compression::Compressible for Policies {
    type Compressed = Policies;
}

#[cfg(feature = "da-compression")]
impl<Ctx> fuel_compression::CompressibleBy<Ctx> for Policies
where
    Ctx: fuel_compression::ContextError,
{
    async fn compress_with(&self, _: &mut Ctx) -> Result<Self::Compressed, Ctx::Error> {
        Ok(self.clone())
    }
}

#[cfg(feature = "da-compression")]
impl<Ctx> fuel_compression::DecompressibleBy<Ctx> for Policies
where
    Ctx: fuel_compression::ContextError,
{
    async fn decompress_with(c: Self::Compressed, _: &Ctx) -> Result<Self, Ctx::Error> {
        Ok(c)
    }
}

impl Serialize for Policies {
    fn size_static(&self) -> usize {
        // u32 gets aligned to 8 bytes in canonical serialization
        self.raw_bits.size_static()
    }

    #[allow(clippy::arithmetic_side_effects)] // Bit count is not large enough to overflow.
    fn size_dynamic(&self) -> usize {
        // Count both known and unknown policies
        let total_policies = self.bits.bits().count_ones() as usize + self.unknown_policies.len();
        total_policies * Word::MIN.size()
    }

    fn encode_static<O: Output + ?Sized>(&self, buffer: &mut O) -> Result<(), Error> {
        // Serialize the raw bits including unknown policies
        self.raw_bits.encode_static(buffer)
    }

    fn encode_dynamic<O: Output + ?Sized>(&self, buffer: &mut O) -> Result<(), Error> {
        // Encode values in bit position order (0 to 31) to maintain serialization order
        for bit_position in 0..32u8 {
            let bit_flag = 1u32 << bit_position;
            
            // Check if this bit is set in raw_bits
            if self.raw_bits & bit_flag != 0 {
                // Check if this is a known policy
                if let Some(known_bit) = PoliciesBits::all().iter()
                    .find(|b| b.bits() == bit_flag)
                {
                    // Encode known policy value
                    let index = PoliciesBits::all().iter()
                        .enumerate()
                        .find(|(_, b)| *b == known_bit)
                        .map(|(i, _)| i)
                        .ok_or(Error::Unknown("Policy bit index not found"))?;
                    self.values[index].encode(buffer)?;
                } else if let Some(value) = self.unknown_policies.get(&bit_position) {
                    // Encode unknown policy value
                    value.encode(buffer)?;
                }
            }
        }
        Ok(())
    }
}

impl Deserialize for Policies {
    fn decode_static<I: Input + ?Sized>(buffer: &mut I) -> Result<Self, Error> {
        let bits_raw = u32::decode(buffer)?;
        // Use from_bits_truncate to ignore unknown bits instead of failing
        // This enables forward compatibility with future policy versions
        let bits = PoliciesBits::from_bits_truncate(bits_raw);
        Ok(Self {
            bits,
            values: Default::default(),
            unknown_policies: HashMap::new(),
            raw_bits: bits_raw,
        })
    }

    fn decode_dynamic<I: Input + ?Sized>(&mut self, buffer: &mut I) -> Result<(), Error> {
        // Decode values for both known and unknown policies in order
        // by iterating through bit positions from 0 to 31
        for bit_position in 0..32u8 {
            let bit_flag = 1u32 << bit_position;
            
            // Check if this bit is set in the raw bits
            if self.raw_bits & bit_flag != 0 {
                // Check if this is a known policy
                if let Some(known_bit) = PoliciesBits::all().iter()
                    .find(|b| b.bits() == bit_flag)
                {
                    // This is a known policy, decode into values array
                    let index = PoliciesBits::all().iter()
                        .enumerate()
                        .find(|(_, b)| *b == known_bit)
                        .map(|(i, _)| i)
                        .ok_or(Error::Unknown("Policy bit index not found"))?;
                    self.values[index] = Word::decode(buffer)?;
                } else {
                    // This is an unknown policy, decode and store in unknown_policies
                    let value = Word::decode(buffer)?;
                    self.unknown_policies.insert(bit_position, value);
                }
            }
        }

        if let Some(maturity) = self.get(PolicyType::Maturity)
            && maturity > u32::MAX as u64
        {
            return Err(Error::Unknown("The maturity in more than `u32::MAX`"));
        }

        if let Some(expiration) = self.get(PolicyType::Expiration)
            && expiration > u32::MAX as u64
        {
            return Err(Error::Unknown("The expiration in more than `u32::MAX`"));
        }

        Ok(())
    }
}

#[cfg(feature = "random")]
impl Distribution<Policies> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Policies {
        let bits: u32 = rng.r#gen();
        let bits = bits & PoliciesBits::all().bits();
        let bits = PoliciesBits::from_bits(bits).expect("We checked that above");
        let values = rng.r#gen();
        let mut policies = Policies {
            bits,
            values: Policies::values_for_bitmask(bits, values),
            unknown_policies: HashMap::new(),
            raw_bits: bits.bits(),
        };

        if policies.is_set(PolicyType::Maturity) {
            let maturity: u32 = rng.r#gen();
            policies.set(PolicyType::Maturity, Some(maturity as u64));
        }

        if policies.is_set(PolicyType::Expiration) {
            let expiration: u32 = rng.r#gen();
            policies.set(PolicyType::Expiration, Some(expiration as u64));
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
    const VALUES: [Word; POLICIES_NUMBER] = [
        0x1000001, 0x2000001, 0x3000001, 0x4000001, 0x5000001, 0x6000001,
    ];

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
    const VALUES: [Word; POLICIES_NUMBER] = [
        0x1000001, 0x2000001, 0x3000001, 0x4000001, 0x5000001, 0x6000001,
    ];

    for bitmask in 0..MAX_BITMASK {
        let bits =
            PoliciesBits::from_bits(bitmask).expect("Should construct a valid bits");
        let policies = Policies {
            bits,
            values: Policies::values_for_bitmask(bits, VALUES),
            unknown_policies: HashMap::new(),
            raw_bits: bitmask,
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

#[test]
fn serde_de_serialization_is_backward_compatible() {
    use serde_test::{
        Configure,
        Token,
        assert_tokens,
    };

    // Given
    let policies = Policies {
        bits: PoliciesBits::Maturity.union(PoliciesBits::MaxFee),
        values: [0, 0, 20, 10, 0, 0],
        unknown_policies: HashMap::new(),
        raw_bits: PoliciesBits::Maturity.union(PoliciesBits::MaxFee).bits(),
    };

    assert_tokens(
        // When
        &policies.clone().compact(),
        // Then
        &[
            Token::Struct {
                name: "Policies",
                len: 2,
            },
            Token::Str("bits"),
            Token::NewtypeStruct {
                name: "PoliciesBits",
            },
            Token::U32(12),
            Token::Str("values"),
            Token::Tuple { len: 4 },
            Token::U64(0),
            Token::U64(0),
            Token::U64(20),
            Token::U64(10),
            Token::TupleEnd,
            Token::StructEnd,
        ],
    );
}

#[test]
fn serde_deserialization_empty_use_backward_compatibility() {
    use serde_test::{
        Configure,
        Token,
        assert_tokens,
    };

    // Given
    let policies = Policies::new();

    assert_tokens(
        // When
        &policies.clone().compact(),
        // Then
        &[
            Token::Struct {
                name: "Policies",
                len: 2,
            },
            Token::Str("bits"),
            Token::NewtypeStruct {
                name: "PoliciesBits",
            },
            Token::U32(0),
            Token::Str("values"),
            Token::Tuple { len: 4 },
            Token::U64(0),
            Token::U64(0),
            Token::U64(0),
            Token::U64(0),
            Token::TupleEnd,
            Token::StructEnd,
        ],
    );
}

#[test]
fn serde_deserialization_new_format() {
    use serde_test::{
        Configure,
        Token,
        assert_tokens,
    };

    // Given
    let policies = Policies {
        bits: PoliciesBits::Maturity
            .union(PoliciesBits::Expiration)
            .union(PoliciesBits::Owner),
        values: [0, 0, 20, 0, 10, 3],
        unknown_policies: HashMap::new(),
        raw_bits: PoliciesBits::Maturity
            .union(PoliciesBits::Expiration)
            .union(PoliciesBits::Owner).bits(),
    };

    assert_tokens(
        &policies.clone().compact(),
        &[
            Token::Struct {
                name: "Policies",
                len: 2,
            },
            Token::Str("bits"),
            Token::NewtypeStruct {
                name: "PoliciesBits",
            },
            Token::U32(policies.bits.bits()),
            Token::Str("values"),
            Token::Seq { len: Some(3) },
            Token::U64(20),
            Token::U64(10),
            Token::U64(3),
            Token::SeqEnd,
            Token::StructEnd,
        ],
    );
}
#[test]
fn unknown_policies_are_preserved_during_deserialization() {
    use fuel_types::canonical::{Deserialize, Serialize};
    
    // Simulate a future version with bit 6 set (unknown to current version)
    let future_bits: u32 = PoliciesBits::Tip.bits() | PoliciesBits::MaxFee.bits() | (1 << 6);
    
    // Manually construct serialized data with known and unknown policies
    let mut buffer = Vec::new();
    
    // Encode bits
    future_bits.encode(&mut buffer).unwrap();
    
    // Encode values for Tip (bit 0), MaxFee (bit 3), and unknown policy (bit 6)
    let tip_value: u64 = 100;
    let max_fee_value: u64 = 200;
    let unknown_value: u64 = 999;
    
    tip_value.encode(&mut buffer).unwrap();
    max_fee_value.encode(&mut buffer).unwrap();
    unknown_value.encode(&mut buffer).unwrap();
    
    // Deserialize
    let policies = Policies::decode(&mut buffer.as_slice()).unwrap();
    
    // Verify known policies are decoded correctly
    assert_eq!(policies.get(PolicyType::Tip), Some(tip_value));
    assert_eq!(policies.get(PolicyType::MaxFee), Some(max_fee_value));
    
    // Verify unknown policy is stored
    assert_eq!(policies.unknown_policies.get(&6), Some(&unknown_value));
    
    // Verify raw bits include the unknown bit
    assert_eq!(policies.raw_bits, future_bits);
}

#[test]
fn unknown_policies_are_preserved_during_round_trip() {
    use fuel_types::canonical::{Deserialize, Serialize};
    
    // Simulate a future version with bits 6 and 7 set (unknown to current version)
    let future_bits: u32 = PoliciesBits::Maturity.bits() | (1 << 6) | (1 << 7);
    
    // Manually construct serialized data
    let mut buffer = Vec::new();
    future_bits.encode(&mut buffer).unwrap();
    
    let maturity_value: u64 = 42;
    let unknown_6_value: u64 = 888;
    let unknown_7_value: u64 = 777;
    
    maturity_value.encode(&mut buffer).unwrap();
    unknown_6_value.encode(&mut buffer).unwrap();
    unknown_7_value.encode(&mut buffer).unwrap();
    
    // Deserialize
    let policies = Policies::decode(&mut buffer.as_slice()).unwrap();
    
    // Re-serialize
    let mut reserialized = Vec::new();
    policies.encode(&mut reserialized).unwrap();
    
    // Deserialize again
    let policies2 = Policies::decode(&mut reserialized.as_slice()).unwrap();
    
    // Verify everything is preserved
    assert_eq!(policies2.get(PolicyType::Maturity), Some(maturity_value));
    assert_eq!(policies2.unknown_policies.get(&6), Some(&unknown_6_value));
    assert_eq!(policies2.unknown_policies.get(&7), Some(&unknown_7_value));
    assert_eq!(policies2.raw_bits, future_bits);
}

#[test]
fn deserialization_with_all_unknown_bits_succeeds() {
    use fuel_types::canonical::{Deserialize, Serialize};
    
    // All bits unknown (beyond current known policies)
    let unknown_bits: u32 = (1 << 10) | (1 << 15) | (1 << 20);
    
    let mut buffer = Vec::new();
    unknown_bits.encode(&mut buffer).unwrap();
    
    let val1: u64 = 111;
    let val2: u64 = 222;
    let val3: u64 = 333;
    
    val1.encode(&mut buffer).unwrap();
    val2.encode(&mut buffer).unwrap();
    val3.encode(&mut buffer).unwrap();
    
    // Should not fail
    let policies = Policies::decode(&mut buffer.as_slice()).unwrap();
    
    // All should be stored as unknown
    assert_eq!(policies.unknown_policies.get(&10), Some(&val1));
    assert_eq!(policies.unknown_policies.get(&15), Some(&val2));
    assert_eq!(policies.unknown_policies.get(&20), Some(&val3));
    assert_eq!(policies.raw_bits, unknown_bits);
}

#[test]
fn mixed_known_and_unknown_policies_maintain_serialization_order() {
    use fuel_types::canonical::{Deserialize, Serialize};
    
    // Mix of known and unknown policies: bits 0 (Tip), 3 (MaxFee), 6 (unknown), 7 (unknown)
    let mixed_bits: u32 = (1 << 0) | (1 << 3) | (1 << 6) | (1 << 7);
    
    let mut buffer = Vec::new();
    mixed_bits.encode(&mut buffer).unwrap();
    
    // Values must be in bit order
    let tip_val: u64 = 10;
    let max_fee_val: u64 = 30;
    let unknown_6_val: u64 = 60;
    let unknown_7_val: u64 = 70;
    
    tip_val.encode(&mut buffer).unwrap();
    max_fee_val.encode(&mut buffer).unwrap();
    unknown_6_val.encode(&mut buffer).unwrap();
    unknown_7_val.encode(&mut buffer).unwrap();
    
    let policies = Policies::decode(&mut buffer.as_slice()).unwrap();
    
    // Re-serialize and verify order is maintained
    let mut reserialized = Vec::new();
    policies.encode(&mut reserialized).unwrap();
    
    assert_eq!(buffer, reserialized, "Serialization order must be preserved");
}

#[test]
fn size_calculation_includes_unknown_policies() {
    use fuel_types::canonical::{Deserialize, Serialize};
    
    let mixed_bits: u32 = PoliciesBits::Tip.bits() | (1 << 10);
    
    let mut buffer = Vec::new();
    mixed_bits.encode(&mut buffer).unwrap();
    100u64.encode(&mut buffer).unwrap();
    200u64.encode(&mut buffer).unwrap();
    
    let policies = Policies::decode(&mut buffer.as_slice()).unwrap();
    
    // Size should account for both known and unknown policies
    // Note: u32 is aligned to 8 bytes in canonical serialization
    let expected_size = 8 + (2 * 8); // 8 bytes for aligned u32 bits, 2 * 8 bytes for two u64 values
    assert_eq!(policies.size(), expected_size);
}