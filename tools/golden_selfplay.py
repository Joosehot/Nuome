"""Golden Boy against itself (nuome --ideas golden), a fixed time a move.
Every move is logged as it is played, with what it rests on (logical,
theoretical, bounded game) and the depth reached; the rules (repetition,
fifty moves, insufficient material) are python-chess's. The game is written
as a PGN with the reasons as comments.

Usage: python tools/golden_selfplay.py [--ms 30000] [--max-plies 300] [--out out/golden/selfplay]
"""
import argparse, os, re, subprocess, time

import chess, chess.pgn, requests

NUOME = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
EXE = os.path.join(NUOME, "target", "release", "nuome.exe")
MOVE = re.compile(r"^move: (\S+) \((.*), ([0-9.]+) s\)$", re.M)
DEPTH = re.compile(r"bounded game \((\d+) plies")


SERVER = "http://localhost:5000"


def push_live(board, moves, result=None, active=True):
    """Mirror the game to the local live page (ChessEngine/server.py, http://localhost:5000/live)."""
    try:
        requests.post(f"{SERVER}/live_game", json={"fen": board.fen(), "moves": moves, "white": "Golden Boy", "black": "Golden Boy", "result": result, "active": active}, timeout=1)
    except Exception:
        pass


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ms", type=int, default=30000)
    ap.add_argument("--max-plies", type=int, default=300)
    ap.add_argument("--out", default="out/golden/selfplay")
    a = ap.parse_args()
    env = dict(os.environ, GOLDEN_TIME_MS=str(a.ms), GOLDEN_PROOF_PLIES="20")
    b = chess.Board()
    game = chess.pgn.Game()
    game.headers.update({"Event": f"Golden Boy self-play, {a.ms // 1000} s a move", "White": "Golden Boy", "Black": "Golden Boy"})
    node = game
    log = open(a.out + ".log", "w", encoding="utf-8")
    t0 = time.time()
    push_live(b, [])
    while not b.is_game_over(claim_draw=True) and b.ply() < a.max_plies:
        out = subprocess.run([EXE, "--ideas", "golden", b.fen()], capture_output=True, text=True, encoding="utf-8", env=env, cwd=NUOME).stdout
        m = MOVE.search(out)
        if not m:
            log.write(f"no move from Golden Boy at {b.fen()}\n{out}\n")
            break
        uci, kind, secs = m.group(1), m.group(2), m.group(3)
        d = DEPTH.search(out)
        depth = d.group(1) if d else "-"
        move = chess.Move.from_uci(uci)
        if move not in b.legal_moves:
            log.write(f"illegal move {uci} at {b.fen()}\n")
            break
        san = b.san(move)
        line = f"{b.fullmove_number}{'.' if b.turn else '...'} {san}  ({kind}, depth {depth}, {secs} s)"
        log.write(line + "\n")
        log.flush()
        b.push(move)
        node = node.add_variation(move, comment=f"{kind}, depth {depth}, {secs} s")
        push_live(b, [m.uci() for m in b.move_stack])
    result = b.result(claim_draw=True)
    game.headers["Result"] = result
    push_live(b, [m.uci() for m in b.move_stack], result=result, active=False)
    reason = b.outcome(claim_draw=True).termination.name if b.outcome(claim_draw=True) else f"stopped at {b.ply()} plies"
    log.write(f"\nresult {result} ({reason}), {b.ply()} plies, {(time.time() - t0) / 60:.0f} min\n")
    log.close()
    with open(a.out + ".pgn", "w", encoding="utf-8") as f:
        print(game, file=f)
    print(f"result {result} ({reason}), {b.ply()} plies -> {a.out}.pgn")


if __name__ == "__main__":
    main()
