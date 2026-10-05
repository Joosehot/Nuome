"""Golden Boy plays Stockfish until it learns, streamed to the live page.

Golden Boy (5 s a move): the opening table when the position is familiar,
the Syzygy tables at five pieces or fewer, otherwise Goldennet with every
line searched as deep as the time allows (`nuome --ideas goldenboy-net`).
Stockfish: 1 s a move. Colours alternate. After every game Goldennet learns
Stockfish's moves from all the games so far (`goldennet-learn`) and is
frozen again (the net before the games: out/golden/goldennet_before_sf_games.txt).

Streamed to http://localhost:5000/live (ChessEngine/server.py).
Log: out/golden/gb_vs_sf.log; games: out/golden/gb_vs_sf.pgn;
Stockfish's moves: out/golden/gb_sf_moves.tsv.

Usage: python tools/goldenboy_vs_sf.py [--games 0 (= until stopped)] [--gb 5] [--sf 1]
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


def gb_move(board, secs, tb):
    if len(board.piece_map()) <= 5:
        return tb_move(board, tb), "the Syzygy tables"
    out = subprocess.run([EXE, "--ideas", "goldenboy-net", board.fen(), str(secs)], cwd=NUOME, capture_output=True, text=True, timeout=600).stdout
    m = MOVE.search(out)
    if not m:
        return None, out.strip()[:200]
    return chess.Move.from_uci(m.group(1)), m.group(2)


def log(line):
    print(line, flush=True)
    with open(os.path.join(OUT, "gb_vs_sf.log"), "a", encoding="utf-8") as f:
        f.write(line + "\n")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--games", type=int, default=0)
    ap.add_argument("--gb", type=float, default=5.0)
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
        moves, sf_rows, how_count = [], [], {}
        t0 = time.time()
        while not b.is_game_over(claim_draw=True) and len(moves) < a.max_plies:
            if (b.turn == chess.WHITE) == gb_white:
                m, how = gb_move(b, a.gb, tb)
                if m is None or m not in b.legal_moves:
                    log(f"game {g}: Golden Boy gave no legal move ({how}); stopping the game")
                    break
                key = how.split(",")[0]
                how_count[key] = how_count.get(key, 0) + 1
            else:
                m = sf.play(b, chess.engine.Limit(time=a.sf)).move
                sf_rows.append(f"{b.fen()}\t{m.uci()}")
            moves.append(b.san(m))
            b.push(m)
            push_live(b, moves, white, black)
        res = b.result(claim_draw=True) if b.is_game_over(claim_draw=True) else "1/2-1/2"
        gb_res = {"1-0": "win" if gb_white else "loss", "0-1": "loss" if gb_white else "win"}.get(res, "draw")
        score[gb_res] += 1
        push_live(b, moves, white, black, result=res, active=False)
        game = chess.pgn.Game.from_board(b)
        game.headers.update({"White": white, "Black": black, "Result": res, "Date": datetime.date.today().isoformat(), "Round": str(g)})
        with open(os.path.join(OUT, "gb_vs_sf.pgn"), "a", encoding="utf-8") as f:
            print(game, file=f, end="\n\n")
        with open(os.path.join(OUT, "gb_sf_moves.tsv"), "a", encoding="utf-8", newline="\n") as f:
            f.write("".join(r + "\n" for r in sf_rows))
        learned = subprocess.run([EXE, "--ideas", "goldennet-learn", os.path.join(OUT, "gb_sf_moves.tsv"), "1"], cwd=NUOME, capture_output=True, text=True).stdout.strip()
        log(f"game {g} ({white} - {black}): {res}, Golden Boy {gb_res}, {len(moves)} plies, {time.time() - t0:.0f} s; Golden Boy's moves by {how_count}; total +{score['win']} ={score['draw']} -{score['loss']}; {learned}")
    sf.quit()


if __name__ == "__main__":
    main()
