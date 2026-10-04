"""Ten puzzles a theme from the Lichess puzzle database (hf_data/
lichess_db_puzzle.csv.zst): Golden Boy plays the puzzle position (after the
opponent's first move) and must find the first solution move. Reports per
theme, and for a miss what it played and why.

Usage: python tools/golden_puzzles.py [--per-theme 10] [--ms 1000] [--themes mateIn2,fork,...]
"""
import argparse, csv, io, os, re, subprocess
import chess, zstandard

DB = r"C:\Users\joose\Desktop\ChessEngine\hf_data\lichess_db_puzzle.csv.zst"
NUOME = r"C:\Users\joose\Nuome"
EXE = os.path.join(NUOME, "target", "release", "nuome.exe")
MOVE_RE = re.compile(r"^move: (\S+)", re.M)
DEFAULT_THEMES = ["mateIn1", "mateIn2", "fork", "pin", "hangingPiece", "defensiveMove", "pawnEndgame", "rookEndgame", "endgame", "opening"]


def pick(themes, per, max_rating=1800):
    want = {t: [] for t in themes}
    with open(DB, "rb") as fh:
        reader = csv.reader(io.TextIOWrapper(zstandard.ZstdDecompressor().stream_reader(fh), encoding="utf-8"))
        next(reader)
        for row in reader:
            pid, fen, moves, rating = row[0], row[1], row[2], int(row[3])
            if rating > max_rating:
                continue
            ts = row[7].split()
            for t in themes:
                if t in ts and len(want[t]) < per:
                    want[t].append((pid, fen, moves.split(), rating))
            if all(len(v) >= per for v in want.values()):
                break
    return want


def ask(fen, ms):
    env = dict(os.environ, GOLDEN_TIME_MS=str(ms))
    p = subprocess.run([EXE, "--ideas", "golden", fen], cwd=NUOME, capture_output=True, text=True, env=env, timeout=300)
    m = MOVE_RE.search(p.stdout)
    why = next((l.strip() for l in p.stdout.splitlines() if "because:" in l), "")
    return (m.group(1) if m else None), why


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--per-theme", type=int, default=10)
    ap.add_argument("--ms", type=int, default=1000)
    ap.add_argument("--themes", default=",".join(DEFAULT_THEMES))
    a = ap.parse_args()
    themes = a.themes.split(",")
    want = pick(themes, a.per_theme)
    total = [0, 0]
    for t in themes:
        right = 0
        misses = []
        for pid, fen, moves, rating in want[t]:
            b = chess.Board(fen)
            b.push_uci(moves[0])  # the opponent's move; the solution starts with moves[1]
            mv, why = ask(b.fen(), a.ms)
            ok = mv == moves[1]
            right += ok
            if not ok:
                misses.append(f"    {pid} ({rating}): played {mv}, solution {moves[1]}  {why[:120]}")
        print(f"{t}: {right}/{len(want[t])}")
        for m in misses[:3]:
            print(m)
        total[0] += right
        total[1] += len(want[t])
    print(f"\nall: {total[0]}/{total[1]}")


if __name__ == "__main__":
    main()
