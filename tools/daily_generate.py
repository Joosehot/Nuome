"""Nuome Daily's puzzles: one for each day, none asked twice.

Each day takes the shortest puzzle not yet used whose answer no earlier day had; when none is left, Golden grows
more from the next seed (`nuome --ideas golden-daily SEED OUT`, about 3 minutes a seed). Days already written stay
as they are. Before a puzzle is written it is checked here with no code from Nuome: every structure built, and every
structure there is on up to 6 points.

Usage: python tools/daily_generate.py [--days 60] [--launch 2026-11-02]
"""
import argparse, datetime, glob, itertools, json, os, re, subprocess, sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
NUOME = os.path.join(ROOT, "target", "release", "nuome.exe")
SEEDS = os.path.join(ROOT, "bench", "golden_problem", "daily")
OUT = os.path.join(ROOT, "docs", "puzzles")
# longer than this and it does not fit in a day's reading
MOST_CHARS = 330
MOST_SEEDS = 200
BRUTE_TO = 6
A, B, OUT_X, OUT_Y, IN_X, IN_Y, COMMON_OUT, COMMON_IN, PATHS, SIZE = range(10)
LIMIT = 1_000_000_000


def val(t, r):
    o = t[0]
    if o == "num":
        return t[1]
    if o == "read":
        return r[t[1]]
    if o == "abs":
        return abs(val(t[1], r))
    if o == "if":
        return val(t[2], r) if val(t[1], r) != 0 else val(t[3], r)
    x, y = val(t[1], r), val(t[2], r)
    if o == "min":
        return min(x, y)
    if o == "max":
        return max(x, y)
    z = {"add": x + y, "sub": x - y, "mul": x * y}[o]
    return max(-LIMIT, min(LIMIT, z))


def reads(at, n, x, y):
    r = [0] * 10
    r[A], r[B], r[SIZE] = at[x][y], at[y][x], n
    for z in range(n):
        r[OUT_X] += at[x][z]
        r[OUT_Y] += at[y][z]
        r[IN_X] += at[z][x]
        r[IN_Y] += at[z][y]
        r[COMMON_OUT] += at[x][z] * at[y][z]
        r[COMMON_IN] += at[z][x] * at[z][y]
        r[PATHS] += at[x][z] * at[z][y]
    return r


def good(tree, at):
    n = len(at)
    return all(x == y or val(tree, reads(at, n, x, y)) == 0 for x in range(n) for y in range(n))


def exists(p, n):
    """every structure on n points, one at a time"""
    pairs = list(itertools.combinations(range(n), 2))
    for bits in itertools.product((0, 1), repeat=len(pairs)):
        at = [[0] * n for _ in range(n)]
        for (i, j), b in zip(pairs, bits):
            if p["class"] == "graph":
                at[i][j] = at[j][i] = b
            else:
                at[i][j], at[j][i] = b, 1 - b
        if good(p["tree"], at):
            return True
    return False


def checked(p):
    """why the puzzle is wrong, or None"""
    if sorted(p["answer"] + p["impossible"]) != list(range(3, p["bound"] + 1)):
        return "the n's from 3 to the bound are not each decided once"
    for n in p["answer"]:
        rows = p["built"][str(n)]
        at = [[r >> j & 1 for j in range(n)] for r in rows]
        if len(rows) != n or any(at[i][i] for i in range(n)):
            return f"built at {n} is not a structure"
        for i, j in itertools.combinations(range(n), 2):
            if (at[i][j] == at[j][i]) != (p["class"] == "graph"):
                return f"built at {n} is not a {p['class']}"
        if not good(p["tree"], at):
            return f"built at {n} breaks the condition"
    for n in range(3, min(BRUTE_TO, p["bound"]) + 1):
        if exists(p, n) != (n in p["answer"]):
            return f"every structure on {n} points tried: the answer is wrong there"
    return None


def grown():
    """the seeds grown so far, in order"""
    seeds = sorted(int(re.search(r"seed(\d+)", f).group(1)) for f in glob.glob(os.path.join(SEEDS, "seed*.json")))
    out = []
    for s in seeds:
        with open(os.path.join(SEEDS, f"seed{s}.json"), encoding="utf-8") as f:
            out.extend(json.load(f))
    return seeds, out


def grow(seed):
    print(f"growing seed {seed} ...", flush=True)
    path = os.path.join(SEEDS, f"seed{seed}.json")
    subprocess.run([NUOME, "--ideas", "golden-daily", str(seed), path], cwd=ROOT, check=True, stderr=subprocess.DEVNULL)


def solution(p):
    return "\n".join([p["question"], "", *p["steps"], "", p["answer_line"], *("  " + c for c in p["checks"]), ""])


def main():
    a = argparse.ArgumentParser()
    a.add_argument("--days", type=int, default=60)
    a.add_argument("--launch", default="2026-11-02")
    args = a.parse_args()
    launch = datetime.date.fromisoformat(args.launch)
    today = datetime.datetime.now(datetime.timezone.utc).date()
    start = max(launch, today)
    os.makedirs(OUT, exist_ok=True)
    os.makedirs(SEEDS, exist_ok=True)

    used, answers = set(), set()
    for f in glob.glob(os.path.join(OUT, "*.json")):
        with open(f, encoding="utf-8") as h:
            p = json.load(h)
        used.add(p["key"])
        answers.add(tuple(p["answer"]))

    wrong = 0
    for k in range(args.days):
        day = start + datetime.timedelta(days=k)
        path = os.path.join(OUT, f"{day}.json")
        if os.path.exists(path):
            continue
        while True:
            seeds, pool = grown()
            free = [p for p in pool if p["key"] not in used and tuple(p["answer"]) not in answers and len(p["question"]) <= MOST_CHARS]
            free.sort(key=lambda p: (len(p["question"]), p["seed"], p["key"]))
            chosen = None
            for p in free:
                why = checked(p)
                if why is None:
                    chosen = p
                    break
                wrong += 1
                used.add(p["key"])
                print(f"SKIPPED {p['key']} (seed {p['seed']}): {why}", file=sys.stderr)
            if chosen:
                break
            seed = max(seeds, default=0) + 1
            if seed > MOST_SEEDS:
                sys.exit(f"{day}: no puzzle left after {MOST_SEEDS} seeds")
            grow(seed)
        p = dict(chosen)
        p["date"], p["number"] = str(day), (day - launch).days + 1
        used.add(p["key"])
        answers.add(tuple(p["answer"]))
        with open(path, "w", encoding="utf-8", newline="\n") as h:
            json.dump(p, h, indent=1)
        with open(os.path.join(OUT, f"{day}.txt"), "w", encoding="utf-8", newline="\n") as h:
            h.write(solution(p))
        print(f"{day} #{p['number']}: n = {p['answer']} (seed {p['seed']})", flush=True)
    print(f"done; {wrong} puzzles failed the check and were skipped")


if __name__ == "__main__":
    main()
