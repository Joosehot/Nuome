"""Golden Boy plays Stockfish and writes the Golden Function longer, streamed.

Golden Boy: the opening table when the position is familiar, the Syzygy
tables at five pieces or fewer, otherwise the growing Golden Function at 1
ply (`nuome --ideas goldenboy-grow`): the net's 64 points (value, side,
attackers) combined by the terms the function chose itself. Stockfish: 1 s a
move. At every Golden Boy position Stockfish says its own move (1 s): a
lesson, and a point when Golden Boy played it; Stockfish's own moves are
lessons too. After every game the function grows (`golden-grow`, twice).

Streamed to http://localhost:5000/live. Log: out/golden/grow_games.log;
games: out/golden/grow_games.pgn; lessons: out/golden/grow_lessons.tsv;
the function: out/golden/grow_function.txt.

Usage: python tools/goldenboy_grow_vs_sf.py [--games 0 (= until stopped)] [--sf 1]
"""
import argparse, datetime, os, re, subprocess, time

import chess, chess.engine, chess.pgn, chess.syzygy, requests

NUOME = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
EXE = os.path.join(NUOME, "target", "release", "nuome.exe")
SF = r"C:\Users\joose\Desktop\ChessEngine\stockfish\stockfish-windows-x86-64-avx2.exe"
TB = r"C:\Users\joose\Tools\syzygy\345-wdl"
SERVER = "http://localhost:5000"
OUT = os.path.join(NUOME, "out", "golden")
MOVE = re.compile(r"^move: (\S+) \((.*)\)", re.M)


def push_live(board, moves, white, black, result=None, active=True):
    try:
        requests.post(f"{SERVER}/live_game", json={"fen": board.fen(), "moves": moves, "white": white, "black": black, "result": result, "active": active}, timeout=1)
    except Exception:
        pass


def tb_move(board, tb):
    best, best_v = None, -9
    for m in board.legal_moves:
        board.push(m)
        if board.is_checkmate():
            v = 2
        elif board.is_stalemate() or len(board.piece_map()) == 2:
            v = 0
        else:
            v = -tb.probe_wdl(board)
        board.pop()
        if v > best_v:
            best, best_v = m, v
    return best


def gb_move(board, tb):
    if len(board.piece_map()) <= 5:
        return tb_move(board, tb), "the Syzygy tables"
    out = subprocess.run([EXE, "--ideas", "goldenboy-grow", board.fen()], cwd=NUOME, capture_output=True, text=True, timeout=600).stdout
    m = MOVE.search(out)
    if not m:
        return None, out.strip()[:200]
    return chess.Move.from_uci(m.group(1)), m.group(2)


def log(line):
    print(line, flush=True)
    with open(os.path.join(OUT, "grow_games.log"), "a", encoding="utf-8") as f:
        f.write(line + "\n")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--games", type=int, default=0)
    ap.add_argument("--sf", type=float, default=1.0)
    ap.add_argument("--max-plies", type=int, default=300)
    a = ap.parse_args()
    sf = chess.engine.SimpleEngine.popen_uci(SF)
    sf.configure({"Threads": 4, "Hash": 256})
    tb = chess.syzygy.open_tablebase(TB)
    score = {"win": 0, "draw": 0, "loss": 0}
    g = 0
    while a.games == 0 or g < a.games:
        g += 1
        gb_white = g % 2 == 1
        white, black = ("Golden Boy", "Stockfish") if gb_white else ("Stockfish", "Golden Boy")
        b = chess.Board()
        moves, lessons, how_count = [], [], {}
        asked = points = 0
        t0 = time.time()
        while not b.is_game_over(claim_draw=True) and len(moves) < a.max_plies:
            if (b.turn == chess.WHITE) == gb_white:
                m, how = gb_move(b, tb)
                if m is None or m not in b.legal_moves:
                    log(f"game {g}: Golden Boy gave no legal move ({how}); the game stops")
                    break
                key = how.split(" (")[0]
                how_count[key] = how_count.get(key, 0) + 1
                if key == "the Golden Function":
                    sfm = sf.play(b, chess.engine.Limit(time=a.sf)).move
                    lessons.append(f"{b.fen()}\t{sfm.uci()}")
                    asked += 1
                    points += m == sfm
            else:
                m = sf.play(b, chess.engine.Limit(time=a.sf)).move
                lessons.append(f"{b.fen()}\t{m.uci()}")
            moves.append(b.san(m))
            b.push(m)
            push_live(b, moves, white, black)
        res = b.result(claim_draw=True) if b.is_game_over(claim_draw=True) else "1/2-1/2"
        gb_res = {"1-0": "win" if gb_white else "loss", "0-1": "loss" if gb_white else "win"}.get(res, "draw")
        score[gb_res] += 1
        push_live(b, moves, white, black, result=res, active=False)
        game = chess.pgn.Game.from_board(b)
        game.headers.update({"White": white, "Black": black, "Result": res, "Date": datetime.date.today().isoformat(), "Round": str(g)})
        with open(os.path.join(OUT, "grow_games.pgn"), "a", encoding="utf-8") as f:
            print(game, file=f, end="\n\n")
        with open(os.path.join(OUT, "grow_lessons.tsv"), "a", encoding="utf-8", newline="\n") as f:
            f.write("".join(r + "\n" for r in lessons))
        grown = []
        for _ in range(2):
            out = subprocess.run([EXE, "--ideas", "golden-grow"], cwd=NUOME, capture_output=True, text=True).stdout.strip().splitlines()
            grown.append(out[0] if out else "(no output)")
        log(f"game {g} ({white} - {black}): {res}, Golden Boy {gb_res}, {len(moves)} plies, {time.time() - t0:.0f} s; moves by {how_count}; points {points}/{asked}; total +{score['win']} ={score['draw']} -{score['loss']}")
        for line in grown:
            log("  " + line)
    sf.quit()


if __name__ == "__main__":
    main()
