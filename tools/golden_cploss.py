"""Average centipawn loss of the golden function on the tablebase positions.

The same positions as tools/golden_tablebase.py (same classes, seed and
generator). For each position the engine plays its move; Stockfish, with the
Syzygy tables, values the position (its best) and the position after the
move; loss = best - played, from the mover's side, capped at 1000 cp (a won
position thrown to a draw costs the cap). Reports the mean loss overall, over
the testable (not lost) positions, and per class.

Usage: python tools/golden_cploss.py [--per-class 100] [--depth 10] [--exe PATH] [--jobs 12] [--sf-time 0.1]
"""
import argparse, multiprocessing as mp, os, re, subprocess, sys, time
import chess, chess.engine

sys.path.insert(0, os.path.dirname(__file__))
import golden_tablebase as gt  # noqa: E402

SF = r"C:\Users\joose\Desktop\ChessEngine\stockfish\stockfish-windows-x86-64-avx2.exe"
CAP = 1000
MOVE_RE = re.compile(r"^move: (\S+) \(", re.M)
_sf = None
_cfg = None


def _init(exe, depth, tb, sf_time):
    global _sf, _cfg
    _sf = chess.engine.SimpleEngine.popen_uci(SF)
    _sf.configure({"Threads": 1, "Hash": 64, "SyzygyPath": tb})
    _cfg = (exe, depth, sf_time)


def score(board, t):
    info = _sf.analyse(board, chess.engine.Limit(time=t))
    return info["score"].relative.score(mate_score=30000)


def work(job):
    cls, fen = job
    exe, depth, t = _cfg
    board = chess.Board(fen)
    try:
        out = subprocess.run([exe, "--ideas", "golden", fen, str(depth)], cwd=gt.NUOME, capture_output=True, text=True, timeout=120).stdout
    except subprocess.TimeoutExpired:
        return cls, fen, None, None
    m = MOVE_RE.search(out)
    if not m:
        return cls, fen, None, None
    move = chess.Move.from_uci(m.group(1))
    best = score(board, t)
    board.push(move)
    played = -score(board, t)
    return cls, fen, m.group(1), min(CAP, max(0, best - played)), best


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--per-class", type=int, default=100)
    ap.add_argument("--depth", type=int, default=10)
    ap.add_argument("--exe", default=r"C:\Users\joose\Desktop\ChessEngine\steldens-rs\target\release\steldens.exe")
    ap.add_argument("--jobs", type=int, default=12)
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--sf-time", type=float, default=0.1)
    ap.add_argument("--tb", default=r"C:\Users\joose\Tools\syzygy\345-wdl")
    ap.add_argument("--classes", default=",".join(gt.DEFAULT_CLASSES))
    a = ap.parse_args()
    t0 = time.time()
    classes = a.classes.split(",")
    jobs = gt.generate(classes, a.per_class, a.seed)
    with mp.Pool(a.jobs, initializer=_init, initargs=(a.exe, a.depth, a.tb, a.sf_time)) as pool:
        rows = []
        for i, r in enumerate(pool.imap_unordered(work, jobs), 1):
            rows.append(r)
            if i % 100 == 0:
                print(f"  {i}/{len(jobs)} positions ({time.time() - t0:.0f} s)", flush=True)
    per = {}
    for r in rows:
        if r[3] is None:
            continue
        per.setdefault(r[0], []).append(r)
    allr = [r for v in per.values() for r in v]
    errors = sum(1 for r in rows if r[3] is None)
    print(f"golden function (engine {os.path.basename(a.exe)}, {a.depth} plies) on {len(allr)} tablebase positions, Stockfish {a.sf_time}s + Syzygy, loss capped at {CAP} cp")
    print(f"{'class':<10}{'pos':>5}{'mean loss':>11}{'perfect':>9}")
    for cls in classes:
        v = per.get(cls, [])
        if v:
            print(f"{cls:<10}{len(v):>5}{sum(r[3] for r in v) / len(v):>11.1f}{sum(r[3] == 0 for r in v):>9}")
    print(f"\naverage cp loss: {sum(r[3] for r in allr) / len(allr):.1f} cp over all {len(allr)} positions")
    print(f"positions with 0 loss: {sum(r[3] == 0 for r in allr)} / {len(allr)}; errors (no move / timeout): {errors}; ({time.time() - t0:.0f} s)")


if __name__ == "__main__":
    main()
