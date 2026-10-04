"""Stockfish against itself: the games the discovering genius learns its
patterns from. Each game starts from a different short opening (the first
few plies played at random among Stockfish's top moves so the games differ),
then 100 ms a move, until the rules end it or 200 plies. One game per line
in out/golden/sf_games.txt: result TAB uci moves, plus a PGN.

Usage: python tools/sf_games.py [--games 100] [--ms 100] [--seed 1]
"""
import argparse, random, time
import chess, chess.engine, chess.pgn

SF = r"C:\Users\joose\Desktop\ChessEngine\stockfish\stockfish-windows-x86-64-avx2.exe"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--games", type=int, default=100)
    ap.add_argument("--ms", type=int, default=100)
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--out", default="out/golden/sf_games.txt")
    a = ap.parse_args()
    rng = random.Random(a.seed)
    eng = chess.engine.SimpleEngine.popen_uci(SF)
    eng.configure({"Threads": 4, "Hash": 256})
    t0 = time.time()
    with open(a.out, "w") as f, open(a.out.replace(".txt", ".pgn"), "w") as pgn:
        for g in range(a.games):
            b = chess.Board()
            # a random opening of 4-8 plies among Stockfish's 3 best moves
            for _ in range(rng.randint(4, 8)):
                infos = eng.analyse(b, chess.engine.Limit(time=0.05), multipv=3)
                moves = [i["pv"][0] for i in infos if "pv" in i]
                if not moves:
                    break
                b.push(rng.choice(moves))
            while not b.is_game_over(claim_draw=True) and len(b.move_stack) < 200:
                r = eng.play(b, chess.engine.Limit(time=a.ms / 1000))
                b.push(r.move)
            result = b.result(claim_draw=True)
            f.write(f"{result}\t{' '.join(m.uci() for m in b.move_stack)}\n")
            f.flush()
            game = chess.pgn.Game.from_board(b)
            game.headers.update({"Event": "Stockfish self-play for Golden Boy", "Round": str(g + 1), "White": "Stockfish", "Black": "Stockfish", "Result": result})
            print(game, file=pgn, end="\n\n")
            if (g + 1) % 10 == 0:
                print(f"  {g + 1}/{a.games} games ({time.time() - t0:.0f} s)", flush=True)
    eng.quit()
    print(f"{a.games} games -> {a.out} ({time.time() - t0:.0f} s)")


if __name__ == "__main__":
    main()
