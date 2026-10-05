"""The Golden formula checked exactly where the truth is known: positions of
up to five pieces, judged by the Syzygy tables. The formula's move is right
when it keeps the position's best result (win stays win, draw stays at least
a draw). Lost positions are left out (every move loses). The check chooses
nothing in the formula; it says whether it holds so far, and the first
counterexample.

Positions: tools/golden_tablebase.py's generator, --seed, --per-class a class.

Usage: python tools/golden_formula_check.py [--per-class 10] [--seed 1]
"""
import argparse, os, subprocess, sys, tempfile

import chess, chess.syzygy

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import golden_tablebase as gt

NUOME = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
EXE = os.path.join(NUOME, "target", "release", "nuome.exe")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--per-class", type=int, default=10)
    ap.add_argument("--seed", type=int, default=1)
    a = ap.parse_args()
    tb = chess.syzygy.open_tablebase(gt.TB_PATH)
    gt._tb = tb
    classes = gt.DEFAULT_CLASSES[:10]
    jobs = [(c, f) for c, f in gt.generate(classes, a.per_class, a.seed, tb) if gt.wdl(chess.Board(f)) > -2]
    with tempfile.NamedTemporaryFile("w", suffix=".txt", delete=False) as fh:
        fh.write("\n".join(f for _, f in jobs))
        path = fh.name
    out = subprocess.run([EXE, "--ideas", "golden-formula", path], capture_output=True, text=True, cwd=NUOME).stdout.split()
    os.unlink(path)
    per, first = {}, None
    for (cls, fen), mv in zip(jobs, out):
        b = chess.Board(fen)
        best = max(gt.child_value(b, m) for m in b.legal_moves)
        ok = mv != "none" and gt.child_value(b, chess.Move.from_uci(mv)) == best
        per.setdefault(cls, []).append(ok)
        if not ok and first is None:
            first = (cls, fen, mv, best, gt.child_value(b, chess.Move.from_uci(mv)) if mv != "none" else None)
    print(f"The Golden formula on {len(jobs)} Syzygy positions (not lost; generator seed {a.seed}, {a.per_class} a class):")
    for cls in classes:
        r = per.get(cls, [])
        print(f"  {cls:<7} {sum(r)}/{len(r)}")
    print(f"  all     {sum(sum(r) for r in per.values())}/{len(jobs)}")
    if first:
        cls, fen, mv, best, got = first
        print(f"first counterexample: {cls} {fen}: the formula plays {mv} (result {got}), the best keeps {best}")
    else:
        print("no counterexample: holds on every position checked")


if __name__ == "__main__":
    main()
