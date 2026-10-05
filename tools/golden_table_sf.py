"""Stockfish's move for every distinct opening position of Golden Answer's
table (out/golden/table_fens.txt, from `nuome --ideas golden-table-fens`):
one move per position at a fixed depth, so the reference is unambiguous.
Resumable: positions already in the output are skipped.

Output: out/golden/table_sf.tsv (key TAB best move in UCI)

Usage: python tools/golden_table_sf.py [--depth 14] [--jobs 12] [--limit N]
"""
import argparse, multiprocessing as mp, os, time

import chess, chess.engine

NUOME = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SF = r"C:\Users\joose\Desktop\ChessEngine\stockfish\stockfish-windows-x86-64-avx2.exe"
FENS = os.path.join(NUOME, "out", "golden", "table_fens.txt")
OUT = os.path.join(NUOME, "out", "golden", "table_sf.tsv")
_eng = None
_depth = 14


def _init(depth):
    global _eng, _depth
    _depth = depth
    _eng = chess.engine.SimpleEngine.popen_uci(SF)
    _eng.configure({"Threads": 1, "Hash": 16})


def best(job):
    key, fen = job
    r = _eng.play(chess.Board(fen), chess.engine.Limit(depth=_depth))
    return key, r.move.uci()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--depth", type=int, default=14)
    ap.add_argument("--jobs", type=int, default=12)
    ap.add_argument("--limit", type=int, default=0)
    a = ap.parse_args()
    done = set()
    if os.path.exists(OUT):
        done = {l.split("\t")[0] for l in open(OUT) if "\t" in l}
    jobs, seen = [], set(done)
    for line in open(FENS):
        _, key, fen = line.rstrip("\n").split("\t")
        if key not in seen:
            seen.add(key)
            jobs.append((key, fen))
    if a.limit:
        jobs = jobs[: a.limit]
    print(f"{len(done)} done, {len(jobs)} to analyse at depth {a.depth} on {a.jobs} processes", flush=True)
    t0 = time.time()
    with open(OUT, "a", newline="\n") as out, mp.Pool(a.jobs, initializer=_init, initargs=(a.depth,)) as pool:
        for i, (key, mv) in enumerate(pool.imap_unordered(best, jobs, chunksize=16), 1):
            out.write(f"{key}\t{mv}\n")
            if i % 5000 == 0:
                out.flush()
                el = time.time() - t0
                print(f"  {i}/{len(jobs)} in {el:.0f} s, about {el / i * (len(jobs) - i) / 60:.0f} min left", flush=True)
    print(f"done: {len(jobs)} in {time.time() - t0:.0f} s", flush=True)


if __name__ == "__main__":
    main()
