// 00: End of cipher spec.
#[derive(Debug, Clone)]
pub enum Op {
    Reverse,
    Xor(u8),
    XorPos,
    Add(u8),
    Sub(u8),
    AddPos,
    SubPos,
    // possibly more, depending on the spec
}

impl Op {
    pub fn apply(&self, byte: u8, pos: usize) -> u8 {
        match *self {
            Op::Reverse => byte.reverse_bits(),
            Op::Xor(n) => byte ^ n,
            Op::XorPos => byte ^ ((pos % 256) as u8),
            Op::Add(n) => byte.wrapping_add(n),
            Op::Sub(n) => byte.wrapping_sub(n),
            Op::AddPos => byte.wrapping_add((pos % 256) as u8),
            Op::SubPos => byte.wrapping_sub((pos % 256) as u8),
        }
    }

    pub fn inverse(&self) -> Op {
        match *self {
            Op::Reverse => Op::Reverse,
            Op::Xor(n) => Op::Xor(n),
            Op::XorPos => Op::XorPos,
            Op::Add(n) => Op::Sub(n),
            Op::Sub(n) => Op::Add(n), // or explicitly Sub(n)
            Op::AddPos => Op::SubPos,
            Op::SubPos => Op::AddPos,
        }
    }
}
