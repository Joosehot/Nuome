"""How often Golden Boy's play (nuome --ideas golden, Golden Answer's search tree)
plays Stockfish's move, on Stockfish's own moves in CCRL games never used for
cloning the heart (the last 5% of out/golden/ccrl_sf_games.txt), a fixed sample.

Usage: python tools/golden_sfmatch.py [--n 200] [--leaf heart|material] [--jobs 6]
"""
import argparse, os, random, re, subprocess
from concurrent.futures import ThreadPoolExecutor

import chess

NUOME = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
EXE = os.path.join(NUOME, "target", "release", "nuome.exe")
GAMES = os.path.join(NUOME, "out", "golden", "ccrl_sf_games.txt")
MOVE = re.compile(r"^move: (\S+) \(([^,]+)", re.M)


def sample(n, seed=7):
    lines = [l for l in open(GAMES) if l.strip()]
    check = lines[len(lines) * 95 // 100:]
    rng = random.Random(seed)
    out = []
    while len(out) < n:
        sf, _, moves = rng.choice(check).rstrip("\n").split("\t")
        moves = moves.split()
        b = chess.Board()
        plies = [i for i in range(len(moves)) if (("W" in sf) if i % 2 == 0 else ("B" in sf))]
        i = rng.choice(plies)
        for m in moves[:i]:
            b.push_uci(m)
        if b.legal_moves.count() > 1:
            out.append((b.fen(), moves[i]))
    return out


def ask(fen, leaf):
    env = dict(os.environ, GOLDEN_LEAF=leaf)
    p = subprocess.run([EXE, "--ideas", "golden", fen], capture_output=True, text=True, encoding="utf-8", env=env, cwd=NUOME, timeout=300)
    m = MOVE.search(p.stdout)
    return (m.group(1), m.group(2)) if m else (None, None)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--n", type=int, default=200)
    ap.add_argument("--leaf", default="heart")
    ap.add_argument("--jobs", type=int, default=6)
    a = ap.parse_args()
    s = sample(a.n)
    with ThreadPoolExecutor(a.jobs) as ex:
        res = list(ex.map(lambda t: ask(t[0], a.leaf), s))
    right = sum(m == sf for (m, _), (_, sf) in zip(res, s))
    kinds = {}
    for (m, k), (_, sf) in zip(res, s):
        r = kinds.setdefault(k, [0, 0])
        r[0] += 1
        r[1] += m == sf
    print(f"leaf {a.leaf}: Stockfish's move in {right}/{len(s)} = {100 * right / len(s):.1f}% (unheard CCRL games, fixed sample)")
    for k, (n, r) in sorted(kinds.items(), key=lambda x: -x[1][0]):
        print(f"  {k}: {r}/{n}")


if __name__ == "__main__":
    main()
