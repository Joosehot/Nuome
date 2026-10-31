//! The taulukonlaskija on the GPU: a whole class of positions solved from the rules, every position at once.
//!
//! The same retrograde analysis as retro.rs, written as vector steps: the values of all the positions of a class are
//! one vector; a pass computes every position's new value from its children's values at once (one GPU thread a
//! position: its legal moves made, each child's value looked up), the step "the vector through the move matrix",
//! until no value changes; what never settles is a draw. Captures and promotions lead into other classes, solved
//! first (retro::table) and handed to the GPU with this one. The values and the index are the same as retro.rs's, so
//! the tables are interchangeable and checked against each other.
use crate::retro::{key_of, Table, NONE, UNSET};
use cudarc::driver::{CudaDevice, LaunchAsync, LaunchConfig};
use std::sync::Arc;

const KERNELS: &str = r#"
#define UNSET (-32768)
#define NONE (-32767)
#define MAXP 5

__device__ int iabs(int x) { return x < 0 ? -x : x; }
__device__ int sgn(int x) { return (x > 0) - (x < 0); }

// is square sq attacked by the side byWhite? (pieces with t == 0 are gone)
__device__ int attacked(int sq, int byWhite, const signed char* t, const int* s, int k) {
    int f = sq & 7, r = sq >> 3;
    for (int i = 0; i < k; i++) {
        int p = t[i];
        if (p == 0 || (p > 0) != byWhite) continue;
        int pf = s[i] & 7, pr = s[i] >> 3, a = iabs(p);
        int df = f - pf, dr = r - pr, adf = iabs(df), adr = iabs(dr);
        if (a == 1) {
            if (dr == (p > 0 ? 1 : -1) && adf == 1) return 1;
        } else if (a == 2) {
            if ((adf == 1 && adr == 2) || (adf == 2 && adr == 1)) return 1;
        } else if (a == 6) {
            if (adf <= 1 && adr <= 1 && (adf | adr)) return 1;
        } else {
            int diag = (adf == adr && adf > 0), line = ((adf == 0) != (adr == 0));
            if ((a == 3 && diag) || (a == 4 && line) || (a == 5 && (diag || line))) {
                int sf = sgn(df), sr = sgn(dr), x = pf + sf, y = pr + sr, blocked = 0;
                while (x != f || y != r) {
                    int q = y * 8 + x;
                    for (int j = 0; j < k; j++) if (t[j] != 0 && s[j] == q) blocked = 1;
                    if (blocked) break;
                    x += sf; y += sr;
                }
                if (!blocked) return 1;
            }
        }
    }
    return 0;
}

// the index of a set of pieces in a class (cls: the class's pieces in order; same-type pieces by ascending square)
__device__ long long index_of(const signed char* cls, int m, const signed char* t, const int* s, int n, int whiteToMove) {
    int used[MAXP] = {0, 0, 0, 0, 0};
    long long idx = 0, mul = 1;
    for (int c = 0; c < m; c++) {
        int best = -1, bs = 99;
        for (int i = 0; i < n; i++) if (!used[i] && t[i] == cls[c] && s[i] < bs) { bs = s[i]; best = i; }
        if (best < 0) return -1;
        used[best] = 1;
        idx += (long long)bs * mul;
        mul *= 64;
    }
    return idx * 2 + (whiteToMove ? 0 : 1);
}

// the class of a set of pieces, in retro.rs's order: kings, white by type descending, black by type descending
__device__ int canon(const signed char* t, int n, signed char* out) {
    int m = 0;
    out[m++] = 6; out[m++] = -6;
    for (int side = 0; side < 2; side++)
        for (int a = 5; a >= 1; a--)
            for (int i = 0; i < n; i++) if (t[i] == (side == 0 ? a : -a)) out[m++] = t[i];
    return m;
}

__device__ int empty_sq(int q, const signed char* t, const int* s, int k) {
    for (int j = 0; j < k; j++) if (t[j] != 0 && s[j] == q) return 0;
    return 1;
}

__device__ short flipv(short v) { return v > 0 ? (short)(-(v + 2)) : (v < 0 ? (short)(-v) : 0); }

// a piece's target squares (its own pieces not removed yet); a pawn's double step only from its first rank
__device__ int gen(int i, const signed char* t, const int* s, int k, int* targets) {
    int p = t[i], white = p > 0, a = iabs(p), f = s[i] & 7, r = s[i] >> 3, nt = 0;
    if (a == 1) {
        int dir = white ? 1 : -1, r1 = r + dir;
        if (r1 >= 0 && r1 < 8) {
            int one = r1 * 8 + f;
            if (empty_sq(one, t, s, k)) {
                targets[nt++] = one;
                if ((white && r == 1) || (!white && r == 6)) {
                    int two = (r + 2 * dir) * 8 + f;
                    if (empty_sq(two, t, s, k)) targets[nt++] = two;
                }
            }
            for (int df = -1; df <= 1; df += 2) {
                int ff = f + df;
                if (ff < 0 || ff > 7) continue;
                int q = r1 * 8 + ff;
                if (!empty_sq(q, t, s, k)) targets[nt++] = q;
            }
        }
    } else if (a == 2 || a == 6) {
        int A[8], B[8];
        if (a == 2) { int X[8] = {1,2,-1,-2,1,2,-1,-2}, Y[8] = {2,1,2,1,-2,-1,-2,-1}; for (int z = 0; z < 8; z++) { A[z] = X[z]; B[z] = Y[z]; } }
        else { int X[8] = {1,1,1,0,0,-1,-1,-1}, Y[8] = {1,0,-1,1,-1,1,0,-1}; for (int z = 0; z < 8; z++) { A[z] = X[z]; B[z] = Y[z]; } }
        for (int z = 0; z < 8; z++) {
            int ff = f + A[z], rr = r + B[z];
            if (ff >= 0 && ff < 8 && rr >= 0 && rr < 8) targets[nt++] = rr * 8 + ff;
        }
    } else {
        int dfs[8] = {1,1,-1,-1,1,-1,0,0}, drs[8] = {1,-1,1,-1,0,0,1,-1};
        int lo = (a == 4) ? 4 : 0, hi = (a == 3) ? 4 : 8;
        for (int z = lo; z < hi; z++) {
            int ff = f + dfs[z], rr = r + drs[z];
            while (ff >= 0 && ff < 8 && rr >= 0 && rr < 8) {
                int q = rr * 8 + ff;
                targets[nt++] = q;
                if (!empty_sq(q, t, s, k)) break;
                ff += dfs[z]; rr += drs[z];
            }
        }
    }
    return nt;
}

// does the side to move have a legal move (en passant aside)?
__device__ int any_legal(const signed char* t, const int* s, int k, int white) {
    int myk = -1;
    for (int i = 0; i < k; i++) if (t[i] == (white ? 6 : -6)) myk = s[i];
    for (int i = 0; i < k; i++) {
        if (t[i] == 0 || (t[i] > 0) != white) continue;
        int targets[64];
        int nt = gen(i, t, s, k, targets);
        for (int z = 0; z < nt; z++) {
            int to = targets[z], cap = -1;
            for (int j = 0; j < k; j++) if (j != i && t[j] != 0 && s[j] == to) cap = j;
            if (cap >= 0 && ((t[cap] > 0) == white || iabs(t[cap]) == 6)) continue;
            signed char ct[MAXP]; int cs[MAXP];
            for (int j = 0; j < k; j++) { ct[j] = t[j]; cs[j] = s[j]; }
            if (cap >= 0) ct[cap] = 0;
            cs[i] = to;
            if (!attacked(iabs(t[i]) == 6 ? to : myk, !white, ct, cs, k)) return 1;
        }
    }
    return 0;
}

// the value of a set of pieces (the taken ones t == 0) in a smaller class, for the side whiteToMove; UNSET when unknown
__device__ short sub_value(const signed char* ct, const int* cs, int k, int whiteToMove, int nsub, const int* sublen,
                           const signed char* subt, const long long* suboff, const short* subval) {
    signed char cl[MAXP]; signed char rt[MAXP]; int rs[MAXP]; int m = 0;
    for (int j = 0; j < k; j++) if (ct[j] != 0) { rt[m] = ct[j]; rs[m] = cs[j]; m++; }
    int cm = canon(rt, m, cl);
    for (int q = 0; q < nsub; q++) {
        if (sublen[q] != cm) continue;
        int same = 1;
        for (int c = 0; c < cm; c++) if (subt[q * MAXP + c] != cl[c]) same = 0;
        if (!same) continue;
        long long ci = index_of(subt + q * MAXP, cm, rt, rs, m, whiteToMove);
        return (ci < 0) ? (short)UNSET : subval[suboff[q] + ci];
    }
    return (short)UNSET;
}

extern "C" __global__ void init(int k, const signed char* cls, short* val, long long n) {
    long long idx = blockIdx.x * (long long)blockDim.x + threadIdx.x;
    if (idx >= n) return;
    int white = (idx % 2) == 0;
    long long rest = idx / 2;
    int s[MAXP]; signed char t[MAXP];
    for (int i = 0; i < k; i++) { s[i] = (int)(rest % 64); rest /= 64; t[i] = cls[i]; }
    for (int i = 0; i < k; i++) {
        for (int j = 0; j < i; j++) if (s[i] == s[j]) { val[idx] = NONE; return; }
        if (iabs(t[i]) == 1 && (s[i] < 8 || s[i] >= 56)) { val[idx] = NONE; return; }
    }
    // the side not to move may not stand in check
    int ok = -1;
    for (int i = 0; i < k; i++) if (t[i] == (white ? -6 : 6)) ok = s[i];
    if (ok < 0 || attacked(ok, white, t, s, k)) { val[idx] = NONE; return; }
    val[idx] = UNSET;
}

extern "C" __global__ void pass(int k, const signed char* cls, const short* cur, short* nxt, long long n,
                                int nsub, const int* sublen, const signed char* subt, const long long* suboff,
                                const short* subval, int* changed) {
    long long idx = blockIdx.x * (long long)blockDim.x + threadIdx.x;
    if (idx >= n) return;
    short old = cur[idx];
    if (old == NONE) { nxt[idx] = NONE; return; }
    int white = (idx % 2) == 0;
    long long rest = idx / 2;
    int s[MAXP]; signed char t[MAXP];
    for (int i = 0; i < k; i++) { s[i] = (int)(rest % 64); rest /= 64; t[i] = cls[i]; }
    int myk = -1;
    for (int i = 0; i < k; i++) if (t[i] == (white ? 6 : -6)) myk = s[i];
    int any = 0, haswin = 0, alllost = 1;
    short bestwin = 32767, worst = 0;
    for (int i = 0; i < k; i++) {
        int p = t[i];
        if ((p > 0) != white) continue;
        int a = iabs(p), r = s[i] >> 3;
        int targets[64];
        int nt = gen(i, t, s, k, targets);
        for (int z = 0; z < nt; z++) {
            int to = targets[z];
            int cap = -1;
            for (int j = 0; j < k; j++) if (j != i && t[j] != 0 && s[j] == to) cap = j;
            if (cap >= 0 && ((t[cap] > 0) == white || iabs(t[cap]) == 6)) continue;
            int last = (a == 1) && ((white && (to >> 3) == 7) || (!white && (to >> 3) == 0));
            int promos[4] = {5, 4, 3, 2};
            int np = last ? 4 : 1;
            for (int pi = 0; pi < np; pi++) {
                signed char ct[MAXP]; int cs[MAXP];
                for (int j = 0; j < k; j++) { ct[j] = t[j]; cs[j] = s[j]; }
                if (cap >= 0) ct[cap] = 0;
                cs[i] = to;
                if (last) ct[i] = (signed char)(white ? promos[pi] : -promos[pi]);
                int kk = (a == 6) ? to : myk;
                if (attacked(kk, !white, ct, cs, k)) continue;
                any = 1;
                short v;
                if (cap < 0 && !last) {
                    v = cur[index_of(cls, k, ct, cs, k, !white)];
                } else {
                    v = sub_value(ct, cs, k, !white, nsub, sublen, subt, suboff, subval);
                }
                // a pawn's double step beside an enemy pawn: the reply may take it en passant, which the table (no
                // en passant square in its index) does not know; the child's value is then the best of all its moves
                if (a == 1 && iabs((to >> 3) - r) == 2) {
                    int over = ((r + (to >> 3)) / 2) * 8 + (to & 7);
                    int opts = 0, win = 0, unset = 0, draw = 0; short bw = 32767, wl = 0;
                    if (any_legal(ct, cs, k, !white)) {
                        opts++;
                        if (v == UNSET) unset = 1; else if (v > 0) { win = 1; bw = v; } else if (v == 0) draw = 1; else wl = v;
                    }
                    for (int j = 0; j < k; j++) {
                        if (ct[j] != (white ? -1 : 1) || (cs[j] >> 3) != (to >> 3) || iabs((cs[j] & 7) - (to & 7)) != 1) continue;
                        signed char et[MAXP]; int es[MAXP];
                        for (int q = 0; q < k; q++) { et[q] = ct[q]; es[q] = cs[q]; }
                        et[i] = 0; es[j] = over;
                        int ok = -1;
                        for (int q = 0; q < k; q++) if (et[q] == (white ? -6 : 6)) ok = es[q];
                        if (attacked(ok, white, et, es, k)) continue;
                        opts++;
                        short e = sub_value(et, es, k, white, nsub, sublen, subt, suboff, subval);
                        if (e == UNSET || e == NONE) { unset = 1; continue; }
                        short theirs = flipv(e);
                        if (theirs > 0) { win = 1; if (theirs < bw) bw = theirs; }
                        else if (theirs == 0) draw = 1;
                        else if (theirs < wl) wl = theirs;
                    }
                    if (opts > 0) v = win ? bw : (unset ? (short)UNSET : (draw ? (short)0 : wl));
                }
                if (v == UNSET || v == NONE) { alllost = 0; continue; }
                short mine = flipv(v);
                if (mine > 0) { haswin = 1; if (mine < bestwin) bestwin = mine; }
                else if (mine == 0) alllost = 0;
                else if (mine < worst) worst = mine;
            }
        }
    }
    short cand;
    if (!any) cand = attacked(myk, !white, t, s, k) ? (short)-1 : (short)0;
    else if (haswin) cand = bestwin;
    else if (alllost) cand = worst;
    else cand = UNSET;
    nxt[idx] = cand;
    if (cand != old) atomicAdd(changed, 1);
}
"#;

fn canon(mut pieces: Vec<i8>) -> Vec<i8> {
    pieces.retain(|&p| p.abs() != 6);
    pieces.sort_by_key(|&p| (p < 0, -(p.abs() as i32)));
    let mut out = vec![6, -6];
    out.extend(pieces);
    out
}

/// every class a capture or a promotion can lead to from this one
pub fn children(cls: &[i8]) -> Vec<Vec<i8>> {
    let mut out: Vec<Vec<i8>> = Vec::new();
    let rest: Vec<i8> = cls.iter().copied().filter(|p| p.abs() != 6).collect();
    let mut push = |v: Vec<i8>| {
        let c = canon(v);
        if !out.contains(&c) {
            out.push(c);
        }
    };
    for i in 0..rest.len() {
        // a capture
        let mut v = rest.clone();
        v.remove(i);
        push(v.clone());
        // a capture by a pawn that promotes
        for j in 0..v.len() {
            if v[j].abs() == 1 && (v[j] > 0) != (rest[i] > 0) {
                for q in [5, 4, 3, 2] {
                    let mut w = v.clone();
                    w[j] = q * v[j].signum();
                    push(w);
                }
            }
        }
    }
    for j in 0..rest.len() {
        if rest[j].abs() == 1 {
            for q in [5, 4, 3, 2] {
                let mut w = rest.clone();
                w[j] = q * rest[j].signum();
                push(w);
            }
        }
    }
    out.into_iter().map(|mut c| {
        let mut full = vec![6, -6];
        full.extend(c.drain(2..));
        full
    }).collect()
}

/// solve a class on the GPU (its children solved first through retro::table)
pub fn solve(cls: &[i8]) -> Result<Table, String> {
    let k = cls.len();
    if k > 5 {
        return Err("five pieces at most".into());
    }
    let subs: Vec<(Vec<i8>, &'static Table)> = children(cls)
        .into_iter()
        .map(|c| retro_table(&c).map(|t| (c, t)))
        .collect::<Result<_, _>>()?;
    let dev: Arc<CudaDevice> = CudaDevice::new(0).map_err(|e| format!("no GPU: {e:?}"))?;
    dev.load_ptx(cudarc::nvrtc::compile_ptx(KERNELS).map_err(|e| format!("{e:?}"))?, "retro", &["init", "pass"]).map_err(|e| format!("{e:?}"))?;
    let n: usize = 2 * 64usize.pow(k as u32);
    let mut cls5 = [0i8; 5];
    cls5[..k].copy_from_slice(cls);
    let cls_d = dev.htod_copy(cls5.to_vec()).map_err(|e| format!("{e:?}"))?;
    let mut a = dev.alloc_zeros::<i16>(n).map_err(|e| format!("{e:?}"))?;
    let mut b = dev.alloc_zeros::<i16>(n).map_err(|e| format!("{e:?}"))?;
    let cfg = LaunchConfig::for_num_elems(n as u32);
    let init = dev.get_func("retro", "init").unwrap();
    unsafe { init.launch(cfg, (k as i32, &cls_d, &mut a, n as i64)) }.map_err(|e| format!("{e:?}"))?;
    let (mut lens, mut types, mut offs, mut vals) = (Vec::new(), Vec::new(), Vec::new(), Vec::<i16>::new());
    for (c, t) in &subs {
        lens.push(c.len() as i32);
        let mut c5 = [0i8; 5];
        c5[..c.len()].copy_from_slice(c);
        types.extend(c5);
        offs.push(vals.len() as i64);
        vals.extend(&t.val);
    }
    if vals.is_empty() {
        vals.push(0);
        lens.push(0);
        types.extend([0i8; 5]);
        offs.push(0);
    }
    let nsub = subs.len() as i32;
    let (lens_d, types_d, offs_d, vals_d) = (dev.htod_copy(lens).unwrap(), dev.htod_copy(types).unwrap(), dev.htod_copy(offs).unwrap(), dev.htod_copy(vals).unwrap());
    let mut changed = dev.alloc_zeros::<i32>(1).unwrap();
    let pass = dev.get_func("retro", "pass").unwrap();
    let mut passes = 0;
    loop {
        dev.htod_sync_copy_into(&[0i32], &mut changed).unwrap();
        unsafe { pass.clone().launch(cfg, (k as i32, &cls_d, &a, &mut b, n as i64, nsub, &lens_d, &types_d, &offs_d, &vals_d, &mut changed)) }.map_err(|e| format!("{e:?}"))?;
        std::mem::swap(&mut a, &mut b);
        passes += 1;
        let c = dev.dtoh_sync_copy(&changed).unwrap()[0];
        if c == 0 || passes > 1000 {
            break;
        }
    }
    let mut val = dev.dtoh_sync_copy(&a).map_err(|e| format!("{e:?}"))?;
    for v in val.iter_mut() {
        if *v == UNSET {
            *v = 0;
        }
    }
    eprintln!("  {} solved on the GPU: {} positions, {passes} passes", key_of(cls), n);
    let _ = NONE;
    Ok(Table { pieces: cls.to_vec(), val })
}

/// a class's table: up to three pieces as retro.rs solves them; four and five on the GPU, kept on disk
pub fn retro_table(cls: &[i8]) -> Result<&'static Table, String> {
    if cls.len() <= 3 {
        return crate::retro::table(cls).ok_or_else(|| "no table".to_string());
    }
    crate::retro::table_with(cls, &|c| solve(c).map_err(|e| e.to_string()))
}

fn parse_class(name: &str) -> Vec<i8> {
    let ty = |c: char| match c { 'P' => 1, 'N' => 2, 'B' => 3, 'R' => 4, 'Q' => 5, _ => 6 };
    let (w, b) = name.split_once('v').unwrap();
    let mut v: Vec<i8> = w.chars().map(ty).collect();
    v.extend(b.chars().map(|c| -ty(c)));
    canon(v)
}

/// `nuome --ideas retro-gpu-check`: the GPU against retro.rs on every three-piece position, then four-piece classes
/// solved on the GPU with a sample of their positions written for the Syzygy check (tools/retro_check.py)
pub fn check() -> Result<(), String> {
    for name in ["KQvK", "KRvK", "KPvK", "KBvK", "KNvK"] {
        let cls = parse_class(name);
        let cpu = crate::retro::table(&cls).ok_or("no table")?;
        let t0 = std::time::Instant::now();
        let gpu = solve(&cls)?;
        let diff = cpu.val.iter().zip(&gpu.val).filter(|(a, b)| a != b).count();
        println!("{name}: the GPU and retro.rs differ in {diff} of {} positions ({:.2} s on the GPU)", cpu.val.len(), t0.elapsed().as_secs_f64());
    }
    let mut out = String::new();
    for name in ["KQvKR", "KRvKR", "KRvKB", "KRvKN", "KQvKQ", "KBNvK", "KBBvK", "KNNvK", "KPvKP", "KRvKP", "KQvKP", "KPPvK", "KBPvK"] {
        let cls = parse_class(name);
        let t0 = std::time::Instant::now();
        let t = retro_table(&cls)?;
        let valid: Vec<usize> = (0..t.val.len()).filter(|&i| t.val[i] != NONE).collect();
        let (mut w, mut d, mut l, mut longest) = (0, 0, 0, 0i16);
        for &i in &valid {
            let v = t.val[i];
            if v > 0 { w += 1; longest = longest.max(v); } else if v < 0 { l += 1 } else { d += 1 }
        }
        println!("{name}: {} positions ({} legal): won {w}, drawn {d}, lost {l}; longest win {longest} plies ({:.1} s)", t.val.len(), valid.len(), t0.elapsed().as_secs_f64());
        let mut x: u64 = 0x2545F4914F6CDD1D;
        for _ in 0..1000 {
            x ^= x << 13; x ^= x >> 7; x ^= x << 17;
            let i = valid[(x % valid.len() as u64) as usize];
            let b = t.board(i).unwrap();
            out.push_str(&format!("{name}	{}	{}
", b.fen(), t.val[i].signum()));
        }
    }
    std::fs::write("out/golden/retro_check.tsv", out).map_err(|e| e.to_string())?;
    println!("samples written to out/golden/retro_check.tsv: python tools/retro_check.py");
    Ok(())
}

/// `nuome --ideas retro-probe`: a FEN a line on stdin, its value in Nuome's own tables a line on stdout
pub fn probe() -> Result<(), String> {
    use std::io::BufRead;
    for line in std::io::stdin().lock().lines() {
        let line = line.map_err(|e| e.to_string())?;
        let b = crate::golden::Board::from_fen(line.trim())?;
        println!("{}", value_of(&b)?);
    }
    Ok(())
}

/// a position's value for the side to move from the tables; with an en passant square (which the tables' index
/// does not hold) the best of its moves
pub fn value_of(b: &crate::golden::Board) -> Result<i16, String> {
    if b.ep.is_none() {
        let t = retro_table(&crate::retro::pieces_of(b))?;
        return Ok(t.index(b).map(|i| t.val[i]).unwrap_or(NONE));
    }
    let ms = b.moves();
    if ms.is_empty() {
        return Ok(if b.in_check() { -1 } else { 0 });
    }
    let mut best = i16::MIN;
    for m in ms {
        let v = crate::retro::flip(value_of(&b.play(m))?);
        let better = match (v > 0, best > 0) {
            (true, true) => v < best,
            (true, false) => true,
            (false, true) => false,
            _ => (v == 0 && best < 0) || (v < best && best < 0),
        };
        if best == i16::MIN || better {
            best = v;
        }
    }
    Ok(best)
}
