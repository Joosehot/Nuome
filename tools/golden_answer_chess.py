"""Golden Answer's chess on endgame positions: every position asked in all
three kinds ("best move in <FEN>"); a move is right when it keeps the
position's best result by the Syzygy tables (win stays win, draw stays at
least a draw).

Positions: tools/golden_tablebase.py's generator (seed 1), --per-class a
class, only positions not already lost for the side to move.

The theoretical kind cites these same tables, so its check is circular (it
shows only that the wiring works); the logical kind is checked
independently (Nuome's own tables and search against de Man's). Abstract
answers claim nothing and are not scored; how often they keep the result is
shown for information.

Usage: python tools/golden_answer_chess.py [--per-class 10] [--jobs 12]
"""
import argparse, os, re, subprocess, sys
from concurrent.futures import ThreadPoolExecutor

import chess, chess.syzygy

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import golden_tablebase as gt

NUOME = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
EXE = os.path.join(NUOME, "target", "release", "nuome.exe")
KINDS = ["logical", "theoretical", "abstract"]
ANSWER = re.compile(r"^answer: (\S+)", re.M)


def ask(kind, fen):
    p = subprocess.run([EXE, "--answer", kind, f"best move in {fen}"], capture_output=True, text=True, encoding="utf-8", timeout=600, cwd=NUOME)
    m = ANSWER.search(p.stdout)
    return m.group(1) if m else None


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--per-class", type=int, default=10)
    ap.add_argument("--jobs", type=int, default=12)
    a = ap.parse_args()
    tb = chess.syzygy.open_tablebase(gt.TB_PATH)
    gt._tb = tb
    jobs = gt.generate(gt.DEFAULT_CLASSES[:10], a.per_class, 1, tb)
    tasks = [(k, cls, fen) for k in KINDS for cls, fen in jobs]
    with ThreadPoolExecutor(a.jobs) as ex:
        answers = list(ex.map(lambda t: ask(t[0], t[2]), tasks))
    rows = {}
    for (k, cls, fen), mv in zip(tasks, answers):
        b = chess.Board(fen)
        best = max(gt.child_value(b, m) for m in b.legal_moves)
        ok = None
        if mv is not None:
            try:
                ok = gt.child_value(b, chess.Move.from_uci(mv)) == best
            except Exception:
                ok = False
        rows.setdefault((k, cls), []).append(ok)
    print(f"Golden Answer, best move in {len(jobs)} endgame positions (Syzygy-judged; set: golden_tablebase generator, seed 1, {a.per_class} a class)\n")
    for k in KINDS:
        label = {"logical": "right", "theoretical": "right (circular)", "abstract": "keeps result (info)"}[k]
        print(f"{k}:")
        print(f"  {'class':<8} {'n':>4} {'answered':>9} {label:>20}")
        tn = ta = tr = 0
        for cls in gt.DEFAULT_CLASSES[:10]:
            r = rows.get((k, cls), [])
            n, ans, right = len(r), sum(x is not None for x in r), sum(x is True for x in r)
            print(f"  {cls:<8} {n:>4} {ans:>9} {f'{right}/{ans}':>20}")
            tn, ta, tr = tn + n, ta + ans, tr + right
        print(f"  {'all':<8} {tn:>4} {ta:>9} {f'{tr}/{ta}':>20}\n")


if __name__ == "__main__":
    main()
