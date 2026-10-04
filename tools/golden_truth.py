"""Exact truth for Nuome's golden-function search, from the Syzygy tables.

Writes one line per position: FEN, TAB, the correct moves (UCI, space
separated) - the moves that keep the position's best WDL value. Only
positions that are not lost and where some legal move is wrong are kept
(elsewhere every move is right and there is nothing to learn).

Usage: python tools/golden_truth.py OUT --classes KQvK,KRvK,... [--per-class 100] [--seed 1]
"""
import argparse, os, sys
import chess, chess.syzygy

sys.path.insert(0, os.path.dirname(__file__))
import golden_tablebase as gt  # noqa: E402


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("out")
    ap.add_argument("--classes", required=True)
    ap.add_argument("--per-class", type=int, default=100)
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--tb", default=r"C:\Users\joose\Tools\syzygy\345-wdl")
    a = ap.parse_args()
    tb = chess.syzygy.open_tablebase(a.tb)
    gt._tb = tb
    jobs = gt.generate(a.classes.split(","), a.per_class, a.seed, tb)
    kept = 0
    with open(a.out, "w") as f:
        for cls, fen in jobs:
            b = chess.Board(fen)
            if gt.wdl(b) == -2:
                continue
            vals = {m: gt.child_value(b, m) for m in b.legal_moves}
            best = max(vals.values())
            good = [m.uci() for m, v in vals.items() if v == best]
            if len(good) == len(vals):
                continue
            f.write(f"{cls}\t{fen}\t{' '.join(sorted(good))}\n")
            kept += 1
    print(f"{kept} positions with something to get wrong -> {a.out}")


if __name__ == "__main__":
    main()
