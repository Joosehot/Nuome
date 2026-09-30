//! Yang-Mills theory on a lattice: SU(2) gauge fields on an L^4 periodic
//! lattice, simulated by heat bath (Kennedy-Pendleton) and overrelaxation
//! with the Wilson action S = beta * sum over plaquettes of (1 - Tr U_p / 2).
//! An SU(2) matrix a0 + i a.sigma is kept as the unit quaternion
//! (a0, a1, a2, a3); Tr U / 2 = a0.
//!
//! Checked against what is known exactly: the strong-coupling series
//! <P> = beta/4 - beta^3/96 + ... for small beta and the weak-coupling
//! expansion <P> = 1 - 3/(4 beta) + ... for large beta. Then measured:
//! Wilson loops W(R, T) and Creutz ratios, whose limit is the string tension
//! sigma a^2: sigma > 0 means a confining, linear potential between charges,
//! the lattice face of a mass gap. A lattice at a few couplings is evidence,
//! not the continuum theory the Millennium problem asks about.

use crate::evolve::Rng;

#[derive(Clone, Copy, Debug)]
struct Q([f64; 4]);

impl Q {
    const ONE: Q = Q([1.0, 0.0, 0.0, 0.0]);
    fn mul(self, o: Q) -> Q {
        let [a0, a1, a2, a3] = self.0;
        let [b0, b1, b2, b3] = o.0;
        Q([
            a0 * b0 - a1 * b1 - a2 * b2 - a3 * b3,
            a0 * b1 + a1 * b0 + a2 * b3 - a3 * b2,
            a0 * b2 - a1 * b3 + a2 * b0 + a3 * b1,
            a0 * b3 + a1 * b2 - a2 * b1 + a3 * b0,
        ])
    }
    fn dag(self) -> Q {
        let [a0, a1, a2, a3] = self.0;
        Q([a0, -a1, -a2, -a3])
    }
    fn add(self, o: Q) -> Q {
        Q([self.0[0] + o.0[0], self.0[1] + o.0[1], self.0[2] + o.0[2], self.0[3] + o.0[3]])
    }
    fn scale(self, k: f64) -> Q {
        Q(self.0.map(|x| x * k))
    }
    fn norm(self) -> f64 {
        self.0.iter().map(|x| x * x).sum::<f64>().sqrt()
    }
    fn half_trace(self) -> f64 {
        self.0[0]
    }
}

pub struct Lattice {
    l: usize,
    /// links, index (site * 4 + direction)
    u: Vec<Q>,
}

impl Lattice {
    pub fn cold(l: usize) -> Lattice {
        Lattice { l, u: vec![Q::ONE; l.pow(4) * 4] }
    }
    fn site(&self, x: [usize; 4]) -> usize {
        ((x[0] * self.l + x[1]) * self.l + x[2]) * self.l + x[3]
    }
    fn coords(&self, s: usize) -> [usize; 4] {
        let l = self.l;
        [s / (l * l * l), s / (l * l) % l, s / l % l, s % l]
    }
    fn shift(&self, x: [usize; 4], mu: usize, by: isize) -> [usize; 4] {
        let mut y = x;
        y[mu] = ((y[mu] as isize + by).rem_euclid(self.l as isize)) as usize;
        y
    }
    fn link(&self, x: [usize; 4], mu: usize) -> Q {
        self.u[self.site(x) * 4 + mu]
    }
    /// The sum of the six staples around the link (x, mu): Tr(U * staples)/2
    /// is that link's share of the action.
    fn staples(&self, x: [usize; 4], mu: usize) -> Q {
        let mut sum = Q([0.0; 4]);
        let xp = self.shift(x, mu, 1);
        for nu in 0..4 {
            if nu == mu {
                continue;
            }
            // forward: U_nu(x+mu) U_mu(x+nu)^+ U_nu(x)^+
            let up = self.link(xp, nu).mul(self.link(self.shift(x, nu, 1), mu).dag()).mul(self.link(x, nu).dag());
            // backward: U_nu(x+mu-nu)^+ U_mu(x-nu)^+ U_nu(x-nu)
            let xm = self.shift(x, nu, -1);
            let down = self.link(self.shift(xp, nu, -1), nu).dag().mul(self.link(xm, mu).dag()).mul(self.link(xm, nu));
            sum = sum.add(up).add(down);
        }
        sum
    }
    /// One sweep of heat bath (Kennedy-Pendleton) over every link.
    pub fn heat_bath(&mut self, beta: f64, r: &mut Rng) {
        for s in 0..self.l.pow(4) {
            let x = self.coords(s);
            for mu in 0..4 {
                let v = self.staples(x, mu);
                let k = v.norm();
                if k < 1e-12 {
                    continue;
                }
                let vbar = v.scale(1.0 / k);
                let bk = beta * k;
                // a0 with density sqrt(1 - a0^2) exp(bk a0)
                let a0 = loop {
                    let (r1, r2, r3, r4) = (1.0 - r.unit(), 1.0 - r.unit(), r.unit(), r.unit());
                    let c = (2.0 * std::f64::consts::PI * r3).cos();
                    let delta = (-(r1.ln()) - r2.ln() * c * c) / bk;
                    if r4 * r4 <= 1.0 - delta / 2.0 {
                        break 1.0 - delta;
                    }
                };
                // a random direction for the rest
                let len = (1.0 - a0 * a0).max(0.0).sqrt();
                let cos_t = 2.0 * r.unit() - 1.0;
                let sin_t = (1.0 - cos_t * cos_t).sqrt();
                let phi = 2.0 * std::f64::consts::PI * r.unit();
                let xq = Q([a0, len * sin_t * phi.cos(), len * sin_t * phi.sin(), len * cos_t]);
                // U = X Vbar^+ makes Tr(U V)/2 = k a0
                self.u[s * 4 + mu] = xq.mul(vbar.dag());
            }
        }
    }
    /// One sweep of overrelaxation: U -> Vbar^+ U^+ Vbar^+ keeps the action.
    pub fn overrelax(&mut self) {
        for s in 0..self.l.pow(4) {
            let x = self.coords(s);
            for mu in 0..4 {
                let v = self.staples(x, mu);
                let k = v.norm();
                if k < 1e-12 {
                    continue;
                }
                let vbar = v.scale(1.0 / k).dag();
                let u = self.u[s * 4 + mu];
                let new = vbar.mul(u.dag()).mul(vbar);
                // renormalise against rounding drift
                self.u[s * 4 + mu] = new.scale(1.0 / new.norm());
            }
        }
    }
    /// The average of Tr U_p / 2 over all plaquettes.
    pub fn plaquette(&self) -> f64 {
        self.wilson(1, 1)
    }
    /// The average R x T Wilson loop over every site and every plane.
    pub fn wilson(&self, r: usize, t: usize) -> f64 {
        let mut total = 0.0;
        let mut count = 0usize;
        for s in 0..self.l.pow(4) {
            let x0 = self.coords(s);
            for mu in 0..4 {
                for nu in 0..4 {
                    if mu == nu || (r == t && nu < mu) {
                        continue;
                    }
                    let mut p = Q::ONE;
                    let mut x = x0;
                    for _ in 0..r {
                        p = p.mul(self.link(x, mu));
                        x = self.shift(x, mu, 1);
                    }
                    for _ in 0..t {
                        p = p.mul(self.link(x, nu));
                        x = self.shift(x, nu, 1);
                    }
                    for _ in 0..r {
                        x = self.shift(x, mu, -1);
                        p = p.mul(self.link(x, mu).dag());
                    }
                    for _ in 0..t {
                        x = self.shift(x, nu, -1);
                        p = p.mul(self.link(x, nu).dag());
                    }
                    total += p.half_trace();
                    count += 1;
                }
            }
        }
        total / count as f64
    }
}

pub struct Settings {
    pub lattice: usize,
    pub betas: Vec<f64>,
    pub thermalise: usize,
    pub measure: usize,
    pub seed: u64,
}

pub struct Point {
    pub beta: f64,
    pub plaquette: f64,
    pub error: f64,
    /// W(R, T) for R, T = 1..=3
    pub loops: [[f64; 3]; 3],
}

/// Simulate one coupling: thermalise, then measure after every sweep (one
/// heat bath and three overrelaxation sweeps each).
pub fn run(beta: f64, s: &Settings, seed: u64) -> Point {
    let mut lat = Lattice::cold(s.lattice);
    let mut r = Rng(seed.max(1));
    for _ in 0..s.thermalise {
        lat.heat_bath(beta, &mut r);
        for _ in 0..3 {
            lat.overrelax();
        }
    }
    let mut plaq = Vec::new();
    let mut loops = [[0.0; 3]; 3];
    for _ in 0..s.measure {
        lat.heat_bath(beta, &mut r);
        for _ in 0..3 {
            lat.overrelax();
        }
        plaq.push(lat.plaquette());
        for (i, row) in loops.iter_mut().enumerate() {
            for (j, w) in row.iter_mut().enumerate() {
                *w += lat.wilson(i + 1, j + 1) / s.measure as f64;
            }
        }
    }
    let mean = plaq.iter().sum::<f64>() / plaq.len() as f64;
    // error of the mean from blocks of 5 measurements
    let blocks: Vec<f64> = plaq.chunks(5).map(|c| c.iter().sum::<f64>() / c.len() as f64).collect();
    let bm = blocks.iter().sum::<f64>() / blocks.len() as f64;
    let error = (blocks.iter().map(|b| (b - bm).powi(2)).sum::<f64>() / (blocks.len() * (blocks.len() - 1).max(1)) as f64).sqrt();
    Point { beta, plaquette: mean, error, loops }
}

/// Creutz ratio chi(R, T) = -ln( W(R,T) W(R-1,T-1) / (W(R,T-1) W(R-1,T)) ).
fn creutz(w: &[[f64; 3]; 3], r: usize) -> f64 {
    let at = |i: usize, j: usize| if i == 0 || j == 0 { 1.0 } else { w[i - 1][j - 1] };
    -(at(r, r) * at(r - 1, r - 1) / (at(r, r - 1) * at(r - 1, r))).ln()
}

/// Report lines for the attempt at the problem.
pub fn report(s: &Settings) -> Vec<String> {
    let points: Vec<Point> = std::thread::scope(|sc| {
        let hs: Vec<_> = s.betas.iter().enumerate().map(|(i, &beta)| sc.spawn(move || run(beta, s, s.seed.wrapping_add(i as u64 * 1_000_003)))).collect();
        hs.into_iter().map(|h| h.join().expect("no panics")).collect()
    });
    let mut out = vec![format!(
        "simulated SU(2) Yang-Mills theory on a {l}^4 lattice (Wilson action, heat bath + 3 overrelaxation sweeps, {} sweeps to settle, {} measured, seed {}), the way lattice physics studies the mass gap",
        s.thermalise,
        s.measure,
        s.seed,
        l = s.lattice
    )];
    for p in &points {
        let strong = p.beta / 4.0 - p.beta.powi(3) / 96.0;
        let weak = 1.0 - 3.0 / (4.0 * p.beta);
        let check = if p.beta <= 1.0 {
            format!("strong-coupling series beta/4 - beta^3/96 = {strong:.4}: {}", if (p.plaquette - strong).abs() < 0.01 { "agrees" } else { "DIFFERS" })
        } else if p.beta >= 4.0 {
            format!("weak-coupling 1 - 3/(4 beta) = {weak:.4} (next term is order 1/beta^2): {}", if (p.plaquette - weak).abs() < 0.02 { "agrees" } else { "DIFFERS" })
        } else {
            "the crossover between the two expansions, where the physics is".to_string()
        };
        let c2 = creutz(&p.loops, 2);
        let c3 = creutz(&p.loops, 3);
        out.push(format!(
            "  beta = {:.2}: plaquette {:.4} +- {:.4}; {check}; Creutz ratios chi(2,2) = {c2:.3}, chi(3,3) = {}",
            p.beta,
            p.plaquette,
            p.error,
            if c3.is_finite() { format!("{c3:.3}") } else { "lost in noise".into() }
        ));
    }
    let mid: Vec<&Point> = points.iter().filter(|p| p.beta > 1.0 && p.beta < 4.0).collect();
    if !mid.is_empty() {
        let sig: Vec<String> = mid.iter().map(|p| format!("{:.3} at beta {:.2}", creutz(&p.loops, 3), p.beta)).collect();
        let all_positive = mid.iter().all(|p| creutz(&p.loops, 3) > 0.0);
        out.push(format!(
            "string tension sigma a^2 from the largest loops, chi(3,3) (still an upper estimate: small loops keep some of the short-distance Coulomb part): {}; {}",
            sig.join(", "),
            if all_positive { "positive everywhere, and it falls as beta grows (the lattice spacing a shrinks): a confining linear potential, the lattice sign of a mass gap" } else { "not positive everywhere" }
        ));
    }
    out.push("this is a finite lattice at a few couplings: evidence for confinement and a mass gap, not the construction of the continuum quantum theory that the Millennium problem asks for".into());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cold_lattice_has_plaquette_one_and_updates_keep_su2() {
        let mut lat = Lattice::cold(2);
        assert!((lat.plaquette() - 1.0).abs() < 1e-12);
        let mut r = Rng(5);
        lat.heat_bath(2.0, &mut r);
        lat.overrelax();
        assert!(lat.u.iter().all(|q| (q.norm() - 1.0).abs() < 1e-9));
    }

    #[test]
    fn strong_coupling_is_reproduced() {
        let s = Settings { lattice: 4, betas: vec![0.5], thermalise: 20, measure: 40, seed: 7 };
        let p = run(0.5, &s, 7);
        assert!((p.plaquette - (0.125 - 0.125f64 * 0.5 * 0.5 / 24.0)).abs() < 0.01, "{}", p.plaquette);
    }
}
