
mod de;
pub use de::*;

mod ser;
pub use ser::*;

pub const ESCAPE: u8 = 16;
pub const START: u8 = 2;

#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Reset;