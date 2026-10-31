"""The truth for cracking a position's code: positions of up to five pieces with their value for the side to move,
exact from the Syzygy tables (de Man). Written once, never re-drawn:

  bench/crack/smoke.tsv  (generator seed 3, 25 a class): the check, never searched on
  bench/crack/train.tsv  (generator seed 4, 100 a class): where Golden invents
Line: class TAB FEN TAB value (WDL for the side to move: 2 win, 1 cursed win, 0 draw, -1 blessed loss, -2 loss).

Every value is kept (won, drawn and lost positions alike), in each of 20 classes.

Usage: python tools/crack_truth.py --name smoke|train
"""
import argparse, os, sys

import chess, chess.syzygy

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import golden_tablebase as gt

NUOME = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(NUOME, "bench", "crack")
CLASSES = ["KQvK", "KRvK", "KPvK", "KBNvK", "KBBvK", "KNNvK", "KBvK", "KNvK", "KRvKR", "KQvKR",
           "KRvKB", "KRvKN", "KPvKP", "KQvKQ", "KRPvKR", "KQvKP", "KRvKP", "KBPvK", "KPPvK", "KQvKRP"]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--name", default="smoke", choices=["smoke", "train"])
    a = ap.parse_args()
    per_class, seed = {"smoke": (25, 3), "train": (100, 4)}[a.name]
    path = os.path.join(OUT, f"{a.name}.tsv")
    if os.path.exists(path):
        sys.exit(f"{path} exists; a set is written once")
    tb = chess.syzygy.open_tablebase(gt.TB_PATH)
    os.makedirs(OUT, exist_ok=True)
    rows = []
    for cls, fen in gt.generate(CLASSES, per_class, seed, None):
        b = chess.Board(fen)
        if b.is_checkmate():
            v = -2
        elif b.is_stalemate() or b.is_insufficient_material():
            v = 0
        else:
            v = tb.probe_wdl(b)
        rows.append(f"{cls}\t{fen}\t{v}")
    with open(path, "w") as f:
        f.write("\n".join(rows) + "\n")
    print(f"{len(rows)} positions -> {path}")


if __name__ == "__main__":
    main()
