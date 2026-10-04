"""Add terms to the golden function until every class loses less than the
target cp in the 1200-position test (golden_cploss.py, seed 1).

Each round: nuome --ideas golden-polish adds terms on all the truth files
(never the test positions) until no term helps - no limit on how many, no
tuning in between; the 1200 positions are measured
per class; if any class is still at or above the target, a new truth file
(a new seed, speed-aware: golden_truth_cp.py) joins the data and the next
round starts. When every class is below the target, every weight is tuned
together once (golden-polish tune) and the test runs again. Progress goes to
stdout as it happens.

Usage: python tools/golden_loop.py [--target 20] [--rounds 10]
"""
import argparse, os, re, subprocess, sys, time

NUOME = r"C:\Users\joose\Nuome"
EXE = os.path.join(NUOME, "target", "release", "nuome.exe")
PY = sys.executable


def run(cmd):
    p = subprocess.run(cmd, cwd=NUOME, capture_output=True, text=True)
    return p.stdout + p.stderr


def terms_now():
    try:
        return sum(1 for l in open(os.path.join(NUOME, "out", "golden", "terms.txt")) if l.strip())
    except OSError:
        return 0


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--target", type=float, default=20.0)
    ap.add_argument("--rounds", type=int, default=10)
    a = ap.parse_args()
    truth = ["out/golden/truth_cp_s3.txt", "out/golden/truth_cp_s4.txt"]
    seed = 5
    t0 = time.time()
    for r in range(1, a.rounds + 1):
        bad = None
        # the supergenius's reasoned terms first; the blind search only when they are not enough
        for how in ["supergenius", "blind"]:
            before = terms_now()
            out = run([EXE, "--ideas", "golden-polish"] + (["supergenius"] if how == "supergenius" else []) + truth)
            found = [l.strip() for l in out.splitlines() if l.strip().startswith("term ")]
            print(f"round {r} ({how}): {terms_now()} terms (was {before}); new: {len(found)}", flush=True)
            for l in found:
                print("    " + l, flush=True)
            m = run([PY, "tools/golden_cploss.py", "--per-class", "100", "--seed", "1", "--exe", EXE])
            rows = re.findall(r"^(K\S+)\s+(\d+)\s+([\d.]+)\s+(\d+)$", m, re.M)
            avg = re.search(r"average cp loss: ([\d.]+)", m)
            bad = [(c, float(v)) for c, _, v, _ in rows if float(v) >= a.target]
            print(f"  1200 test: average {avg.group(1) if avg else '?'} cp; classes at or above {a.target:g}: "
                  + (", ".join(f"{c} {v:.1f}" for c, v in bad) or "none") + f"  ({time.time() - t0:.0f} s)", flush=True)
            if not bad:
                break
        if not bad:
            print("every class is below the target; now every weight is tuned together", flush=True)
            run([EXE, "--ideas", "golden-polish", "tune"] + truth)
            m = run([PY, "tools/golden_cploss.py", "--per-class", "100", "--seed", "1", "--exe", EXE])
            print("\n".join(m.strip().splitlines()[-16:]), flush=True)
            return
        path = f"out/golden/truth_cp_s{seed}.txt"
        print("  " + run([PY, "tools/golden_truth_cp.py", path, "--seed", str(seed)]).strip().splitlines()[-1], flush=True)
        truth.append(path)
        seed += 1
    print("rounds used up", flush=True)


if __name__ == "__main__":
    main()
