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
        DeserializeForwardCompatible,
        Error,
        Input,
        Output,
        Serialize,
        SerializeForwardCompatible,
    },
};

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
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Hash)]
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
        } else {
            self.bits.remove(policy_type.bit());
            self.values[policy_type.index()] = 0;
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
                        ));
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
                                ));
                            }
                        };
                    let mut values: [Word; POLICIES_NUMBER] = [0; POLICIES_NUMBER];
                    values[..4].copy_from_slice(&decoded_values);
                    Ok(Policies { bits, values })
                // New backward compatible behavior
                } else {
                    let decoded_values = match seq.next_element::<Vec<Word>>()? {
                        Some(values) => values,
                        None => {
                            return Err(serde::de::Error::invalid_length(
                                1,
                                &"struct Policies with 2 elements",
                            ));
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
                    Ok(Policies { bits, values })
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
                Ok(Policies { bits, values })
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
        Ok(*self)
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
        self.bits.bits().size_static()
    }

    #[allow(clippy::arithmetic_side_effects)] // Bit count is not large enough to overflow.
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

impl SerializeForwardCompatible for Policies {
    type Metadata = PoliciesDeserializeMetadata;

    fn size_static_forward_compatible(&self, _metadata: &Self::Metadata) -> usize {
        self.bits.bits().size_static()
    }

    #[allow(clippy::arithmetic_side_effects)]
    fn size_dynamic_forward_compatible(&self, metadata: &Self::Metadata) -> usize {
        let known_count = self.bits.bits().count_ones() as usize;
        let unknown_count = metadata.unknown_bits.count_ones() as usize;
        let total_count = known_count + unknown_count;

        total_count * Word::MIN.size()
    }

    fn encode_static_forward_compatible<O: Output + ?Sized>(
        &self,
        buffer: &mut O,
        metadata: &Self::Metadata,
    ) -> Result<(), Error> {
        let raw_bits = self.bits.bits() | metadata.unknown_bits;
        raw_bits.encode_static(buffer)
    }

    fn encode_dynamic_forward_compatible<O: Output + ?Sized>(
        &self,
        buffer: &mut O,
        metadata: &Self::Metadata,
    ) -> Result<(), Error> {
        let raw_bits = self.bits.bits() | metadata.unknown_bits;

        for bit_position in 0u32..32u32 {
            let bit_mask = 1u32 << bit_position;
            if raw_bits & bit_mask != 0 {
                let value = if self.bits.bits() & bit_mask != 0 {
                    self.values[bit_position as usize]
                } else {
                    // Unknown policy - get from metadata stash
                    metadata
                        .unknown_policies
                        .iter()
                        .find(|(pos, _)| *pos == bit_position as usize)
                        .map(|(_, val)| *val)
                        .unwrap_or(0)
                };

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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PoliciesDeserializeMetadata {
    /// Which policy bits were unknown
    pub unknown_bits: u32,

    /// Whether any unknown policy was encountered
    pub has_unknown_policy: bool,

    pub unknown_policies: Vec<(usize, Word)>,
}

impl DeserializeForwardCompatible for Policies {
    type Metadata = PoliciesDeserializeMetadata;

    fn decode_static_forward_compatible<I: Input + ?Sized>(
        buffer: &mut I,
    ) -> Result<(Self, Self::Metadata), Error> {
        let raw_bits = u32::decode(buffer)?;
        let bits = PoliciesBits::from_bits_truncate(raw_bits);

        let metadata = PoliciesDeserializeMetadata {
            unknown_bits: raw_bits & !bits.bits(),
            has_unknown_policy: raw_bits & !bits.bits() != 0,
            unknown_policies: Vec::new(),
        };

        Ok((
            Self {
                bits,
                values: Default::default(),
            },
            metadata,
        ))
    }

    fn decode_dynamic_forward_compatible<I: Input + ?Sized>(
        &mut self,
        buffer: &mut I,
        metadata: &mut Self::Metadata,
    ) -> Result<(), Error> {
        for bit_position in 0u32..32u32 {
            let bit_mask = 1u32 << bit_position;

            let raw_bits = self.bits.bits() | metadata.unknown_bits;
            if raw_bits & bit_mask != 0 {
                let value = Word::decode(buffer)?;

                if self.bits.bits() & bit_mask != 0 {
                    self.values[bit_position as usize] = value;
                } else {
                    metadata
                        .unknown_policies
                        .push((bit_position as usize, value));
                }
            }
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
    };

    assert_tokens(
        // When
        &policies.compact(),
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
        &policies.compact(),
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
    };

    assert_tokens(
        &policies.compact(),
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
fn forward_compatible_deserialization_with_unknown_policies() {
    use fuel_types::canonical::{
        Deserialize,
        DeserializeForwardCompatible,
        Serialize,
    };

    // Simulate a future policy bit (bit 6 doesn't exist yet)
    let unknown_bit = 1u32 << 6;
    let known_bits = PoliciesBits::Tip.union(PoliciesBits::Maturity);
    let raw_bits = known_bits.bits() | unknown_bit;

    // Manually serialize policies with unknown bit
    // Values must be encoded in bit order: 0 (Tip), 2 (Maturity), 6 (unknown)
    let mut buffer = Vec::new();
    raw_bits.encode(&mut buffer).expect("Should encode bits");
    100u64
        .encode(&mut buffer)
        .expect("Should encode tip value (bit 0)");
    50u64
        .encode(&mut buffer)
        .expect("Should encode maturity value (bit 2)");
    999u64
        .encode(&mut buffer)
        .expect("Should encode unknown policy value (bit 6)");

    // Strict deserialization should fail
    let strict_result = Policies::from_bytes(&buffer);
    assert!(strict_result.is_err());
    assert!(matches!(
        strict_result.unwrap_err(),
        fuel_types::canonical::Error::Unknown(_)
    ));

    // Forward-compatible deserialization should succeed
    let (policies, metadata) = Policies::from_bytes_forward_compatible(&buffer)
        .expect("Forward-compatible deserialization should succeed");

    // Known policies should be accessible
    assert_eq!(policies.get(PolicyType::Tip), Some(100));
    assert_eq!(policies.get(PolicyType::Maturity), Some(50));

    // Metadata should report unknown policy
    assert!(metadata.has_unknown_policy);
    assert_eq!(metadata.unknown_bits, unknown_bit);

    // Policies should only have known bits set
    assert_eq!(policies.bits(), known_bits.bits());
}

#[test]
fn forward_compatible_deserialization_without_unknown_policies() {
    use fuel_types::canonical::DeserializeForwardCompatible;

    // Given - Only known policies
    let policies = Policies::new()
        .with_tip(100)
        .with_witness_limit(200)
        .with_maturity(BlockHeight::new(10));

    let bytes = policies.to_bytes();

    // When - Forward-compatible deserialization should succeed
    let (deserialized, metadata) = Policies::from_bytes_forward_compatible(&bytes)
        .expect("Forward-compatible deserialization should succeed");

    // Then - Should not have unknown policies
    assert!(!metadata.has_unknown_policy);
    assert_eq!(metadata.unknown_bits, 0);

    // Policies should match
    assert_eq!(deserialized, policies);
}

#[test]
fn forward_compatible_deserialization_with_multiple_unknown_policies() {
    use fuel_types::canonical::{
        Deserialize,
        DeserializeForwardCompatible,
        Serialize,
    };

    // Given - Simulate multiple unknown policy bits
    let unknown_bit1 = 1u32 << 6;
    let unknown_bit2 = 1u32 << 7;
    let unknown_bit3 = 1u32 << 10;
    let known_bits = PoliciesBits::MaxFee.union(PoliciesBits::Expiration);
    let raw_bits = known_bits.bits() | unknown_bit1 | unknown_bit2 | unknown_bit3;

    // When - Manually serialize
    // Values must be in bit order: 3 (MaxFee), 4 (Expiration), 6, 7, 10
    let mut buffer = Vec::new();
    raw_bits.encode(&mut buffer).expect("Should encode bits");
    500u64
        .encode(&mut buffer)
        .expect("Should encode max fee (bit 3)");
    12345u64
        .encode(&mut buffer)
        .expect("Should encode expiration (bit 4)");
    1111u64
        .encode(&mut buffer)
        .expect("Should encode unknown bit 6");
    2222u64
        .encode(&mut buffer)
        .expect("Should encode unknown bit 7");
    3333u64
        .encode(&mut buffer)
        .expect("Should encode unknown bit 10");

    // Then - Strict should fail
    assert!(Policies::from_bytes(&buffer).is_err());

    // Forward compatibility should succeed
    let (policies, metadata) =
        Policies::from_bytes_forward_compatible(&buffer).expect("should succeed");

    // All unknown bits should be tracked
    assert!(metadata.has_unknown_policy);
    assert_eq!(
        metadata.unknown_bits,
        unknown_bit1 | unknown_bit2 | unknown_bit3
    );

    // Known policies should be accessible
    assert_eq!(policies.get(PolicyType::MaxFee), Some(500));
    assert_eq!(policies.get(PolicyType::Expiration), Some(12345));
}

#[test]
fn forward_compatible_deserialization_preserves_known_policies_only() {
    use fuel_types::canonical::{
        DeserializeForwardCompatible,
        Serialize,
    };

    // Given - Mix of all known policies plus unknown bits
    let unknown_bits = 0b1111_0000_0000u32; // Bits 8, 9, 10, 11 are unknown
    let all_known = PoliciesBits::all().bits();
    let raw_bits = all_known | unknown_bits;

    let mut buffer = Vec::new();
    raw_bits.encode(&mut buffer).expect("Should encode bits");

    // When - Encode values in bit order: 0, 1, 2, 3, 4, 5, 8, 9, 10, 11
    100u64.encode(&mut buffer).expect("Tip (bit 0)");
    200u64.encode(&mut buffer).expect("WitnessLimit (bit 1)");
    10u64.encode(&mut buffer).expect("Maturity (bit 2)");
    500u64.encode(&mut buffer).expect("MaxFee (bit 3)");
    12345u64.encode(&mut buffer).expect("Expiration (bit 4)");
    0xABCDu64.encode(&mut buffer).expect("Owner (bit 5)");
    1000u64.encode(&mut buffer).expect("Unknown bit 8");
    2000u64.encode(&mut buffer).expect("Unknown bit 9");
    3000u64.encode(&mut buffer).expect("Unknown bit 10");
    4000u64.encode(&mut buffer).expect("Unknown bit 11");

    let (policies, metadata) =
        Policies::from_bytes_forward_compatible(&buffer).expect("Should deserialize");

    // Then - All known policies should be present
    assert_eq!(policies.get(PolicyType::Tip), Some(100));
    assert_eq!(policies.get(PolicyType::WitnessLimit), Some(200));
    assert_eq!(policies.get(PolicyType::Maturity), Some(10));
    assert_eq!(policies.get(PolicyType::MaxFee), Some(500));
    assert_eq!(policies.get(PolicyType::Expiration), Some(12345));
    assert_eq!(policies.get(PolicyType::Owner), Some(0xABCD));

    // Unknown bits should be tracked
    assert!(metadata.has_unknown_policy);
    assert_eq!(metadata.unknown_bits, unknown_bits);
}

#[test]
fn forward_compatible_deserialization_with_all_policies_unknown() {
    use fuel_types::canonical::{
        DeserializeForwardCompatible,
        Serialize,
    };

    let future_bits = 0b1111_1111_1100_0000u32; // Bits 6-15 are set

    let mut buffer = Vec::new();
    future_bits.encode(&mut buffer).unwrap();

    // Encode values for all the unknown bits (6-15)
    for i in 6u32..16u32 {
        if future_bits & (1u32 << i) != 0 {
            (u64::from(i) * 100).encode(&mut buffer).unwrap();
        }
    }

    let (policies, metadata) = Policies::from_bytes_forward_compatible(&buffer)
        .expect("Should deserialize even with all unknown");

    // Should have empty known policies
    assert!(policies.is_empty());
    assert_eq!(policies.bits(), 0);

    // But should track all as unknown
    assert!(metadata.has_unknown_policy);
    assert_eq!(metadata.unknown_bits, future_bits);
}

#[test]
fn forward_compatible_deserialization_empty_policies() {
    use fuel_types::canonical::DeserializeForwardCompatible;

    let empty_policies = Policies::new();
    let bytes = empty_policies.to_bytes();

    let (deserialized, metadata) = Policies::from_bytes_forward_compatible(&bytes)
        .expect("Should deserialize empty policies");

    assert_eq!(deserialized, empty_policies);
    assert!(deserialized.is_empty());
    assert!(!metadata.has_unknown_policy);
    assert_eq!(metadata.unknown_bits, 0);
}

#[test]
fn test_roundtrip_with_unknown_policies_preserves_bytes() {
    use fuel_types::canonical::{
        DeserializeForwardCompatible,
        Serialize,
    };

    // Given - A transaction from the future with unknown policy bits 6 and 7
    let unknown_bit_6 = 1u32 << 6;
    let unknown_bit_7 = 1u32 << 7;
    let known_bits = PoliciesBits::Tip.union(PoliciesBits::MaxFee);
    let raw_bits = known_bits.bits() | unknown_bit_6 | unknown_bit_7;

    let mut original_bytes = Vec::new();
    raw_bits.encode(&mut original_bytes).unwrap(); // u32 = 4 bytes + 4 padding = 8 bytes
    100u64.encode(&mut original_bytes).unwrap(); // Tip value (bit 0)
    500u64.encode(&mut original_bytes).unwrap(); // MaxFee value (bit 3)
    1000u64.encode(&mut original_bytes).unwrap(); // Unknown policy bit 6 value
    2000u64.encode(&mut original_bytes).unwrap(); // Unknown policy bit 7 value

    // When - Deserialize with forward compatibility
    let (policies, metadata) = Policies::from_bytes_forward_compatible(&original_bytes)
        .expect("Should deserialize");

    // Then - Should have captured unknown policy values
    assert!(metadata.has_unknown_policy);
    assert_eq!(metadata.unknown_bits, unknown_bit_6 | unknown_bit_7);
    assert_eq!(metadata.unknown_policies.len(), 2);
    assert_eq!(metadata.unknown_policies[0], (6, 1000));
    assert_eq!(metadata.unknown_policies[1], (7, 2000));

    // When - Re-serialize
    let reserialized_bytes = policies.to_bytes_forward_compatible(&metadata);

    // Then - Should produce identical bytes
    assert_eq!(original_bytes, reserialized_bytes);
}

#[test]
fn test_roundtrip_without_unknown_policies_stays_identical() {
    use fuel_types::canonical::DeserializeForwardCompatible;

    // Given - Normal policies without unknown bits
    let policies = Policies::new()
        .with_tip(100)
        .with_maturity(BlockHeight::new(50));

    let original_bytes = policies.to_bytes();

    // When - Roundtrip through forward-compatible deserialization
    let (deserialized, metadata) =
        Policies::from_bytes_forward_compatible(&original_bytes).unwrap();

    // Then - Should not have unknown policies
    assert!(!metadata.has_unknown_policy);
    assert_eq!(metadata.unknown_policies.len(), 0);

    // When - Re-serialize
    let reserialized_bytes = deserialized.to_bytes_forward_compatible(&metadata);

    // Then - Should be identical
    assert_eq!(original_bytes, reserialized_bytes);
}

#[test]
fn test_unknown_policies_preserved_in_correct_bit_order() {
    use fuel_types::canonical::{
        DeserializeForwardCompatible,
        Serialize,
    };

    // Given - Mix of known and unknown policies in non-sequential bit positions
    // Known: bit 0 (Tip), bit 3 (MaxFee)
    // Unknown: bit 6, bit 10, bit 15
    let known_bits = PoliciesBits::Tip.union(PoliciesBits::MaxFee);
    let unknown_bits = (1u32 << 6) | (1u32 << 10) | (1u32 << 15);
    let raw_bits = known_bits.bits() | unknown_bits;

    let mut original_bytes = Vec::new();
    raw_bits.encode(&mut original_bytes).unwrap();

    // Values must be encoded in bit order: 0, 3, 6, 10, 15
    100u64.encode(&mut original_bytes).unwrap(); // bit 0 (Tip)
    500u64.encode(&mut original_bytes).unwrap(); // bit 3 (MaxFee)
    1111u64.encode(&mut original_bytes).unwrap(); // bit 6 (unknown)
    2222u64.encode(&mut original_bytes).unwrap(); // bit 10 (unknown)
    3333u64.encode(&mut original_bytes).unwrap(); // bit 15 (unknown)

    // When - Deserialize
    let (policies, metadata) = Policies::from_bytes_forward_compatible(&original_bytes)
        .expect("Should deserialize");

    // Then - Unknown policies should be in correct order
    assert_eq!(metadata.unknown_policies.len(), 3);
    assert_eq!(metadata.unknown_policies[0], (6, 1111));
    assert_eq!(metadata.unknown_policies[1], (10, 2222));
    assert_eq!(metadata.unknown_policies[2], (15, 3333));

    // Known policies should be accessible
    assert_eq!(policies.get(PolicyType::Tip), Some(100));
    assert_eq!(policies.get(PolicyType::MaxFee), Some(500));

    // When - Re-serialize
    let reserialized_bytes = policies.to_bytes_forward_compatible(&metadata);

    // Then - Must match original bytes exactly (critical for signing!)
    assert_eq!(original_bytes, reserialized_bytes);
}

#[test]
fn test_transaction_signing_with_unknown_policies() {
    use fuel_types::canonical::DeserializeForwardCompatible;

    // Given - Simulate a signed transaction with unknown policies
    let known_bits = PoliciesBits::Tip.union(PoliciesBits::WitnessLimit);
    let unknown_bits = (1u32 << 8) | (1u32 << 9);
    let raw_bits = known_bits.bits() | unknown_bits;

    let mut tx_bytes = Vec::new();
    raw_bits.encode(&mut tx_bytes).unwrap();
    150u64.encode(&mut tx_bytes).unwrap(); // Tip
    1000u64.encode(&mut tx_bytes).unwrap(); // WitnessLimit
    777u64.encode(&mut tx_bytes).unwrap(); // Unknown bit 8
    888u64.encode(&mut tx_bytes).unwrap(); // Unknown bit 9

    // When - Deserialize, potentially modify known fields, then re-serialize
    let (mut policies, metadata) =
        Policies::from_bytes_forward_compatible(&tx_bytes).unwrap();

    // Modify a known field (simulating transaction modification)
    policies.set(PolicyType::Tip, Some(200)); // Changed from 150 to 200

    // When - Re-serialize with metadata
    let new_tx_bytes = policies.to_bytes_forward_compatible(&metadata);

    // Then - Unknown policies should be preserved even though we modified a known field
    // Re-decode to verify
    let (revalidated, revalidated_meta) =
        Policies::from_bytes_forward_compatible(&new_tx_bytes).unwrap();

    assert_eq!(revalidated.get(PolicyType::Tip), Some(200)); // Modified value
    assert_eq!(revalidated.get(PolicyType::WitnessLimit), Some(1000)); // Unchanged
    assert_eq!(revalidated_meta.unknown_policies.len(), 2);
    assert_eq!(revalidated_meta.unknown_policies[0], (8, 777));
    assert_eq!(revalidated_meta.unknown_policies[1], (9, 888));
}
