#!/usr/bin/env python3
"""Game evidence for the golden function (nuome --ideas golden).

Usage (run from anywhere; paths are relative to the Nuome repo root):
  python tools/golden_games.py accuracy [--depth 12] [--file out/golden/games.txt] [--pgn out/golden/games.pgn]
  python tools/golden_games.py match [--elo 1320,1600,2000] [--games 4] [--depth 2] [--movetime-sf 100]
                                     [--analysis-depth 12] [--max-plies 300]

accuracy: Stockfish analyses every move of the self-play games (the first 2 fixed opening
          plies are skipped), per game and overall, and writes a PGN with per-move comments.
match:    the golden function (a copy of target/release/nuome.exe) plays Stockfish with
          UCI_LimitStrength at each Elo, alternating colours over a few fixed openings, writes
          out/golden/vs_stockfish.pgn, prints the score per Elo, then runs the accuracy
          analysis on the golden function's own moves.
Both subcommands are re-runnable: the exe is re-copied each run and outputs are overwritten.
"""
import argparse
import datetime
import math
import os
import re
import shutil
import subprocess
import sys

import chess
import chess.engine
import chess.pgn

ROOT = r"C:\Users\joose\Nuome"
SRC_EXE = os.path.join(ROOT, "target", "release", "nuome.exe")
EXE = os.path.join(ROOT, "out", "golden", "nuome_games.exe")
STOCKFISH = r"C:\Users\joose\Desktop\ChessEngine\stockfish\stockfish-windows-x86-64-avx2.exe"
FORMULA = os.path.join(ROOT, "out", "golden", "formula.txt")
CAP = 1000  # centipawn cap for evals and for per-move loss

# fixed openings for the match (first plies), so the games differ
OPENINGS = [
    [],
    ["e2e4", "e7e5"],
    ["d2d4", "d7d5"],
    ["c2c4", "e7e5"],
    ["e2e4", "c7c5"],
    ["g1f3", "d7d5"],
]


def rp(p):
    return p if os.path.isabs(p) else os.path.join(ROOT, p)


def formula_name():
    if os.path.exists(FORMULA):
        with open(FORMULA, encoding="utf-8", errors="replace") as f:
            return f.read().strip() or "default (formula.txt empty)"
    return "default"


def copy_exe():
    os.makedirs(os.path.dirname(EXE), exist_ok=True)
    shutil.copy2(SRC_EXE, EXE)
    st = os.stat(EXE)
    print(f"golden binary: copied {SRC_EXE} -> {EXE} ({st.st_size} bytes, built "
          f"{datetime.datetime.fromtimestamp(os.stat(SRC_EXE).st_mtime):%Y-%m-%d %H:%M:%S})")
    print("formula:", formula_name())


def open_stockfish():
    eng = chess.engine.SimpleEngine.popen_uci(STOCKFISH)
    try:
        eng.configure({"Threads": 1, "Hash": 64})
    except chess.engine.EngineError:
        pass
    return eng


# ---------------------------------------------------------------- analysis

def win_pct(cp):
    return 50 + 50 * (2 / (1 + math.exp(-0.00368208 * cp)) - 1)


def move_accuracy(wb, wa):
    a = 103.1668 * math.exp(-0.04354 * (wb - wa)) - 3.1669
    return max(0.0, min(100.0, a))


def eval_cp(board, eng, depth, cache):
    """Eval of `board` from the side to move's point of view (capped), and Stockfish's best move."""
    key = len(board.move_stack)  # one game per cache; the position after ply i is the one before ply i+1
    if key in cache:
        return cache[key]
    if board.is_checkmate():
        res = (-CAP, None)
    elif board.is_stalemate() or board.is_insufficient_material():
        res = (0, None)
    else:
        info = eng.analyse(board, chess.engine.Limit(depth=depth))
        sc = info["score"].pov(board.turn).score(mate_score=100000)
        pv = info.get("pv") or []
        res = (max(-CAP, min(CAP, sc)), pv[0] if pv else None)
    cache[key] = res
    return res


def analyse_game(moves, eng, depth, include, start_board=None):
    """moves: list of chess.Move from start; include(ply, board) -> bool selects moves to score.
    Returns list of per-move dicts (only for included plies)."""
    board = start_board.copy() if start_board else chess.Board()
    cache = {}
    out = []
    for ply, mv in enumerate(moves):
        if include(ply, board):
            before, best = eval_cp(board, eng, depth, cache)
            mover = board.turn
            board.push(mv)
            opp, _ = eval_cp(board, eng, depth, cache)
            after = -opp
            loss = max(0, min(CAP, before - after))
            wb, wa = win_pct(before), win_pct(after)
            out.append({
                "ply": ply, "move": mv, "white": mover == chess.WHITE,
                "before": before, "after": after, "loss": loss,
                "acc": move_accuracy(wb, wa) if wa < wb else 100.0,
                "best": best, "match": best == mv,
            })
        else:
            board.push(mv)
    return out


def summarize(rows):
    n = len(rows)
    if n == 0:
        return None
    return {
        "n": n,
        "acpl": sum(r["loss"] for r in rows) / n,
        "acc": sum(r["acc"] for r in rows) / n,
        "match": 100.0 * sum(r["match"] for r in rows) / n,
        "inacc": sum(50 <= r["loss"] < 100 for r in rows),
        "mist": sum(100 <= r["loss"] < 300 for r in rows),
        "blun": sum(r["loss"] >= 300 for r in rows),
    }


def fmt(s):
    if s is None:
        return "no moves analysed"
    return (f"moves {s['n']:4d}  avg loss {s['acpl']:6.1f} cp  accuracy {s['acc']:5.1f}%  "
            f"SF-match {s['match']:5.1f}%  inacc {s['inacc']:3d}  mistakes {s['mist']:3d}  blunders {s['blun']:3d}")


def annotate(node_moves, rows_by_ply, game):
    node = game
    for ply, mv in enumerate(node_moves):
        node = node.add_variation(mv)
        r = rows_by_ply.get(ply)
        if r is not None:
            c = f"loss {r['loss']} cp, eval {r['after'] / 100:+.2f} (mover)"
            if not r["match"] and r["best"] is not None:
                c += f", SF best {r['best'].uci()}"
            node.comment = c
            if r["loss"] >= 300:
                node.nags.add(chess.pgn.NAG_BLUNDER)
            elif r["loss"] >= 100:
                node.nags.add(chess.pgn.NAG_MISTAKE)
            elif r["loss"] >= 50:
                node.nags.add(chess.pgn.NAG_DUBIOUS_MOVE)


# ---------------------------------------------------------------- accuracy

def cmd_accuracy(a):
    path = rp(a.file)
    if not os.path.exists(path):
        sys.exit(f"{path} missing; generate it with: out\\golden\\nuome_games.exe --ideas golden-play 8 2")
    print("formula:", formula_name())
    games = []
    with open(path, encoding="utf-8") as f:
        for line in f:
            line = line.rstrip("\n")
            if not line.strip():
                continue
            parts = line.split("\t")
            result, how, ucis = parts[0], parts[1], parts[2].split() if len(parts) > 2 else []
            games.append((result, how, ucis))
    print(f"{len(games)} self-play games from {path}; Stockfish depth {a.depth}; skipping the first {a.skip} plies\n")
    eng = open_stockfish()
    allrows = []
    pgn_out = []
    try:
        for gi, (result, how, ucis) in enumerate(games, 1):
            b = chess.Board()
            moves = []
            for u in ucis:
                m = chess.Move.from_uci(u)
                if m not in b.legal_moves:
                    print(f"  game {gi}: illegal move {u} at ply {len(moves)}; truncated")
                    break
                moves.append(m)
                b.push(m)
            rows = analyse_game(moves, eng, a.depth, lambda ply, _b: ply >= a.skip)
            allrows += rows
            sw = summarize([r for r in rows if r["white"]])
            sb = summarize([r for r in rows if not r["white"]])
            print(f"game {gi}: {result} by {how}, {len(moves)} plies")
            print("   all   ", fmt(summarize(rows)))
            print("   white ", fmt(sw))
            print("   black ", fmt(sb))
            g = chess.pgn.Game()
            g.headers.update({"Event": "golden self-play", "Site": "Nuome", "Round": str(gi),
                              "White": "golden", "Black": "golden", "Result": result,
                              "Termination": how, "Annotator": f"Stockfish 17 depth {a.depth}"})
            annotate(moves, {r["ply"]: r for r in rows}, g)
            pgn_out.append(str(g))
    finally:
        eng.quit()
    print("\nOVERALL ", fmt(summarize(allrows)))
    out = rp(a.pgn)
    with open(out, "w", encoding="utf-8") as f:
        f.write("\n\n".join(pgn_out) + "\n")
    print(f"PGN written to {out}")


# ---------------------------------------------------------------- match

def golden_move(board, depth):
    p = subprocess.run([EXE, "--ideas", "golden", board.fen(), str(depth)], cwd=ROOT,
                       capture_output=True, text=True, timeout=600)
    for line in p.stdout.splitlines():
        m = re.match(r"^move: (\S+) \(", line)
        if m:
            return chess.Move.from_uci(m.group(1))
    raise RuntimeError(f"no move line (exit {p.returncode}): {p.stdout[-300:]} {p.stderr[-300:]}")


def play_game(eng, elo, golden_white, opening, depth, movetime, max_plies):
    board = chess.Board()
    for u in opening:
        board.push_uci(u)
    term = None
    result = None
    while True:
        if board.is_game_over(claim_draw=True):
            out = board.outcome(claim_draw=True)
            result, term = out.result(), out.termination.name.lower()
            break
        if board.ply() >= max_plies:
            result, term = "1/2-1/2", f"max {max_plies} plies"
            break
        golden_turn = (board.turn == chess.WHITE) == golden_white
        if golden_turn:
            try:
                mv = golden_move(board, depth)
            except Exception as e:
                mv, err = None, str(e)
            if mv is None or mv not in board.legal_moves:
                result = "0-1" if golden_white else "1-0"
                term = f"golden gave no legal move ({mv.uci() if mv else err})"
                break
        else:
            mv = eng.play(board, chess.engine.Limit(time=movetime / 1000.0)).move
        board.push(mv)
    return board, result, term


def cmd_match(a):
    copy_exe()
    elos = [int(x) for x in a.elo.split(",") if x.strip()]
    eng = open_stockfish()
    lo, hi = eng.options["UCI_Elo"].min, eng.options["UCI_Elo"].max
    print(f"Stockfish {eng.id.get('name')}: UCI_Elo accepts {lo}..{hi}")
    for e in elos:
        if not lo <= e <= hi:
            eng.quit()
            sys.exit(f"Elo {e} outside {lo}..{hi}")
    print(f"golden depth {a.depth}; Stockfish {a.movetime_sf} ms/move; {a.games} games per Elo; max {a.max_plies} plies\n")
    games = []  # (elo, game_no, golden_white, moves, result, term, opening_plies, pgn_game)
    pgn_out = []
    try:
        for elo in elos:
            eng.configure({"UCI_LimitStrength": True, "UCI_Elo": elo})
            for i in range(a.games):
                golden_white = i % 2 == 0
                opening = OPENINGS[(i // 2) % len(OPENINGS)]
                board, result, term = play_game(eng, elo, golden_white, opening, a.depth, a.movetime_sf, a.max_plies)
                moves = list(board.move_stack)
                games.append((elo, i + 1, golden_white, moves, result, term, len(opening)))
                gs = {"1-0": 1.0, "0-1": 0.0}.get(result, 0.5)
                gs = gs if golden_white else 1 - gs
                print(f"  Elo {elo} game {i + 1}: golden {'white' if golden_white else 'black'}, "
                      f"opening {' '.join(opening) or 'start'}: {result} ({term}, {len(moves)} plies) "
                      f"-> golden {gs:g}", flush=True)
                g = chess.pgn.Game()
                g.headers.update({"Event": "golden vs Stockfish", "Site": "Nuome", "Round": f"{elo}.{i + 1}",
                                  "White": f"golden d{a.depth}" if golden_white else f"Stockfish UCI_Elo {elo}",
                                  "Black": f"Stockfish UCI_Elo {elo}" if golden_white else f"golden d{a.depth}",
                                  "Result": result, "Termination": term})
                games[-1] = games[-1] + (g,)
    except BaseException:
        eng.quit()
        raise

    print("\nscore per Elo (golden's point of view):")
    total = 0
    for elo in elos:
        gl = [x for x in games if x[0] == elo]
        w = d = l = 0
        for (_, _, gw, _, res, _, _, _) in gl:
            if res == "1/2-1/2":
                d += 1
            elif (res == "1-0") == gw:
                w += 1
            else:
                l += 1
        n = len(gl)
        s = (w + 0.5 * d) / n if n else 0
        line = f"  Elo {elo}: +{w} ={d} -{l}  score {w + 0.5 * d:g}/{n} ({100 * s:.0f}%)"
        if n < 20:
            line += "  - sample too small for an Elo estimate"
        elif s in (0.0, 1.0):
            line += "  - no Elo estimate from a 0% or 100% score"
        else:
            diff = -400 * math.log10(1 / s - 1)
            se = math.sqrt(s * (1 - s) / n)
            lo_s, hi_s = max(1e-3, s - 1.96 * se), min(1 - 1e-3, s + 1.96 * se)
            line += (f"  performance ~{elo + diff:.0f} (95% ~{elo - 400 * math.log10(1 / lo_s - 1):.0f}"
                     f"..{elo - 400 * math.log10(1 / hi_s - 1):.0f})")
        print(line)
        total += n
    print("  (Stockfish's UCI_Elo scale is calibrated at long time controls against CCRL-like pools;"
          " treat any number as rough)")

    print(f"\naccuracy of the golden function's own moves (Stockfish depth {a.analysis_depth}, opening plies skipped):")
    allrows = []
    try:
        for (elo, no, gw, moves, res, term, nop, g) in games:
            gcol = chess.WHITE if gw else chess.BLACK
            rows = analyse_game(moves, eng, a.analysis_depth,
                                lambda ply, b, gcol=gcol, nop=nop: ply >= nop and b.turn == gcol)
            for r in rows:
                r["elo"] = elo
            allrows += rows
            print(f"  Elo {elo} game {no} ({'white' if gw else 'black'}, {res}): {fmt(summarize(rows))}")
            annotate(moves, {r["ply"]: r for r in rows}, g)
            g.headers["Annotator"] = f"Stockfish 17 depth {a.analysis_depth} (golden moves only)"
            pgn_out.append(str(g))
    finally:
        eng.quit()
    for elo in elos:
        print(f"  Elo {elo} all games: {fmt(summarize([r for r in allrows if r["elo"] == elo]))}")
    print(f"  OVERALL: {fmt(summarize(allrows))}")
    out = rp(a.pgn)
    with open(out, "w", encoding="utf-8") as f:
        f.write("\n\n".join(pgn_out) + "\n")
    print(f"PGN written to {out}")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    p1 = sub.add_parser("accuracy")
    p1.add_argument("--depth", type=int, default=12)
    p1.add_argument("--file", default="out/golden/games.txt")
    p1.add_argument("--pgn", default="out/golden/games.pgn")
    p1.add_argument("--skip", type=int, default=2, help="opening plies to skip (self-play: 2)")
    p2 = sub.add_parser("match")
    p2.add_argument("--elo", default="1320,1600,2000")
    p2.add_argument("--games", type=int, default=4)
    p2.add_argument("--depth", type=int, default=2)
    p2.add_argument("--movetime-sf", type=int, default=100)
    p2.add_argument("--analysis-depth", type=int, default=12)
    p2.add_argument("--max-plies", type=int, default=300)
    p2.add_argument("--pgn", default="out/golden/vs_stockfish.pgn")
    a = ap.parse_args()
    if a.cmd == "accuracy":
        cmd_accuracy(a)
    else:
        cmd_match(a)


if __name__ == "__main__":
    main()
