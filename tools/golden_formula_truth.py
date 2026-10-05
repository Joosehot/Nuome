"""The truth for the Golden formula question: positions of up to five pieces
whose value V is known exactly (Syzygy), with the value every move keeps.
Written once, never re-drawn:

  bench/golden_formula/search.tsv  (generator seed 1): where the choice is searched
  bench/golden_formula/check.tsv   (generator seed 2): never searched, the check

Only positions where some move fails to keep the value are kept (elsewhere
every move satisfies the question's condition and nothing is tested).
Line: FEN TAB move:value move:value ... (value: WDL for the mover after it).

Usage: python tools/golden_formula_truth.py [--per-class 30]
"""
import argparse, os, sys

import chess, chess.syzygy

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import golden_tablebase as gt

NUOME = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(NUOME, "bench", "golden_formula")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--per-class", type=int, default=30)
    a = ap.parse_args()
    tb = chess.syzygy.open_tablebase(gt.TB_PATH)
    gt._tb = tb
    os.makedirs(OUT, exist_ok=True)
    for name, seed in (("search", 1), ("check", 2)):
        path = os.path.join(OUT, f"{name}.tsv")
        if os.path.exists(path):
            print(f"{path} exists; a set is written once")
            continue
        rows = []
        for cls, fen in gt.generate(gt.DEFAULT_CLASSES[:10], a.per_class, seed, tb):
            b = chess.Board(fen)
            vals = [(m.uci(), gt.child_value(b, m)) for m in b.legal_moves]
            if len({v for _, v in vals}) > 1:
                rows.append(fen + "\t" + " ".join(f"{m}:{v}" for m, v in vals))
        with open(path, "w", newline="\n") as fh:
            fh.write("\n".join(rows) + "\n")
        print(f"{path}: {len(rows)} positions")


if __name__ == "__main__":
    main()
