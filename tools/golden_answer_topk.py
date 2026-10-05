"""Golden Answer's chess answer (nuome --answer theoretical "best move in <FEN>")
against Stockfish's best moves: how often it is Stockfish's first move, in its
top 3, in its top 5. Positions: Stockfish's own moves in CCRL games never used
for cloning Golden heart (the last 5% of out/golden/ccrl_sf_games.txt), a
fixed sample. Stockfish's ranking: multipv 5 at a fixed depth.

Usage: python tools/golden_answer_topk.py [--n 10] [--depth 18]
"""
import argparse, os, re, subprocess, sys

import chess, chess.engine

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from golden_sfmatch import sample, EXE, NUOME

SF = r"C:\Users\joose\Desktop\ChessEngine\stockfish\stockfish-windows-x86-64-avx2.exe"
ANSWER = re.compile(r"^answer: (\S+)", re.M)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--n", type=int, default=10)
    ap.add_argument("--depth", type=int, default=18)
    a = ap.parse_args()
    eng = chess.engine.SimpleEngine.popen_uci(SF)
    eng.configure({"Threads": 8, "Hash": 512})
    top = [0, 0, 0]
    for fen, played in sample(a.n):
        out = subprocess.run([EXE, "--answer", "theoretical", f"best move in {fen}"], capture_output=True, text=True, encoding="utf-8", cwd=NUOME).stdout
        m = ANSWER.search(out)
        mv = m.group(1) if m else None
        d = re.search(r"search of (\d+) plies .*?; (\d+) positions", out)
        reach = f"depth {d.group(1)}, {int(d.group(2)):,} positions" if d else "no search (tables or rules)"
        infos = eng.analyse(chess.Board(fen), chess.engine.Limit(depth=a.depth), multipv=5)
        ranked = [i["pv"][0].uci() for i in infos if "pv" in i]
        rank = ranked.index(mv) + 1 if mv in ranked else None
        for k, lim in enumerate((1, 3, 5)):
            top[k] += rank is not None and rank <= lim
        print(f"  {fen}\n    Golden Answer {mv}, Stockfish {' '.join(ranked)} -> {'rank ' + str(rank) if rank else 'not in top 5'}  [{reach}]")
    eng.quit()
    n = a.n
    print(f"\nGolden Answer's move: Stockfish's best {top[0]}/{n}, in its top 3 {top[1]}/{n}, in its top 5 {top[2]}/{n} (depth {a.depth})")


if __name__ == "__main__":
    main()
