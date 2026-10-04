"""bench100 for the golden function on a full board: the 100 positions of
ChessEngine's eval_bucket_100_qs.json (Stockfish's best move known), the
lumberjack with a time budget per move, Stockfish-move hits, big errors
(loss >= 100 cp) and total loss capped at 500 cp per position - the same
measure as ChessEngine's bench100.py, so the numbers compare (StarNet 1 s:
76/100, 6 big errors, loss 1901).

Usage: python tools/golden_bench100.py [--ms 1000] [--jobs 6]
"""
import argparse, json, multiprocessing as mp, os, re, subprocess, time
import chess, chess.engine

CE = r"C:\Users\joose\Desktop\ChessEngine"
SF = os.path.join(CE, "stockfish", "stockfish-windows-x86-64-avx2.exe")
EXE = r"C:\Users\joose\Nuome\target\release\nuome.exe"
NUOME = r"C:\Users\joose\Nuome"
MOVE_RE = re.compile(r"^move: (\S+) \(", re.M)
_ms = 1000


def _init(ms):
    global _ms
    _ms = ms


def play(row):
    name, best, fen = row[0], row[1], row[-1]
    env = dict(os.environ, GOLDEN_TIME_MS=str(_ms))
    out = subprocess.run([EXE, "--ideas", "golden", fen], cwd=NUOME, capture_output=True, text=True, env=env, timeout=600).stdout
    m = MOVE_RE.search(out)
    return name, best, fen, (m.group(1) if m else None)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ms", type=int, default=1000)
    ap.add_argument("--jobs", type=int, default=6)
    a = ap.parse_args()
    rows = json.load(open(os.path.join(CE, "eval_bucket_100_qs.json")))
    cache_f = os.path.join(CE, "bucket100_cache.json")
    try:
        cache = json.load(open(cache_f))
    except Exception:
        cache = {}
    t0 = time.time()
    with mp.Pool(a.jobs, initializer=_init, initargs=(a.ms,)) as pool:
        results = pool.map(play, rows)
    sf = chess.engine.SimpleEngine.popen_uci(SF)
    sf.configure({"Threads": 8, "Hash": 256})

    def sfcp(fen, san):
        k = fen + "|" + san
        if k not in cache:
            b = chess.Board(fen)
            cache[k] = sf.analyse(b, chess.engine.Limit(time=1), root_moves=[b.parse_san(san)])["score"].relative.score(mate_score=10000)
        return cache[k]

    hit = big = tot = 0
    misses = []
    for name, best, fen, uci in results:
        b = chess.Board(fen)
        if uci is None:
            misses.append({"name": name, "best": best, "played": None, "loss": 500, "fen": fen})
            tot += 500
            big += 1
            continue
        san = b.san(chess.Move.from_uci(uci))
        if san == best:
            hit += 1
            continue
        loss = max(0, sfcp(fen, best) - sfcp(fen, san))
        tot += min(loss, 500)
        big += loss >= 100
        misses.append({"name": name, "best": best, "played": san, "loss": loss, "fen": fen})
    sf.quit()
    json.dump(cache, open(cache_f, "w"))
    json.dump(misses, open(os.path.join(NUOME, "out", "golden", "bench100_misses.json"), "w"), indent=1)
    print(f"lumberjack {a.ms} ms/move: SF move {hit}/100, big errors (>= 100 cp) {big}, loss (max 500/position) {tot}  ({time.time() - t0:.0f} s)")
    for m in sorted(misses, key=lambda m: -m["loss"])[:8]:
        print(f"  {m['name']:<24} best {m['best']:<7} played {m['played']}  loss {m['loss']}")


if __name__ == "__main__":
    main()
