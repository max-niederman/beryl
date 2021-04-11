//! Utilities to generate non-conflicting Crystals

use crate::crystal::Crystal;
use crate::BerylError;
use std::convert::TryInto;
use std::time::{Duration, SystemTime};

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
            id: (id <= 0x3FFF)
                .then(|| id)
                .ok_or(BerylError::GeneratorIdOutOfBounds)?,
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
    use super::{now, Generator};
    use crate::BerylError;
    use std::time::{Duration, SystemTime};

    #[test]
    fn now_changes() {
        let start = now(SystemTime::UNIX_EPOCH);
        std::thread::sleep(Duration::from_millis(1));
        assert!(now(SystemTime::UNIX_EPOCH) > start);
    }

    #[test]
    fn initializes() {
        let g = Generator::new(0, SystemTime::UNIX_EPOCH).unwrap();
        assert_eq!(g.count.wrapping_add(1), 0);

        // Should fail to initialize
        assert_eq!(
            Generator::new(u16::MAX, SystemTime::UNIX_EPOCH),
            Err(BerylError::GeneratorIdOutOfBounds)
        );
        assert_eq!(
            Generator::new(0x4000, SystemTime::UNIX_EPOCH),
            Err(BerylError::GeneratorIdOutOfBounds)
        );
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
                last_timestamp: now(SystemTime::UNIX_EPOCH),
            }
        }

        assert_eq!(
            exhausted_gen().try_generate(),
            Err(BerylError::GeneratorExhausted)
        );
        assert_eq!(exhausted_gen().generate_block_spin().counter(), 0);
        assert_eq!(exhausted_gen().generate_block_sleep().counter(), 0);
        assert_eq!(exhausted_gen().generate_unchecked().counter(), 0);
    }
}
