use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;

/// Utility to wrap non-hashable types with their string impl
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Hashed<T: Clone + Debug>(pub T);

impl<T: Clone + Copy + Debug> Copy for Hashed<T> {}

impl<T: Clone + Debug> Display for Hashed<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<T: Clone + Debug + ToString> Hash for Hashed<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.to_string().hash(state);
    }
}

impl<T: Clone + Debug + FromStr> FromStr for Hashed<T> {
    type Err = T::Err;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        T::from_str(s).map(Hashed)
    }
}

impl<T: Clone + Debug> AsRef<T> for Hashed<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl<T: Clone + Debug> AsMut<T> for Hashed<T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T: Clone + Copy + Debug> Deref for Hashed<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Clone + Copy + Debug> DerefMut for Hashed<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}
