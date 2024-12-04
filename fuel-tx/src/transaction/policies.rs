use core::{
    marker::PhantomData,
    ops::Deref,
};
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
}

impl PolicyType {
    pub const fn index(&self) -> usize {
        match self {
            PolicyType::Tip => 0,
            PolicyType::WitnessLimit => 1,
            PolicyType::Maturity => 2,
            PolicyType::MaxFee => 3,
            PolicyType::Expiration => 4,
        }
    }

    pub const fn bit(&self) -> PoliciesBits {
        match self {
            PolicyType::Tip => PoliciesBits::Tip,
            PolicyType::WitnessLimit => PoliciesBits::WitnessLimit,
            PolicyType::Maturity => PoliciesBits::Maturity,
            PolicyType::MaxFee => PoliciesBits::MaxFee,
            PolicyType::Expiration => PoliciesBits::Expiration,
        }
    }
}

/// The total number of policies.
pub const POLICIES_NUMBER: usize = PoliciesBits::all().bits().count_ones() as usize;

/// Container for managing policies.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "da-compression",
    derive(fuel_compression::Compress, fuel_compression::Decompress)
)]
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

        if let Some(maturity) = self.get(PolicyType::Maturity) {
            if maturity > u32::MAX as u64 {
                return false;
            }
        }

        if let Some(expiration) = self.get(PolicyType::Expiration) {
            if expiration > u32::MAX as u64 {
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
// 1. Always write the 4 elements for the first 4 policies even if they are not set for
//    backward compatibility.
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
            let first_four_values: [Word; 4] =
                self.values[..4].try_into().map_err(|_| {
                    serde::ser::Error::custom("The first 4 values should be present")
                })?;
            let mut values = vec![];
            for (value, bit) in self.values.iter().zip(PoliciesBits::all().iter()).skip(4)
            {
                if self.bits.contains(bit) {
                    values.push(*value);
                }
            }
            state.serialize_field("values", &(first_four_values, values))?;
        }
        state.end()
    }
}

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
        impl<'de> serde::de::Visitor<'de> for FieldVisitor {
            type Value = Field;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("field identifier")
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    0 => Ok(Field::Bits),
                    1 => Ok(Field::Values),
                    _ => Ok(Field::Ignore),
                }
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

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
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
                    Ok(Policies { bits, values })
                // New backward compatible behavior
                } else {
                    let mut decoded_values =
                        match seq.next_element::<([Word; 4], Vec<Word>)>()? {
                            Some(values) => values,
                            None => {
                                return Err(serde::de::Error::invalid_length(
                                    1,
                                    &"struct Policies with 2 elements",
                                ))
                            }
                        };
                    let mut values: [Word; POLICIES_NUMBER] = [0; POLICIES_NUMBER];
                    values[..4].copy_from_slice(&decoded_values.0);
                    for (index, bit) in PoliciesBits::all().iter().enumerate().skip(4) {
                        if bits.contains(bit) {
                            if let Some(value) = decoded_values.1.pop() {
                                values[index] = value;
                            }
                        }
                    }
                    if !decoded_values.1.is_empty() {
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
                                let mut decoded_values =
                                    map.next_value::<([Word; 4], Vec<Word>)>()?;
                                let mut tmp_values: [Word; POLICIES_NUMBER] =
                                    [0; POLICIES_NUMBER];
                                tmp_values[..4].copy_from_slice(&decoded_values.0);
                                for (index, bit) in
                                    PoliciesBits::all().iter().enumerate().skip(4)
                                {
                                    if bits.contains(bit) {
                                        if let Some(value) = decoded_values.1.pop() {
                                            tmp_values[index] = value;
                                        }
                                    }
                                }
                                if !decoded_values.1.is_empty() {
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
                return Err(Error::Unknown("The maturity in more than `u32::MAX`"));
            }
        }

        if let Some(expiration) = self.get(PolicyType::Expiration) {
            if expiration > u32::MAX as u64 {
                return Err(Error::Unknown("The expiration in more than `u32::MAX`"));
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

        if policies.is_set(PolicyType::Maturity) {
            let maturity: u32 = rng.gen();
            policies.set(PolicyType::Maturity, Some(maturity as u64));
        }

        if policies.is_set(PolicyType::Expiration) {
            let expiration: u32 = rng.gen();
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
    const VALUES: [Word; POLICIES_NUMBER] =
        [0x1000001, 0x2000001, 0x3000001, 0x4000001, 0x5000001];

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
    const VALUES: [Word; POLICIES_NUMBER] =
        [0x1000001, 0x2000001, 0x3000001, 0x4000001, 0x5000001];

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
        assert_tokens,
        Configure,
        Token,
    };

    // Given
    let policies = Policies {
        bits: PoliciesBits::Maturity.union(PoliciesBits::MaxFee),
        values: [0, 0, 20, 10, 0],
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
fn serde_deserialization_new_format() {
    use serde_test::{
        assert_tokens,
        Token,
        Configure
    };

    // Given
    let policies = Policies {
        bits: PoliciesBits::Maturity.union(PoliciesBits::Expiration),
        values: [0, 0, 20, 0, 10],
    };

    assert_tokens(&policies.compact(), &[
        Token::Struct {
            name: "Policies",
            len: 2,
        },
        Token::Str("bits"),
        Token::NewtypeStruct {
            name: "PoliciesBits",
        },
        Token::U32(20),
        Token::Str("values"),
        Token::Tuple { len: 2 },
        Token::Tuple { len: 4 },
        Token::U64(0),
        Token::U64(0),
        Token::U64(20),
        Token::U64(0),
        Token::TupleEnd,
        Token::Seq { len: Some(1) },
        Token::U64(10),
        Token::SeqEnd,
        Token::TupleEnd,
        Token::StructEnd,
    ]);
}