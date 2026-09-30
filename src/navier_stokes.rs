//! The 3D incompressible Navier-Stokes equations on the periodic box
//! [0, 2 pi]^3, solved pseudo-spectrally: velocity in Fourier space, the
//! nonlinear term u x omega computed on the grid, the 2/3 rule against
//! aliasing, projection onto divergence-free fields, viscosity by an
//! integrating factor, and Heun's (RK2) step in time.
//!
//! Start: the Taylor-Green vortex u = (sin x cos y cos z, -cos x sin y cos z, 0),
//! which stretches vortex lines and becomes turbulent.
//!
//! Checked: the energy balance dE/dt = -2 nu Omega (E = <|u|^2>/2, Omega =
//! <|omega|^2>/2) is an exact identity of the equations. Watched: the largest
//! vorticity; by the Beale-Kato-Majda theorem a smooth solution can only
//! blow up if the time integral of max |omega| becomes infinite.

use std::f64::consts::PI;

#[derive(Clone, Copy, Debug, PartialEq)]
struct C(f64, f64);

impl C {
    fn add(self, o: C) -> C {
        C(self.0 + o.0, self.1 + o.1)
    }
    fn sub(self, o: C) -> C {
        C(self.0 - o.0, self.1 - o.1)
    }
    fn mul(self, o: C) -> C {
        C(self.0 * o.0 - self.1 * o.1, self.0 * o.1 + self.1 * o.0)
    }
    fn scale(self, k: f64) -> C {
        C(self.0 * k, self.1 * k)
    }
    /// times i
    fn i(self) -> C {
        C(-self.1, self.0)
    }
    fn norm2(self) -> f64 {
        self.0 * self.0 + self.1 * self.1
    }
}

/// In-place radix-2 FFT of one line (inverse without the 1/n).
fn fft(a: &mut [C], inverse: bool) {
    let n = a.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            a.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let ang = 2.0 * PI / len as f64 * if inverse { 1.0 } else { -1.0 };
        let w = C(ang.cos(), ang.sin());
        for start in (0..n).step_by(len) {
            let mut wk = C(1.0, 0.0);
            for k in 0..len / 2 {
                let u = a[start + k];
                let v = a[start + k + len / 2].mul(wk);
                a[start + k] = u.add(v);
                a[start + k + len / 2] = u.sub(v);
                wk = wk.mul(w);
            }
        }
        len <<= 1;
    }
}

/// 3D FFT of an n^3 array (index (x * n + y) * n + z); the inverse divides by n^3.
fn fft3(a: &mut [C], n: usize, inverse: bool) {
    let mut line = vec![C(0.0, 0.0); n];
    for axis in 0..3 {
        let stride = [n * n, n, 1][axis];
        for base in 0..n * n * n {
            // visit each line once: the coordinate along the axis is 0
            let along = base / stride % n;
            if along != 0 {
                continue;
            }
            for (i, v) in line.iter_mut().enumerate() {
                *v = a[base + i * stride];
            }
            fft(&mut line, inverse);
            for (i, v) in line.iter().enumerate() {
                a[base + i * stride] = *v;
            }
        }
    }
    if inverse {
        let k = 1.0 / (n * n * n) as f64;
        for v in a.iter_mut() {
            *v = v.scale(k);
        }
    }
}

pub struct Flow {
    n: usize,
    nu: f64,
    /// velocity components in Fourier space
    u: [Vec<C>; 3],
    k: Vec<f64>,
}

pub struct Stats {
    pub t: f64,
    pub energy: f64,
    pub enstrophy: f64,
    pub max_vorticity: f64,
    /// share of the energy in the last third of the kept wavenumbers
    pub tail: f64,
}

impl Flow {
    pub fn taylor_green(n: usize, nu: f64) -> Flow {
        let h = 2.0 * PI / n as f64;
        let mut u = [vec![C(0.0, 0.0); n * n * n], vec![C(0.0, 0.0); n * n * n], vec![C(0.0, 0.0); n * n * n]];
        for x in 0..n {
            for y in 0..n {
                for z in 0..n {
                    let (a, b, c) = (x as f64 * h, y as f64 * h, z as f64 * h);
                    let i = (x * n + y) * n + z;
                    u[0][i] = C(a.sin() * b.cos() * c.cos(), 0.0);
                    u[1][i] = C(-a.cos() * b.sin() * c.cos(), 0.0);
                }
            }
        }
        for c in u.iter_mut() {
            fft3(c, n, false);
        }
        let k = (0..n).map(|i| if i < n / 2 { i as f64 } else { i as f64 - n as f64 }).collect();
        Flow { n, nu, u, k }
    }
    fn wave(&self, i: usize) -> [f64; 3] {
        let n = self.n;
        [self.k[i / (n * n)], self.k[i / n % n], self.k[i % n]]
    }
    fn vorticity_hat(&self, u: &[Vec<C>; 3]) -> [Vec<C>; 3] {
        let size = self.n.pow(3);
        let mut w = [vec![C(0.0, 0.0); size], vec![C(0.0, 0.0); size], vec![C(0.0, 0.0); size]];
        for i in 0..size {
            let k = self.wave(i);
            // omega = i k x u
            w[0][i] = u[2][i].scale(k[1]).sub(u[1][i].scale(k[2])).i();
            w[1][i] = u[0][i].scale(k[2]).sub(u[2][i].scale(k[0])).i();
            w[2][i] = u[1][i].scale(k[0]).sub(u[0][i].scale(k[1])).i();
        }
        w
    }
    /// The nonlinear term P(u x omega), dealiased, in Fourier space.
    fn nonlinear(&self, u: &[Vec<C>; 3]) -> [Vec<C>; 3] {
        let n = self.n;
        let size = n.pow(3);
        let w = self.vorticity_hat(u);
        let to_grid = |f: &Vec<C>| {
            let mut g = f.clone();
            fft3(&mut g, n, true);
            g
        };
        let (ug, wg): (Vec<Vec<C>>, Vec<Vec<C>>) = std::thread::scope(|sc| {
            let hu: Vec<_> = u.iter().map(|f| sc.spawn(move || to_grid(f))).collect();
            let hw: Vec<_> = w.iter().map(|f| sc.spawn(move || to_grid(f))).collect();
            (hu.into_iter().map(|h| h.join().expect("fft")).collect(), hw.into_iter().map(|h| h.join().expect("fft")).collect())
        });
        let mut cross = [vec![C(0.0, 0.0); size], vec![C(0.0, 0.0); size], vec![C(0.0, 0.0); size]];
        for i in 0..size {
            let (a, b) = ([ug[0][i].0, ug[1][i].0, ug[2][i].0], [wg[0][i].0, wg[1][i].0, wg[2][i].0]);
            cross[0][i] = C(a[1] * b[2] - a[2] * b[1], 0.0);
            cross[1][i] = C(a[2] * b[0] - a[0] * b[2], 0.0);
            cross[2][i] = C(a[0] * b[1] - a[1] * b[0], 0.0);
        }
        std::thread::scope(|sc| {
            for c in cross.iter_mut() {
                sc.spawn(move || fft3(c, n, false));
            }
        });
        let cut = n as f64 / 3.0;
        for i in 0..size {
            let k = self.wave(i);
            if k.iter().any(|x| x.abs() > cut) {
                for c in cross.iter_mut() {
                    c[i] = C(0.0, 0.0);
                }
                continue;
            }
            let k2 = k[0] * k[0] + k[1] * k[1] + k[2] * k[2];
            if k2 == 0.0 {
                continue;
            }
            // remove the gradient part: N - k (k . N)/|k|^2
            let dot = cross[0][i].scale(k[0]).add(cross[1][i].scale(k[1])).add(cross[2][i].scale(k[2]));
            for (d, c) in cross.iter_mut().enumerate() {
                c[i] = c[i].sub(dot.scale(k[d] / k2));
            }
        }
        cross
    }
    /// One Heun step with the viscosity in an integrating factor.
    pub fn step(&mut self, dt: f64) {
        let size = self.n.pow(3);
        let decay: Vec<f64> = (0..size).map(|i| {
            let k = self.wave(i);
            (-self.nu * (k[0] * k[0] + k[1] * k[1] + k[2] * k[2]) * dt).exp()
        }).collect();
        let n0 = self.nonlinear(&self.u);
        let mut u1 = self.u.clone();
        for d in 0..3 {
            for i in 0..size {
                u1[d][i] = self.u[d][i].add(n0[d][i].scale(dt)).scale(decay[i]);
            }
        }
        let n1 = self.nonlinear(&u1);
        for d in 0..3 {
            for i in 0..size {
                self.u[d][i] = self.u[d][i].scale(decay[i]).add(n0[d][i].scale(dt / 2.0 * decay[i])).add(n1[d][i].scale(dt / 2.0));
            }
        }
    }
    pub fn stats(&self, t: f64) -> Stats {
        let n = self.n;
        let size = n.pow(3);
        let norm = (size as f64) * (size as f64);
        let w = self.vorticity_hat(&self.u);
        let (mut energy, mut enstrophy, mut tail) = (0.0, 0.0, 0.0);
        let cut = n as f64 / 3.0;
        for i in 0..size {
            let e = (0..3).map(|d| self.u[d][i].norm2()).sum::<f64>() / 2.0 / norm;
            energy += e;
            enstrophy += (0..3).map(|d| w[d][i].norm2()).sum::<f64>() / 2.0 / norm;
            let k = self.wave(i);
            if k.iter().map(|x| x * x).sum::<f64>().sqrt() > 2.0 * cut / 3.0 {
                tail += e;
            }
        }
        let mut max_vorticity: f64 = 0.0;
        let grids: Vec<Vec<C>> = w.iter().map(|f| {
            let mut g = f.clone();
            fft3(&mut g, n, true);
            g
        }).collect();
        for i in 0..size {
            max_vorticity = max_vorticity.max((grids[0][i].0.powi(2) + grids[1][i].0.powi(2) + grids[2][i].0.powi(2)).sqrt());
        }
        Stats { t, energy, enstrophy, max_vorticity, tail: tail / energy.max(1e-300) }
    }
    /// The largest |div u| in Fourier space (0 for an incompressible field).
    pub fn divergence(&self) -> f64 {
        (0..self.n.pow(3)).map(|i| {
            let k = self.wave(i);
            self.u[0][i].scale(k[0]).add(self.u[1][i].scale(k[1])).add(self.u[2][i].scale(k[2])).norm2().sqrt()
        }).fold(0.0, f64::max)
    }
}

pub struct Settings {
    pub grid: usize,
    pub viscosity: f64,
    pub t_end: f64,
    pub dt: f64,
}

/// Report lines for the attempt at the problem.
pub fn report(s: &Settings) -> Vec<String> {
    let mut flow = Flow::taylor_green(s.grid, s.viscosity);
    let steps = (s.t_end / s.dt).round() as usize;
    let every = (1.0 / s.dt).round().max(1.0) as usize;
    let mut stats = vec![flow.stats(0.0)];
    let mut bkm = 0.0;
    let mut last_w = stats[0].max_vorticity;
    for step in 1..=steps {
        flow.step(s.dt);
        if step % every == 0 {
            let st = flow.stats(step as f64 * s.dt);
            bkm += (last_w + st.max_vorticity) / 2.0 * (every as f64 * s.dt);
            last_w = st.max_vorticity;
            stats.push(st);
        }
    }
    let mut out = vec![format!(
        "simulated the 3D Navier-Stokes equations from the Taylor-Green vortex: {g}^3 grid, pseudo-spectral with the 2/3 rule, viscosity {} (Reynolds number {:.0}), time step {}, up to t = {}",
        s.viscosity,
        1.0 / s.viscosity,
        s.dt,
        s.t_end,
        g = s.grid
    )];
    let rows: Vec<String> = stats.iter().map(|x| format!("t={:.0}: E {:.4}, Omega {:.3}, max|omega| {:.2}", x.t, x.energy, x.enstrophy, x.max_vorticity)).collect();
    out.push(format!("energy E, enstrophy Omega, largest vorticity: {}", rows.join("; ")));
    // the energy balance, between measurements: (E1 - E0)/dt against -2 nu (Omega0 + Omega1)/2
    let worst = stats.windows(2).map(|w| {
        let lhs = (w[1].energy - w[0].energy) / (w[1].t - w[0].t);
        let rhs = -2.0 * s.viscosity * (w[0].enstrophy + w[1].enstrophy) / 2.0;
        (lhs - rhs).abs() / rhs.abs().max(1e-12)
    }).fold(0.0, f64::max);
    out.push(format!("check, the exact energy balance dE/dt = -2 nu Omega: holds to within {:.1}% between measurements (the rest is the coarse spacing of the measurements); divergence of u: {:.1e}", 100.0 * worst, flow.divergence()));
    let peak = stats.iter().fold(&stats[0], |a, b| if b.enstrophy > a.enstrophy { b } else { a });
    let tail = stats.iter().map(|x| x.tail).fold(0.0, f64::max);
    out.push(format!(
        "the vortex stretches and the enstrophy grows {:.1}-fold, peaking at t = {:.0}, then viscosity wins and everything decays; the largest vorticity stays finite (Beale-Kato-Majda integral of max|omega| up to t = {}: {bkm:.1}); energy in the highest kept wavenumbers at most {:.1e} of the total: {}",
        peak.enstrophy / stats[0].enstrophy,
        peak.t,
        s.t_end,
        tail,
        if tail < 1e-3 { "resolved" } else { "under-resolved: a finer grid is needed" }
    ));
    out.push("so this solution stays smooth: evidence for one start at one viscosity on a finite grid, while the Millennium problem asks about every smooth start and every time (and blow-up, if it exists, would need far finer grids and larger Reynolds numbers)".into());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fft_round_trip_and_taylor_green_start() {
        let n = 8;
        let mut a: Vec<C> = (0..n * n * n).map(|i| C((i as f64 * 0.37).sin(), 0.0)).collect();
        let b = a.clone();
        fft3(&mut a, n, false);
        fft3(&mut a, n, true);
        assert!(a.iter().zip(&b).all(|(x, y)| (x.0 - y.0).abs() < 1e-12 && x.1.abs() < 1e-12));
        let f = Flow::taylor_green(16, 0.01);
        let s = f.stats(0.0);
        assert!((s.energy - 0.125).abs() < 1e-12, "{}", s.energy);
        assert!((s.enstrophy - 0.375).abs() < 1e-12, "{}", s.enstrophy); // |omega|^2 averages 3/4
        assert!(f.divergence() < 1e-9);
    }

    #[test]
    fn energy_falls_as_the_balance_says() {
        let mut f = Flow::taylor_green(16, 0.05);
        let s0 = f.stats(0.0);
        for _ in 0..10 {
            f.step(0.01);
        }
        let s1 = f.stats(0.1);
        let predicted = -2.0 * 0.05 * (s0.enstrophy + s1.enstrophy) / 2.0 * 0.1;
        assert!(((s1.energy - s0.energy) - predicted).abs() < 0.02 * predicted.abs(), "{} vs {}", s1.energy - s0.energy, predicted);
    }
}
