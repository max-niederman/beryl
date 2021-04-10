//! # Beryl
//! Beryl is a format for unique identifiers. This crate is the reference implementation of that
//! format.
//!
//! ## Crystals
//! Beryl identifiers, or [`Crystal`]s, are encoded into 64 bits as follows:
//! - **Generator ID**: 12-bit unsigned integer identifying the Crystal's generator. Further segmentation is
//! left to the application, as conflicts will not occur unless the scheme is changed unevenly over
//! less than a millisecond.
//! - **Generator Counter**: 10-bit unsigned integer incremented for every Crystal generated and
//! reset each millisecond.
//! - **Timestamp**: 42-bit unsigned integer number of milliseconds since an application-defined
//! epoch.
//!
//! ## Epochs
//! Beryl defines no standard epoch which a timestamp should be measured from, as the limited
//! timestamp size (2<sup>42</sup> milliseconds is about 140 years) may call for non-standard epochs. For
//! ease of use, the UNIX Epoch should be best.

use std::convert::TryInto;
use std::fmt;
use std::time::{Duration, SystemTime};

/// Enumeration of Crystal parts
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CrystalPart {
    GeneratorID,
    Counter,
    Timestamp,
}

/// Enumeration of possible Beryl errors
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BerylError {
    PartOutOfBounds(CrystalPart),
    GeneratorExhausted,
}

impl fmt::Display for BerylError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BerylError::PartOutOfBounds(part) => {
                writeln!(f, "Crystal part was out of bounds: {:?}", part)
            },
            BerylError::GeneratorExhausted => {
                writeln!(f, "Generator ran out of Crystals for this millisecond")
            }
        }
    }
}

impl From<BerylError> for std::io::Error {
    fn from(be: BerylError) -> Self {
        Self::new(std::io::ErrorKind::Other, format!("{}", be))
    }
}

/// Wrapper struct over a [`u64`] which provides functions to destructure a Crystal
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Crystal {
    crystal: u64,
}

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
    pub fn from_parts(generator: u16, counter: u16, timestamp: u64) -> Result<Self, BerylError> {
        Ok(Self::from_parts_unchecked(
            (generator <= 0xFFF)
                .then(|| generator)
                .ok_or(BerylError::PartOutOfBounds(CrystalPart::GeneratorID))?,
            (counter <= 0x3FF)
                .then(|| counter)
                .ok_or(BerylError::PartOutOfBounds(CrystalPart::Counter))?,
            (timestamp <= 0x3FFFFFFFFFF)
                .then(|| timestamp)
                .ok_or(BerylError::PartOutOfBounds(CrystalPart::Timestamp))?,
        ))
    }

    /// Like [`Crystal::from_parts`], but doesn't ensure each part is correctly sized
    pub fn from_parts_unchecked(generator: u16, counter: u16, timestamp: u64) -> Self {
        Self {
            crystal: ((generator as u64) << 52) | ((counter as u64) << 42) | timestamp,
        }
    }

    /// Returns the ID of the Crystal's generator
    pub fn generator(&self) -> u16 {
        (self.crystal >> 52).try_into().unwrap()
    }

    /// Returns the Crystal's counter
    pub fn counter(&self) -> u16 {
        ((self.crystal & 0xFFC0000000000) >> 42).try_into().unwrap()
    }

    /// Returns the timestamp of the Crystal's creation
    pub fn timestamp(&self) -> u64 {
        self.crystal & 0x3FFFFFFFFFF
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
    fn from(raw: u64) -> Self {
        Self { crystal: raw }
    }
}

impl From<Crystal> for u64 {
    fn from(cry: Crystal) -> Self {
        cry.crystal
    }
}

/// Because some serialization formats do not support unsigned integers, we provide conversion to
/// and from [`i64`]. This is achieved using [`std::mem::transmute`], but is safe because we're
/// using [`u64`] to represent raw bits anyway.
impl From<i64> for Crystal {
    fn from(raw: i64) -> Self {
        Self {
            crystal: unsafe { std::mem::transmute(raw) }
        }
    }
}

impl From<Crystal> for i64 {
    fn from(cry: Crystal) -> Self {
        unsafe { std::mem::transmute(cry.crystal) }
    }
}

/// Generates Crystals
#[derive(PartialEq, Eq, Debug)]
pub struct Generator {
    /// The unique ID of this generator within the Beryl scope
    pub id: u16,
    /// The epoch which timestamps are measured from
    pub epoch: SystemTime,

    count: u16,
    last_timestamp: u64,
}

impl Generator {
    /// Construct a new generator with the given ID and Epoch
    pub fn new(id: u16, epoch: SystemTime) -> Result<Self, BerylError> {
        Ok(Self {
            id: (id < 0x1000)
                .then(|| id)
                .ok_or(BerylError::PartOutOfBounds(CrystalPart::GeneratorID))?,
            epoch,
            count: u16::MAX,
            last_timestamp: now(epoch),
        })
    }

    /// Generate a [`Crystal`] using the recommended blocking method, spinning until the
    /// millisecond is over. This may not be the best method for your use case. You should run your
    /// own benchmarks if [`Generator`] speed is a consideration.
    ///
    /// # Example
    /// ```
    /// # use beryl::Generator;
    /// use std::time::SystemTime;
    /// 
    /// # fn main() -> std::io::Result<()> {
    /// let mut gen = Generator::new(0, SystemTime::UNIX_EPOCH)?;
    /// 
    /// // Generate a few snowflakes
    /// for _ in 0..2 {
    ///     println!("{:?}", gen.generate());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn generate(&mut self) -> Crystal {
        self.generate_block_spin()
    }

    /// Try to generate a [`Crystal`], returning [`BerylError::GeneratorExhausted`] if the
    /// [`Generator`] has exhausted the set of Crystals which can be generated in that millisecond.
    pub fn try_generate(&mut self) -> Result<Crystal, BerylError> {
        if self.count == 0x3FF && self.last_timestamp == now(self.epoch) {
            Err(BerylError::GeneratorExhausted)
        } else {
            Ok(self.generate_unchecked())
        }
    }

    /// Generate a [`Crystal`], checking the time every 100ns when the counter is saturated. This
    /// wastes a little time if the system has a high-precision `sleep` call, and a lot if it
    /// doesn't. You should do your own testing if [`Generator`] speed is a consideration.
    pub fn generate_block_sleep(&mut self) -> Crystal {
        while self.count == 0x3FF && self.last_timestamp == now(self.epoch) {
            std::thread::sleep(Duration::from_nanos(100))
        }

        self.generate_unchecked()
    }

    /// Generate a [`Crystal`], checking the time constantly until the next millisecond when the
    /// counter is saturated. This generates many syscalls; however, for use cases where the entire
    /// program must block on the generator or on systems which lack high-precision `sleep` calls,
    /// it may be the best option. You should do your own testing if [`Generator`] speed is a
    /// consideration.
    pub fn generate_block_spin(&mut self) -> Crystal {
        while self.count == 0x3FF && self.last_timestamp == now(self.epoch) {}

        self.generate_unchecked()
    }

    /// Generate a [`Crystal`] without checking to make sure that [`Crystal`] hasn't been generated
    /// before.
    ///
    /// # Warning
    /// Do not use this unless you know what you are doing. If you aren't absolutely sure you'll
    /// produce less than 1024 Crystals/ms, this will create conflicting Crystals.
    pub fn generate_unchecked(&mut self) -> Crystal {
        if self.last_timestamp == now(self.epoch) {
            self.count = self.count.wrapping_add(1) & 0x3FF;
        } else {
            self.count = 0;
        }

        Crystal::from_parts_unchecked(self.id, self.count, now(self.epoch))
    }
}

fn now(epoch: SystemTime) -> u64 {
    SystemTime::now()
        .duration_since(epoch)
        .unwrap()
        .as_millis()
        .try_into()
        .unwrap()
}

#[cfg(test)]
mod tests {
    use crate::now;
    use std::time::{Duration, SystemTime};

    #[test]
    fn now_changes() {
        let start = now(SystemTime::UNIX_EPOCH);
        std::thread::sleep(Duration::from_millis(1));
        assert!(now(SystemTime::UNIX_EPOCH) > start);
    }

    mod crystal {
        use crate::{BerylError, Crystal, CrystalPart};

        #[test]
        fn converts_i64() {
            assert_eq!(i64::from(Crystal::from_parts(0, 0, 0).unwrap()), 0i64);
            assert_eq!(i64::from(Crystal::from_parts(0xFFF, 0x3FF, 0x3FFFFFFFFFF).unwrap()), -1i64);

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
                Crystal::from_parts(0x1000, 0, 0),
                Err(BerylError::PartOutOfBounds(CrystalPart::GeneratorID))
            );
            assert_eq!(
                Crystal::from_parts(0, 0x400, 0),
                Err(BerylError::PartOutOfBounds(CrystalPart::Counter))
            );
            assert_eq!(
                Crystal::from_parts(0, 0, 0x40000000000),
                Err(BerylError::PartOutOfBounds(CrystalPart::Timestamp))
            );

            // Should not be out of bounds
            assert_eq!(Crystal::from_parts(0, 0, 0), Ok(Crystal { crystal: u64::MIN }));
            assert_eq!(Crystal::from_parts(0xFFF, 0x3FF, 0x3FFFFFFFFFF), Ok(Crystal { crystal: u64::MAX }));
        }
    }

    mod generator {
        use crate::{Generator, BerylError, CrystalPart};
        use std::time::{SystemTime, Duration};

        #[test]
        fn initializes() {
            let g = Generator::new(0, SystemTime::UNIX_EPOCH).unwrap();
            assert_eq!(g.count.wrapping_add(1), 0);

            // Should fail to initialize
            assert_eq!(Generator::new(u16::MAX, SystemTime::UNIX_EPOCH), Err(BerylError::PartOutOfBounds(CrystalPart::GeneratorID)));
            assert_eq!(Generator::new(0x1000, SystemTime::UNIX_EPOCH), Err(BerylError::PartOutOfBounds(CrystalPart::GeneratorID)));
        }

        #[test]
        fn generates() {
            let mut g = Generator::new(0, SystemTime::UNIX_EPOCH).unwrap();

            let first = g.try_generate().unwrap(); // Safe because we know the `count` is at 0
            assert_eq!(first.generator(), 0);
            assert_eq!(first.counter(), 0);
            
            let second = g.try_generate().unwrap();
            assert_eq!(second.counter(), 1);

            std::thread::sleep(Duration::from_millis(2));
            let third = g.try_generate().unwrap();
            assert_eq!(third.counter(), 0);
        }

        #[test]
        fn generate_exhausted() {
            fn exhausted_gen() -> Generator {
                Generator {
                    id: 0,
                    epoch: SystemTime::UNIX_EPOCH,
                    count: 0x3FF,
                    last_timestamp: crate::now(SystemTime::UNIX_EPOCH),
                }
            }

            assert_eq!(exhausted_gen().try_generate(), Err(BerylError::GeneratorExhausted));
            assert_eq!(exhausted_gen().generate_block_spin().counter(), 0);
            assert_eq!(exhausted_gen().generate_block_sleep().counter(), 0);
            assert_eq!(exhausted_gen().generate_unchecked().counter(), 0);
        }
    }
}
