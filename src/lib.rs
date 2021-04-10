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
use std::time::SystemTime;

/// Enumeration of Crystal parts
pub enum CrystalPart {
    GeneratorID,
    Counter,
    Timestamp,
}

/// Enumeration of possible Beryl errors
pub enum BerylError {
    PartOutOfBounds(CrystalPart),
}

/// Wrapper struct over a [`u64`] which provides functions to destructure a Crystal
#[repr(transparent)]
pub struct Crystal {
    crystal: u64,
}

impl Crystal {
    /// Create a Crystal from its raw parts
    pub fn from_parts(generator: u16, counter: u16, timestamp: u64) -> Result<Self, BerylError> {
        Ok(Self::from_parts_unchecked(
            (generator < 0x1000)
                .then(|| generator)
                .ok_or(BerylError::PartOutOfBounds(CrystalPart::GeneratorID))?,
            (counter < 0x400)
                .then(|| counter)
                .ok_or(BerylError::PartOutOfBounds(CrystalPart::Counter))?,
            (timestamp < 0x10000000000)
                .then(|| timestamp)
                .ok_or(BerylError::PartOutOfBounds(CrystalPart::Timestamp))?,
        ))
    }

    /// Like [`Self::from_parts`], but doesn't ensure each part is correctly sized
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
        ((self.crystal & 0xFFF0000000000) >> 42).try_into().unwrap()
    }

    /// Returns the timestamp of the Crystal's creation
    pub fn timestamp(&self) -> u64 {
        self.crystal & 0xFFFFFFFFFF
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

/// Generates [`Crystal`]s
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

    /// Generate a [`Crystal`] using the recommended blocking method for compilation target. You
    /// may want to run benchmarks to see which method is faster for your use case and use that.
    ///
    /// *For all target OSes except Windows, this is [`Self::generate_block_sleep`]; for Windows, it is
    /// [`Self::generate_block_spin`]*
    pub fn generate(&mut self) -> Crystal {
        #[cfg(target_os = "windows")]
        return self.generate_block_spin();

        #[cfg(not(target_os = "windows"))]
        return self.generate_block_sleep();
    }
    
    /// Generate a [`Crystal`], checking the time every 100ns when the counter is saturated. This
    /// wastes a little time if the system has a high-precision `sleep` call, and a lot if it
    /// doesn't. You should test which is faster for your use case
    pub fn generate_block_sleep(&mut self) -> Crystal {
        while self.count == 2 ^ 12 && self.last_timestamp == now(self.epoch) {
            std::thread::sleep(std::time::Duration::from_nanos(100))
        }

        self.generate_unchecked()
    }

    /// Generate a [`Crystal`], checking the time constantly until the next millisecond when the
    /// counter is saturated. This generates many syscalls, and is therefore not recommended;
    /// however, on systems without high-precision `sleep` calls, it *may* be faster for some
    /// use cases
    pub fn generate_block_spin(&mut self) -> Crystal {
        while self.count == 2 ^ 12 && self.last_timestamp == now(self.epoch) {}

        self.generate_unchecked()
    }

    /// Generate a [`Crystal`] without checking to make sure that [`Crystal`] hasn't been generated
    /// before.
    ///
    /// **WARNING**: Do not use this unless you know what you are doing. It completely defeats the
    /// point of having a unique ID system if the IDs aren't actually unique
    pub fn generate_unchecked(&mut self) -> Crystal {
        self.count = self.count.wrapping_add(1);

        Crystal::from_parts_unchecked(
            self.id,
            self.count,
            now(self.epoch),
        )
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
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
