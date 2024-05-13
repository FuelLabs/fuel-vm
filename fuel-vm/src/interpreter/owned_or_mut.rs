/// Owned or mutable reference.
#[derive(Debug)]
pub enum OwnedOrMut<'a, T> {
    /// Owned value.
    Owned(T),
    /// Mutable reference.
    Borrowed(&'a mut T),
}

#[cfg(any(test, featutre = "test-helpers"))]
impl<T: Clone> Clone for OwnedOrMut<'_, T> {
    fn clone(&self) -> Self {
        OwnedOrMut::Owned(match self {
            OwnedOrMut::Owned(m) => m.clone(),
            OwnedOrMut::Borrowed(m) => (**m).clone(),
        })
    }
}

impl<T> From<T> for OwnedOrMut<'_, T> {
    fn from(t: T) -> Self {
        OwnedOrMut::Owned(t)
    }
}
impl<T: Default> Default for OwnedOrMut<'_, T> {
    fn default() -> Self {
        OwnedOrMut::Owned(T::default())
    }
}
impl<T> AsRef<T> for OwnedOrMut<'_, T> {
    fn as_ref(&self) -> &T {
        match self {
            OwnedOrMut::Owned(m) => m,
            OwnedOrMut::Borrowed(m) => m,
        }
    }
}

impl<T> AsMut<T> for OwnedOrMut<'_, T> {
    fn as_mut(&mut self) -> &mut T {
        match self {
            OwnedOrMut::Owned(m) => m,
            OwnedOrMut::Borrowed(m) => m,
        }
    }
}
