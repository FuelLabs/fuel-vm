use generic_array::{ArrayLength, GenericArray};

pub trait Digest {
    type OutputSize: ArrayLength<u8>;

    fn new() -> Self;
    fn update(&mut self, input: impl AsRef<[u8]>);
    fn finalize(self) -> GenericArray<u8, <Self as Digest>::OutputSize>;
}
