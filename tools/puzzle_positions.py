"""Stockfish-checked puzzle positions for the discovering genius: every move
of the solver in a Lichess puzzle's solution line (hf_data/lichess_db_puzzle.csv.zst,
downloaded from database.lichess.org; every solution is verified by Stockfish),
one JSON line {"fen", "move"} each. The puzzles tools/golden_puzzles.py tests
on are left out, so the test stays unseen.

Usage: python tools/puzzle_positions.py [--positions 300000] [--out out/golden/puzzle_positions.jsonl]
"""
import argparse, csv, io, json, sys
import chess, zstandard

sys.path.insert(0, __file__.rsplit("\\", 1)[0].rsplit("/", 1)[0])
from golden_puzzles import DB, DEFAULT_THEMES, pick


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--positions", type=int, default=300000)
    ap.add_argument("--out", default="out/golden/puzzle_positions.jsonl")
    a = ap.parse_args()
    # the test's puzzles (the first 30 a theme: the tuning set and the fresh set), left out
    test = {pid for v in pick(DEFAULT_THEMES, 30).values() for pid, *_ in v}
    n = 0
    with open(DB, "rb") as fh, open(a.out, "w") as out:
        reader = csv.reader(io.TextIOWrapper(zstandard.ZstdDecompressor().stream_reader(fh), encoding="utf-8"))
        next(reader)
        for row in reader:
            if row[0] in test:
                continue
            b = chess.Board(row[1])
            moves = row[2].split()
            for i, mv in enumerate(moves):
                if i % 2 == 1:  # the solver's moves; the first is the opponent's
                    out.write(json.dumps({"fen": b.fen(), "move": mv}) + "\n")
                    n += 1
                b.push_uci(mv)
            if n >= a.positions:
                break
    print(f"{n} puzzle positions -> {a.out} ({len(test)} test puzzles left out)")


if __name__ == "__main__":
    main()
