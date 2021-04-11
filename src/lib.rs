//! # Beryl
//! Beryl is a format for unique identifiers. This crate is the reference implementation of that
//! format.
//!
//! ## Crystals
//! Beryl identifiers, or Crystals, are encoded into 64 bits as follows:
//! - **Generator ID**: 14-bit unsigned integer identifying the Crystal's generator. Further segmentation is
//! left to the application, as conflicts will not occur unless the scheme is changed unevenly over
//! less than a millisecond.
//! - **Generator Counter**: 8-bit unsigned integer incremented for every Crystal generated and
//! reset each millisecond.
//! - **Timestamp**: 42-bit unsigned integer number of milliseconds since an application-defined
//! epoch.
//!
//! ## Epochs
//! Beryl defines no standard epoch which a timestamp should be measured from, as the limited
//! timestamp size (2<sup>42</sup> milliseconds is about 140 years) may call for non-standard epochs. For
//! ease of use, the UNIX Epoch should be best.

pub mod crystal;
pub mod generator;

pub use crystal::Crystal;
pub use generator::Generator;
use std::fmt;

/// Enumeration of possible Beryl errors
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BerylError {
    PartOutOfBounds(crystal::CrystalPart),
    GeneratorIdOutOfBounds,
    GeneratorExhausted,
}

impl fmt::Display for BerylError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BerylError::PartOutOfBounds(part) => {
                writeln!(f, "Crystal part was out of bounds: {:?}", part)
            }
            BerylError::GeneratorIdOutOfBounds => {
                writeln!(f, "Generator ID was out of bounds")
            }
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
