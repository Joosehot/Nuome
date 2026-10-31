"""Nuome's own tables (the taulukonlaskija on the GPU) against Syzygy: out/golden/retro_check.tsv has a sample of each
class's positions with Nuome's value for the side to move (1 win, 0 draw, -1 loss); every one is probed in Syzygy
(the win/draw/loss by perfect play; a cursed win counts as a win: Nuome's tables do not count the 50-move rule).

Usage: python tools/retro_check.py
"""
import collections, os, sys
import chess, chess.syzygy
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import golden_tablebase as gt

tb = chess.syzygy.open_tablebase(gt.TB_PATH)
tally = collections.defaultdict(lambda: [0, 0])
wrong = []
for line in open("out/golden/retro_check.tsv"):
    cls, fen, v = line.rstrip("\n").split("\t")
    b = chess.Board(fen)
    if b.is_checkmate():
        truth = -1
    elif b.is_stalemate() or b.is_insufficient_material():
        truth = 0
    else:
        w = tb.probe_wdl(b)
        truth = (w > 0) - (w < 0)
    tally[cls][1] += 1
    if truth == int(v):
        tally[cls][0] += 1
    elif len(wrong) < 10:
        wrong.append((cls, fen, v, truth))
for cls, (ok, n) in tally.items():
    print(f"{cls}: {ok}/{n} agree with Syzygy")
for w in wrong:
    print("disagree:", *w)
