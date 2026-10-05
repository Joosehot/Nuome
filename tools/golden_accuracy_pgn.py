"""Accuracy of a Golden Boy game (a PGN from tools/golden_selfplay.py): every
move judged by Stockfish at a fixed depth. For each move: the centipawn loss
from the mover's side (capped at 1000), Lichess-style accuracy from the
win-percentage drop, whether it was Stockfish's move. Reported for white,
black and both, and by what the move rested on (the PGN comments).

Usage: python tools/golden_accuracy_pgn.py [PGN=out/golden/selfplay.pgn] [--depth 18]
"""
import argparse, math

import chess, chess.engine, chess.pgn

SF = r"C:\Users\joose\Desktop\ChessEngine\stockfish\stockfish-windows-x86-64-avx2.exe"


def win(cp):
    return 50 + 50 * (2 / (1 + math.exp(-0.00368208 * cp)) - 1)


def cp_of(score, turn):
    return score.pov(turn).score(mate_score=10000)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("pgn", nargs="?", default="out/golden/selfplay.pgn")
    ap.add_argument("--depth", type=int, default=18)
    a = ap.parse_args()
    game = chess.pgn.read_game(open(a.pgn, encoding="utf-8"))
    eng = chess.engine.SimpleEngine.popen_uci(SF)
    eng.configure({"Threads": 8, "Hash": 512})
    b = game.board()
    rows = []
    info = eng.analyse(b, chess.engine.Limit(depth=a.depth))
    for node in game.mainline():
        mover = b.turn
        before = cp_of(info["score"], mover)
        best = info.get("pv", [None])[0]
        b.push(node.move)
        info = eng.analyse(b, chess.engine.Limit(depth=a.depth)) if not b.is_game_over() else None
        if info is None:
            after = 10000 if b.is_checkmate() else 0
        else:
            after = cp_of(info["score"], mover)
        loss = max(0, min(1000, before - after))
        acc = max(0.0, min(100.0, 103.1668 * math.exp(-0.04354 * max(0.0, win(before) - win(after))) - 3.1669))
        kind = (node.comment or "").split(",")[0] or "?"
        rows.append(("white" if mover else "black", kind, loss, acc, node.move == best))
        if info is None:
            break
    eng.quit()

    def show(name, rs):
        if not rs:
            return
        n = len(rs)
        acpl = sum(r[2] for r in rs) / n
        acc = sum(r[3] for r in rs) / n
        sf = sum(r[4] for r in rs)
        bl = sum(r[2] >= 300 for r in rs)
        mi = sum(100 <= r[2] < 300 for r in rs)
        print(f"  {name:<48} {n:>4} moves  accuracy {acc:5.1f}%  ACPL {acpl:6.1f}  Stockfish's move {sf}/{n}  mistakes {mi}  blunders {bl}")

    print(f"{a.pgn}: {len(rows)} moves judged by Stockfish at depth {a.depth}; result {game.headers.get('Result')}")
    show("white", [r for r in rows if r[0] == "white"])
    show("black", [r for r in rows if r[0] == "black"])
    show("both", rows)
    print("by what the move rested on:")
    for k in sorted({r[1] for r in rows}):
        show(k, [r for r in rows if r[1] == k])


if __name__ == "__main__":
    main()
