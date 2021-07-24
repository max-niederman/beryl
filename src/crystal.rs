use crate::BerylError;
use std::convert::TryInto;
use std::fmt;

/// Enumeration of Crystal parts
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CrystalPart {
    GeneratorId,
    Counter,
    Timestamp,
}

/// Wrapper struct over a [`u64`] which provides functions to destructure a Crystal
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Crystal(u64);

impl Crystal {
    /// Create a Crystal from its raw parts
    ///
    /// # Example
    /// ```
    /// # use beryl::Crystal;
    /// #
    /// let c = Crystal::from_parts(42, 42, 42).unwrap();
    /// println!("{:?}", c);
    /// ```
    #[inline]
    pub fn from_parts(generator: u16, counter: u16, timestamp: u64) -> Result<Self, BerylError> {
        Ok(Self::from_parts_unchecked(
            (generator <= 0x3FFF)
                .then(|| generator)
                .ok_or(BerylError::PartOutOfBounds(CrystalPart::GeneratorId))?,
            (counter <= 0xFF)
                .then(|| counter)
                .ok_or(BerylError::PartOutOfBounds(CrystalPart::Counter))?,
            (timestamp <= 0x3FFFFFFFFFF)
                .then(|| timestamp)
                .ok_or(BerylError::PartOutOfBounds(CrystalPart::Timestamp))?,
        ))
    }

    /// Like [`Crystal::from_parts`], but doesn't ensure each part is correctly sized
    #[inline]
    pub fn from_parts_unchecked(generator: u16, counter: u16, timestamp: u64) -> Self {
        Self {
            0: ((generator as u64) << 50) | ((counter as u64) << 42) | timestamp,
        }
    }

    /// Returns the ID of the Crystal's generator
    #[inline]
    pub fn generator(&self) -> u16 {
        (self.0 >> 50).try_into().unwrap()
    }

    /// Returns the Crystal's counter
    #[inline]
    pub fn counter(&self) -> u16 {
        ((self.0 & 0x3FC0000000000) >> 42).try_into().unwrap()
    }

    /// Returns the timestamp of the Crystal's creation
    #[inline]
    pub fn timestamp(&self) -> u64 {
        self.0 & 0x3FFFFFFFFFF
    }
}

impl fmt::Debug for Crystal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Crystal")
            .field("generator", &self.generator())
            .field("counter", &self.counter())
            .field("timestamp", &self.timestamp())
            .finish()
    }
}

impl From<u64> for Crystal {
    #[inline]
    fn from(bits: u64) -> Self {
        Self { 0: bits }
    }
}

impl From<Crystal> for u64 {
    #[inline]
    fn from(cry: Crystal) -> Self {
        cry.0
    }
}

/// Because some serialization formats do not support unsigned integers, we provide conversion to
/// and from [`i64`]. This is achieved using [`std::mem::transmute`], but is safe because we're
/// using [`u64`] to represent raw bits anyway.
impl From<i64> for Crystal {
    #[inline]
    fn from(raw: i64) -> Self {
        Self {
            0: unsafe { std::mem::transmute(raw) },
        }
    }
}

impl From<Crystal> for i64 {
    #[inline]
    fn from(cry: Crystal) -> Self {
        unsafe { std::mem::transmute(cry.0) }
    }
}

#[cfg(test)]
mod tests {
    use super::{Crystal, CrystalPart};
    use crate::BerylError;

    #[test]
    fn converts_i64() {
        assert_eq!(i64::from(Crystal::from_parts(0, 0, 0).unwrap()), 0i64);
        assert_eq!(
            i64::from(Crystal::from_parts(0x3FFF, 0xFF, 0x3FFFFFFFFFF).unwrap()),
            -1i64
        );

        // Conversion should be transitive by bit representation
        assert_eq!(Crystal::from(-1i64), Crystal::from(u64::MAX));
        assert_eq!(Crystal::from(0i64), Crystal::from(0u64));
    }

    #[test]
    fn to_parts_inverts_from_parts() {
        let c = Crystal::from_parts(42, 42, 42).unwrap();
        assert_eq!(c.generator(), 42);
        assert_eq!(c.counter(), 42);
        assert_eq!(c.timestamp(), 42);
    }

    #[test]
    fn from_parts_checks_bounds() {
        // Should be out of bounds
        Crystal::from_parts(u16::MAX, u16::MAX, u64::MAX).unwrap_err();
        assert_eq!(
            Crystal::from_parts(0x4000, 0, 0),
            Err(BerylError::PartOutOfBounds(CrystalPart::GeneratorId))
        );
        assert_eq!(
            Crystal::from_parts(0, 0x100, 0),
            Err(BerylError::PartOutOfBounds(CrystalPart::Counter))
        );
        assert_eq!(
            Crystal::from_parts(0, 0, 0x40000000000),
            Err(BerylError::PartOutOfBounds(CrystalPart::Timestamp))
        );

        // Should not be out of bounds
        assert_eq!(Crystal::from_parts(0, 0, 0), Ok(Crystal { 0: u64::MIN }));
        assert_eq!(
            Crystal::from_parts(0x3FFF, 0xFF, 0x3FFFFFFFFFF),
            Ok(Crystal { 0: u64::MAX })
        );
    }
}
