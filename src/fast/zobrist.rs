// Zobrist keys replicating fast.js's exact PRNG sequence (L38-41): dual 32-bit
// keys, same seed and generation order, so TT indices and collision behavior
// are identical to the JS engine.
pub struct Zobrist {
    pub piece: [u32; 15 * 120],  // [pc * 120 + sq120]
    pub piece2: [u32; 15 * 120],
    pub side: u32,
    pub side2: u32,
    pub castle: [u32; 16],
    pub castle2: [u32; 16],
    pub ep: [u32; 120], // keyed by 120-square
    pub ep2: [u32; 120],
}

// JS: s=(s+0x9e3779b9)>>>0; z=s; z=imul(z^(z>>>16),0x85ebca6b);
//     z=imul(z^(z>>>13),0xc2b2ae35); return (z^(z>>>16))>>>0
#[inline]
fn rnd(s: &mut u32) -> u32 {
    *s = s.wrapping_add(0x9e37_79b9);
    let mut z = *s;
    z = (z ^ (z >> 16)).wrapping_mul(0x85eb_ca6b);
    z = (z ^ (z >> 13)).wrapping_mul(0xc2b2_ae35);
    z ^ (z >> 16)
}

pub fn build() -> Zobrist {
    let mut s: u32 = 0x6d2b_79f5;
    // JS line 40 evaluates ZS=rnd(), ZS2=rnd() first
    let side = rnd(&mut s);
    let side2 = rnd(&mut s);
    let mut piece = [0u32; 15 * 120];
    let mut piece2 = [0u32; 15 * 120];
    for i in 0..15 * 120 {
        piece[i] = rnd(&mut s);
        piece2[i] = rnd(&mut s);
    }
    let mut castle = [0u32; 16];
    let mut castle2 = [0u32; 16];
    for i in 0..16 {
        castle[i] = rnd(&mut s);
        castle2[i] = rnd(&mut s);
    }
    let mut ep = [0u32; 120];
    let mut ep2 = [0u32; 120];
    for i in 0..120 {
        ep[i] = rnd(&mut s);
        ep2[i] = rnd(&mut s);
    }
    Zobrist { piece, piece2, side, side2, castle, castle2, ep, ep2 }
}

pub static ZOB: std::sync::OnceLock<Zobrist> = std::sync::OnceLock::new();
pub fn zob() -> &'static Zobrist {
    ZOB.get_or_init(build)
}
