"""Stockfish's games from CCRL 40/15 (computerchess.org.uk), the recordings
for golden_voice: every per-engine file of a Stockfish version, downloaded
one at a time (the site asks for no aggressive downloading), unpacked, and
written as one game a line:

    <colours Stockfish played: W, B or WB> TAB <result> TAB <uci moves>

Only the moves are kept (the comments' evaluations are dropped).

Usage: python tools/ccrl_sf.py [--out out/golden/ccrl_sf_games.txt] [--dir hf_data/ccrl]
"""
import argparse, io, os, re, time, urllib.request

import chess.pgn
import py7zr

BASE = "https://computerchess.org.uk/4040/"
HEADERS = {"User-Agent": "Mozilla/5.0"}


def fetch(url):
    return urllib.request.urlopen(urllib.request.Request(url, headers=HEADERS), timeout=120).read()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", default="out/golden/ccrl_sf_games.txt")
    ap.add_argument("--dir", default=r"C:\Users\joose\Desktop\ChessEngine\hf_data\ccrl")
    a = ap.parse_args()
    os.makedirs(a.dir, exist_ok=True)
    page = fetch(BASE + "games.html").decode("utf-8", "replace")
    files = sorted(set(re.findall(r'href="(games-by-engine-commented/Stockfish[^"]+\.7z)"', page)))
    print(f"{len(files)} Stockfish files", flush=True)
    games = 0
    seen = set()
    with open(a.out, "w") as out:
        for i, f in enumerate(files):
            local = os.path.join(a.dir, os.path.basename(f))
            if not os.path.exists(local):
                url = BASE + urllib.request.quote(f, safe="/")
                data = fetch(url)
                with open(local, "wb") as fh:
                    fh.write(data)
                time.sleep(1)
            tmp = os.path.join(a.dir, "_unpacked")
            os.makedirs(tmp, exist_ok=True)
            with py7zr.SevenZipFile(local, "r") as z:
                z.extractall(path=tmp)
            for name in os.listdir(tmp):
                path = os.path.join(tmp, name)
                with open(path, encoding="utf-8", errors="replace") as fh:
                    text = io.StringIO(fh.read())
                os.remove(path)
                if True:
                    while True:
                        g = chess.pgn.read_game(text)
                        if g is None:
                            break
                        w, b = g.headers.get("White", ""), g.headers.get("Black", "")
                        sf = ("W" if "Stockfish" in w else "") + ("B" if "Stockfish" in b else "")
                        if not sf or g.headers.get("SetUp") == "1" or g.headers.get("FEN"):
                            continue
                        moves = [m.uci() for m in g.mainline_moves()]
                        key = (sf, " ".join(moves))
                        if len(moves) < 10 or key in seen:
                            continue
                        seen.add(key)
                        out.write(f"{sf}\t{g.headers.get('Result', '*')}\t{' '.join(moves)}\n")
                        games += 1
            print(f"  {i + 1}/{len(files)} {os.path.basename(f)}: {games} games so far", flush=True)
    print(f"{games} Stockfish games -> {a.out}")


if __name__ == "__main__":
    main()
