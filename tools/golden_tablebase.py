#!/usr/bin/env python3
"""Exact counterexample hunt for the golden-function conjecture, using Syzygy WDL tables.

The conjecture: `nuome --ideas golden "<FEN>" [DEPTH]` plays correctly in every position.
Where the truth is known exactly (at most 5 pieces), a move is correct when it keeps the
best game-theoretic WDL value the side to move has (win stays win, draw stays at least draw,
cursed win stays cursed win, blessed loss does not slide into a real loss). Positions that
are lost (WDL -2) are not testable with WDL alone: every move loses.

Usage (Git Bash, from anywhere):
    python tools/golden_tablebase.py [--per-class N] [--depth D] [--classes KQvK,KRvK,...]
                                     [--seed S] [--jobs J] [--tb PATH] [--exe PATH]
                                     [--only-testable]

The exe is copied to out/golden/nuome_tb.exe at start (the original may be rebuilt meanwhile)
and run with cwd = the Nuome folder, so it finds out/golden/formula.txt.
"""
import argparse
import multiprocessing as mp
import os
import random
import re
import shutil
import subprocess
import sys
import time
import zlib

import chess
import chess.syzygy

NUOME = r"C:\Users\joose\Nuome"
SRC_EXE = os.path.join(NUOME, "target", "release", "nuome.exe")
OUT = os.path.join(NUOME, "out", "golden")
TB_EXE = os.path.join(OUT, "nuome_tb.exe")
TB_PATH = r"C:\Users\joose\Tools\syzygy\345-wdl"

DEFAULT_CLASSES = ["KQvK", "KRvK", "KPvK", "KBNvK", "KBBvK", "KRvKR", "KQvKR",
                   "KRvKB", "KRvKN", "KPvKP", "KQvKQ", "KRPvKR"]

PIECES = {"K": chess.KING, "Q": chess.QUEEN, "R": chess.ROOK, "B": chess.BISHOP,
          "N": chess.KNIGHT, "P": chess.PAWN}
MOVE_RE = re.compile(r"^move: (\S+) \(", re.M)
WDL_NAME = {2: "win", 1: "cursed win", 0: "draw", -1: "blessed loss", -2: "loss"}


def random_position(cls, rng):
    """A random legal position of material class cls (e.g. 'KRPvKR'), or None on a miss."""
    white, black = cls.split("v")
    board = chess.Board(None)
    squares = list(chess.SQUARES)
    rng.shuffle(squares)
    i = 0
    for color, side in ((chess.WHITE, white), (chess.BLACK, black)):
        for ch in side:
            pt = PIECES[ch]
            sq = squares[i]
            i += 1
            if pt == chess.PAWN and chess.square_rank(sq) in (0, 7):
                return None
            board.set_piece_at(sq, chess.Piece(pt, color))
    board.turn = rng.random() < 0.5
    board.castling_rights = 0
    board.ep_square = None
    # is_valid: kings present, no pawns on ranks 1/8, side not to move not in check, ...
    if not board.is_valid():
        return None
    if not any(board.legal_moves):
        return None
    return board


def generate(classes, per_class, seed, tb=None):
    """per_class distinct positions per class; with tb given, only positions not lost for the mover."""
    jobs = []
    for cls in classes:
        rng = random.Random(seed * 1_000_003 + zlib.crc32(cls.encode()))
        seen = set()
        tries = 0
        while len(seen) < per_class and tries < per_class * 1000:
            tries += 1
            b = random_position(cls, rng)
            if b is None:
                continue
            fen = b.fen()
            if fen in seen:
                continue
            if tb is not None and tb.probe_wdl(b) == -2:
                continue
            seen.add(fen)
            jobs.append((cls, fen))
    return jobs


_tb = None
_cfg = None


def _init(tb_path, exe, depth):
    global _tb, _cfg
    _tb = chess.syzygy.open_tablebase(tb_path)
    _cfg = (exe, depth)


def wdl(board):
    """WDL from the side to move, handling terminal and bare-king positions."""
    if board.is_checkmate():
        return -2
    if board.is_stalemate() or len(board.piece_map()) == 2:
        return 0
    return _tb.probe_wdl(board)


def child_value(board, move):
    """WDL for the mover after `move`, i.e. minus the opponent's WDL."""
    board.push(move)
    try:
        return -wdl(board)
    finally:
        board.pop()


def check(job):
    cls, fen = job
    exe, depth = _cfg
    board = chess.Board(fen)
    true = wdl(board)
    values = {m: child_value(board, m) for m in board.legal_moves}
    best = max(values.values())
    assert best == true, (fen, best, true)
    res = {"cls": cls, "fen": fen, "wdl": true}
    if true == -2:
        res["status"] = "untestable"
        return res
    try:
        p = subprocess.run([exe, "--ideas", "golden", fen, str(depth)], cwd=NUOME,
                           capture_output=True, text=True, timeout=300)
        out = p.stdout
    except subprocess.TimeoutExpired:
        res["status"] = "error"
        res["note"] = "timeout"
        return res
    m = MOVE_RE.search(out)
    good = sorted(mv.uci() for mv, v in values.items() if v == best)
    res["good"] = good
    res["trivial"] = len(good) == len(values)  # every legal move is correct
    if not m:
        res["status"] = "error"
        res["note"] = "no move line (exit %s): %s" % (p.returncode, (out + p.stderr).strip()[-200:])
        return res
    uci = m.group(1)
    res["move"] = uci
    try:
        mv = chess.Move.from_uci(uci)
    except ValueError:
        mv = None
    if mv is None or mv not in values:
        res["status"] = "counterexample"
        res["after"] = None
        res["note"] = "illegal move"
        return res
    res["after"] = values[mv]
    res["status"] = "holds" if values[mv] == best else "counterexample"
    return res


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--per-class", type=int, default=100)
    ap.add_argument("--depth", type=int, default=2)
    ap.add_argument("--classes", default=",".join(DEFAULT_CLASSES))
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--jobs", type=int, default=12)
    ap.add_argument("--tb", default=TB_PATH)
    ap.add_argument("--exe", default=SRC_EXE, help="binary to copy and test")
    ap.add_argument("--no-copy", action="store_true", help="run --exe where it is (it finds its own data files)")
    ap.add_argument("--only-testable", action="store_true",
                    help="skip lost positions while generating, so every class gets N testable ones")
    a = ap.parse_args()

    classes = [c.strip() for c in a.classes.split(",") if c.strip()]
    for c in classes:
        n = len(c.replace("v", ""))
        if not re.fullmatch(r"K[QRBNP]*vK[QRBNP]*", c) or n > 5:
            sys.exit("bad class %r (need e.g. KRPvKR, at most 5 pieces)" % c)

    os.makedirs(OUT, exist_ok=True)
    run_exe = a.exe if a.no_copy else TB_EXE
    if not a.no_copy:
        shutil.copy2(a.exe, TB_EXE)
    exe_info = "%s (copied from %s, modified %s, %d bytes)" % (
        TB_EXE, a.exe, time.strftime("%Y-%m-%d %H:%M:%S", time.localtime(os.path.getmtime(a.exe))),
        os.path.getsize(a.exe))
    fpath = os.path.join(OUT, "formula.txt")
    if os.path.exists(fpath):
        with open(fpath, encoding="utf-8", errors="replace") as f:
            formula = "out/golden/formula.txt:\n" + f.read().strip()
    else:
        formula = "out/golden/formula.txt not present: the binary's built-in default formula was used"

    gen_tb = chess.syzygy.open_tablebase(a.tb) if a.only_testable else None
    jobs = generate(classes, a.per_class, a.seed, gen_tb)
    print("binary: " + exe_info)
    print(formula)
    print("testing %d positions over %d classes, depth %d, seed %d, %d processes"
          % (len(jobs), len(classes), a.depth, a.seed, a.jobs), flush=True)

    t0 = time.time()
    with mp.Pool(a.jobs, initializer=_init, initargs=(a.tb, run_exe, a.depth)) as pool:
        results = pool.map(check, jobs, chunksize=4)
    secs = time.time() - t0

    keys = ("n", "untestable", "tested", "holds", "cx", "error", "hard", "hard_holds")
    stats = {c: dict.fromkeys(keys, 0) for c in classes}
    for r in results:
        s = stats[r["cls"]]
        s["n"] += 1
        st = r["status"]
        if st == "untestable":
            s["untestable"] += 1
        elif st == "error":
            s["error"] += 1
        else:
            s["tested"] += 1
            s["holds" if st == "holds" else "cx"] += 1
            if not r.get("trivial"):
                s["hard"] += 1
                s["hard_holds"] += st == "holds"

    lines = []
    lines.append("Golden function vs Syzygy 3-4-5 WDL tablebases")
    lines.append("binary: " + exe_info)
    lines.append(formula)
    lines.append("depth %d, seed %d, %d positions per class%s, %.1f s" % (
        a.depth, a.seed, a.per_class, " (only testable)" if a.only_testable else "", secs))
    lines.append("")
    lines.append("%-8s %6s %10s %7s %6s %15s %6s %13s" % ("class", "pos", "lost(skip)", "tested", "holds", "counterexamples", "errors", "non-trivial"))
    T = dict.fromkeys(keys, 0)
    for c in classes:
        s = stats[c]
        for k in T:
            T[k] += s[k]
        lines.append("%-8s %6d %10d %7d %6d %15d %6d %7d/%-5d" % (c, s["n"], s["untestable"], s["tested"], s["holds"], s["cx"], s["error"], s["hard_holds"], s["hard"]))
    lines.append("%-8s %6d %10d %7d %6d %15d %6d %7d/%-5d" % ("total", T["n"], T["untestable"], T["tested"], T["holds"], T["cx"], T["error"], T["hard_holds"], T["hard"]))
    lines.append("non-trivial = holds / tested positions where at least one legal move is wrong")
    lines.append("")
    cxs = [r for r in results if r["status"] == "counterexample"]
    lines.append("The conjecture holds in %d of %d testable positions (%d of %d where some move is wrong)."
                 % (T["holds"], T["tested"], T["hard_holds"], T["hard"]))
    if cxs:
        r = cxs[0]
        lines.append("First counterexample: %s  played %s (%s -> %s), a correct move: %s"
                     % (r["fen"], r["move"], WDL_NAME[r["wdl"]],
                        WDL_NAME.get(r["after"], r.get("note", "?")), r["good"][0]))
    else:
        lines.append("No counterexample found in this sample.")
    errs = [r for r in results if r["status"] == "error"]
    if errs:
        lines.append("Errors (no usable output): %d, e.g. %s: %s" % (len(errs), errs[0]["fen"], errs[0].get("note")))
    lines.append("")
    lines.append("Correct = the move keeps the best WDL value (win stays win, draw stays at least a draw).")
    lines.append("Caveat: WDL preservation alone does not show the function converts a win before the")
    lines.append("50-move rule (it could shuffle forever while 'keeping' the win); that needs DTZ tables.")
    lines.append("Lost positions are skipped: with WDL only, every move is equally lost.")
    summary = "\n".join(lines)
    print()
    print(summary)

    with open(os.path.join(OUT, "tablebase_summary.txt"), "w", encoding="utf-8") as f:
        f.write(summary + "\n")
    with open(os.path.join(OUT, "counterexamples.txt"), "w", encoding="utf-8") as f:
        f.write("# class | FEN | true WDL | move played | WDL after move | correct moves\n")
        for r in cxs:
            f.write("%s | %s | %s | %s | %s | %s\n" % (
                r["cls"], r["fen"], WDL_NAME[r["wdl"]], r["move"],
                WDL_NAME.get(r["after"], r.get("note", "?")), " ".join(r["good"])))


if __name__ == "__main__":
    main()
