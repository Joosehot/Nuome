//! Magic bitboards and attack tables, copied from Steldens (ChessEngine/steldens-rs/src/bitboard.rs) for NEURO: the network written with table lookups and bit counts instead of square loops.
// Bitboard helpers: square tables and runtime-generated magic bitboards.
// Mirrors fast.js square indexing: API squares are a1=0..h8=63; the engine
// internally also uses 10x12 mailbox squares (see position.rs).

pub const FILE_OF64: [usize; 64] = {
    let mut t = [0usize; 64];
    let mut s = 0;
    while s < 64 {
        t[s] = s & 7;
        s += 1;
    }
    t
};

// 64 <-> 120 mailbox square conversion (fast.js sq64to120 / sq120to64).
pub const SQ64_TO_120: [usize; 64] = {
    let mut t = [0usize; 64];
    let mut s = 0;
    while s < 64 {
        t[s] = 21 + (s & 7) + 10 * (s >> 3);
        s += 1;
    }
    t
};

pub const SQ120_TO_64: [i8; 120] = {
    let mut t = [-1i8; 120];
    let mut s = 0;
    while s < 64 {
        t[21 + (s & 7) + 10 * (s >> 3)] = s as i8;
        s += 1;
    }
    t
};

const BISHOP_DIRS: [(i32, i32); 4] = [(-1, -1), (-1, 1), (1, -1), (1, 1)];
const ROOK_DIRS: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];

// Knight / king / pawn attack tables (64-square indexing).
pub struct AttackTables {
    pub knight: [u64; 64],
    pub king: [u64; 64],
    pub pawn_w: [u64; 64],
    pub pawn_b: [u64; 64],
}

pub const KNIGHT_DELTAS: [(i32, i32); 8] = [
    (-2, -1), (-2, 1), (-1, -2), (-1, 2), (1, -2), (1, 2), (2, -1), (2, 1),
];
pub const KING_DELTAS: [(i32, i32); 8] = [
    (-1, -1), (-1, 0), (-1, 1), (0, -1), (0, 1), (1, -1), (1, 0), (1, 1),
];

fn build_attack_tables() -> AttackTables {
    let mut t = AttackTables {
        knight: [0; 64],
        king: [0; 64],
        pawn_w: [0; 64],
        pawn_b: [0; 64],
    };
    for s in 0..64 {
        let (r, f) = ((s / 8) as i32, (s % 8) as i32);
        for &(dr, df) in &KNIGHT_DELTAS {
            let (nr, nf) = (r + dr, f + df);
            if (0..8).contains(&nr) && (0..8).contains(&nf) {
                t.knight[s] |= 1 << (nr * 8 + nf);
            }
        }
        for &(dr, df) in &KING_DELTAS {
            let (nr, nf) = (r + dr, f + df);
            if (0..8).contains(&nr) && (0..8).contains(&nf) {
                t.king[s] |= 1 << (nr * 8 + nf);
            }
        }
        if r < 7 {
            if f > 0 { t.pawn_w[s] |= 1 << ((r + 1) * 8 + f - 1); }
            if f < 7 { t.pawn_w[s] |= 1 << ((r + 1) * 8 + f + 1); }
        }
        if r > 0 {
            if f > 0 { t.pawn_b[s] |= 1 << ((r - 1) * 8 + f - 1); }
            if f < 7 { t.pawn_b[s] |= 1 << ((r - 1) * 8 + f + 1); }
        }
    }
    t
}

fn sliding_attacks(sq: usize, occ: u64, dirs: &[(i32, i32); 4]) -> u64 {
    let (r0, f0) = ((sq / 8) as i32, (sq % 8) as i32);
    let mut a = 0u64;
    for &(dr, df) in dirs {
        let (mut r, mut f) = (r0 + dr, f0 + df);
        while (0..8).contains(&r) && (0..8).contains(&f) {
            let s = r * 8 + f;
            a |= 1 << s;
            if occ & (1u64 << s) != 0 {
                break;
            }
            r += dr;
            f += df;
        }
    }
    a
}

pub struct MagicEntry {
    mask: u64,
    magic: u64,
    shift: u32,
    base: u32,
}

pub struct Magics {
    rook: [MagicEntry; 64],
    bishop: [MagicEntry; 64],
    rook_tab: Vec<u64>,
    bishop_tab: Vec<u64>,
}

// Fixed-seed xorshift64* so magic generation is deterministic.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    fn sparse(&mut self) -> u64 {
        self.next() & self.next() & self.next()
    }
}

fn build_magics(dirs: &[(i32, i32); 4]) -> ([MagicEntry; 64], Vec<u64>) {
    let mut entries: [MagicEntry; 64] = unsafe { std::mem::MaybeUninit::zeroed().assume_init() };
    let mut table: Vec<u64> = Vec::new();
    let mut rng = Rng(0x1234_5678_9ABC_DEF0);
    for s in 0..64 {
        let mask = {
            // ray squares except the last square of each ray
            let (r0, f0) = ((s / 8) as i32, (s % 8) as i32);
            let mut m = 0u64;
            for &(dr, df) in dirs {
                let (mut r, mut f) = (r0 + dr, f0 + df);
                loop {
                    let (nr, nf) = (r + dr, f + df);
                    if !(0..8).contains(&nr) || !(0..8).contains(&nf) {
                        break;
                    }
                    m |= 1 << (r * 8 + f);
                    r = nr;
                    f = nf;
                }
            }
            m
        };
        let bits = mask.count_ones();
        let size = 1usize << bits;
        // enumerate occupancies (carry-rippler) and their attack sets
        let mut occs: Vec<u64> = Vec::with_capacity(size);
        let mut atks: Vec<u64> = Vec::with_capacity(size);
        let mut occ = 0u64;
        loop {
            occs.push(occ);
            atks.push(sliding_attacks(s, occ, dirs));
            occ = occ.wrapping_sub(mask) & mask;
            if occ == 0 {
                break;
            }
        }
        let base = table.len() as u32;
        table.resize(base as usize + occs.len(), 0);
        let shift = 64 - bits;
        let magic = loop {
            let cand = rng.sparse();
            if (cand.wrapping_mul(mask) >> 56).count_ones() < 6 {
                continue;
            }
            let mut used = vec![false; size];
            let mut ok = true;
            for &o in &occs {
                let idx = (o.wrapping_mul(cand) >> shift) as usize;
                if used[idx] {
                    ok = false;
                    break;
                }
                used[idx] = true;
            }
            if ok {
                break cand;
            }
        };
        for (i, &o) in occs.iter().enumerate() {
            let idx = (o.wrapping_mul(magic) >> shift) as usize;
            table[base as usize + idx] = atks[i];
        }
        entries[s] = MagicEntry { mask, magic, shift, base };
    }
    (entries, table)
}

pub struct Tables {
    pub att: AttackTables,
    magics: Magics,
}

impl Tables {
    pub fn new() -> Tables {
        let (rook, rook_tab) = build_magics(&ROOK_DIRS);
        let (bishop, bishop_tab) = build_magics(&BISHOP_DIRS);
        Tables {
            att: build_attack_tables(),
            magics: Magics { rook, bishop, rook_tab, bishop_tab },
        }
    }

    #[inline]
    fn lookup(entry: &MagicEntry, tab: &Vec<u64>, occ: u64) -> u64 {
        tab[entry.base as usize + ((occ & entry.mask).wrapping_mul(entry.magic) >> entry.shift) as usize]
    }

    pub fn rook_attacks(&self, sq64: usize, occ: u64) -> u64 {
        Self::lookup(&self.magics.rook[sq64], &self.magics.rook_tab, occ)
    }
    pub fn bishop_attacks(&self, sq64: usize, occ: u64) -> u64 {
        Self::lookup(&self.magics.bishop[sq64], &self.magics.bishop_tab, occ)
    }
    pub fn queen_attacks(&self, sq64: usize, occ: u64) -> u64 {
        self.rook_attacks(sq64, occ) | self.bishop_attacks(sq64, occ)
    }
}

static TABLES: std::sync::OnceLock<Tables> = std::sync::OnceLock::new();
pub fn tables() -> &'static Tables {
    TABLES.get_or_init(Tables::new)
}
