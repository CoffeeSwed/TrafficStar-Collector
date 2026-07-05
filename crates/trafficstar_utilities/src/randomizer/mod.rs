use rand_pcg::rand_core::Rng;

use rand::RngCore;

pub struct Lcg128XRandomizer {
    rng: rand_pcg::Lcg128Xsl64,
}

impl Lcg128XRandomizer {
    pub fn new(randomizer : rand_pcg::Lcg128Xsl64) -> Self {
        Lcg128XRandomizer { rng : randomizer }
    }
}

impl RngCore for Lcg128XRandomizer {
    fn next_u32(&mut self) -> u32 {
        self.rng.next_u32()
    }

    fn next_u64(&mut self) -> u64 {
        self.rng.next_u64()
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.rng.fill_bytes(dest);
    }
    
    #[allow(clippy::needless_range_loop)]
    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand::Error> {
        let mut index = 0;
        while index < dest.len(){
            let random = self.next_u32();
            for byte in random.to_le_bytes(){
                dest[index] = byte;
                index += 1;
            }
        }
        Ok(())
    }
}
