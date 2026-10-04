"""Truth that also asks for speed: for each position, Stockfish with the
Syzygy tables values every legal move; the right moves are those that lose
at most --margin cp against the best (so a slower mate counts as wrong).
Same line format as golden_truth.py (class TAB fen TAB right moves); only
positions where some move is wrong are kept.

Usage: python tools/golden_truth_cp.py OUT [--classes ...] [--per-class 100] [--seed 3] [--margin 20] [--time 0.05] [--jobs 12]
"""
import argparse, multiprocessing as mp, os, sys
import chess, chess.engine

sys.path.insert(0, os.path.dirname(__file__))
import golden_tablebase as gt  # noqa: E402

SF = r"C:\Users\joose\Desktop\ChessEngine\stockfish\stockfish-windows-x86-64-avx2.exe"
CAP = 1000
_sf = None
_cfg = None


def _init(tb, t, margin):
    global _sf, _cfg
    _sf = chess.engine.SimpleEngine.popen_uci(SF)
    _sf.configure({"Threads": 1, "Hash": 64, "SyzygyPath": tb})
    _cfg = (t, margin)


def work(job):
    cls, fen = job
    t, margin = _cfg
    b = chess.Board(fen)
    vals = {}
    for m in b.legal_moves:
        b.push(m)
        if b.is_checkmate():
            v = 30000
        elif b.is_stalemate() or b.is_insufficient_material():
            v = 0
        else:
            v = -_sf.analyse(b, chess.engine.Limit(time=t))["score"].relative.score(mate_score=30000)
        b.pop()
        vals[m.uci()] = v
    best = max(vals.values())
    good = sorted(u for u, v in vals.items() if min(CAP, best - v) <= margin)
    return cls, fen, good, len(vals)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("out")
    ap.add_argument("--classes", default=",".join(gt.DEFAULT_CLASSES))
    ap.add_argument("--per-class", type=int, default=100)
    ap.add_argument("--seed", type=int, default=3)
    ap.add_argument("--margin", type=int, default=20)
    ap.add_argument("--time", type=float, default=0.05)
    ap.add_argument("--jobs", type=int, default=12)
    ap.add_argument("--tb", default=r"C:\Users\joose\Tools\syzygy\345-wdl")
    a = ap.parse_args()
    jobs = gt.generate(a.classes.split(","), a.per_class, a.seed)
    kept = 0
    with mp.Pool(a.jobs, initializer=_init, initargs=(a.tb, a.time, a.margin)) as pool, open(a.out, "w") as f:
        for i, (cls, fen, good, n) in enumerate(pool.imap_unordered(work, jobs), 1):
            if len(good) < n:
                f.write(f"{cls}\t{fen}\t{' '.join(good)}\n")
                kept += 1
            if i % 300 == 0:
                print(f"  {i}/{len(jobs)}", flush=True)
    print(f"{kept} positions with something to get wrong -> {a.out}")


if __name__ == "__main__":
    main()
