
/// Implementation from
/// https://resources.basicmicro.com/cyclic-redundancy-check-explained/.
///
/// Generates a checksum that can be used to detect (not recover) from errors in
/// a packet payload that can appear from data loss or corruption.
#[derive(Clone, Debug)]
pub struct CRC16 {
    pub state: u16,
    pub poly: u16,
}

impl CRC16 {
    pub const fn new(init: u16, poly: u16) -> Self { Self { state: init, poly } }

    pub fn resume(&mut self, byte: u8) {
        self.state ^= (byte as u16) << 8;
        for _ in 0..8 {
            if (self.state & 0x8000) != 0 {
                self.state = (self.state << 1) ^ self.poly;
            } else {
                self.state = self.state << 1;
            }
        }
    }

    pub const fn complete(self) -> u16 { self.state }
}

impl Default for CRC16 {
    fn default() -> Self { Self::new(0, 0x1021) }
}
