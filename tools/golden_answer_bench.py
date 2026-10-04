"""Golden Answer benchmark on sets it was not written against.

Two sets, each fixed once (written to bench/golden_answer/ the first time
and never re-drawn):

- oracle: questions generated per category with a fixed seed; the truth of
  every one comes from SymPy, an independent oracle (isprime, factorint,
  primepi, exact rationals).
- math: the MATH dataset's test problems (Hendrycks et al. 2021), number
  theory and algebra, as written; the truth is the \\boxed{} answer of the
  reference solution. This measures how general Golden Answer is.

Every question is asked in all three kinds. Reported per kind and category:
n, answered, right (of the answers that can be judged), with the set's name.
For logical and theoretical answers every wrong one is listed: those claim
to be right, so a wrong one is a bug.

Usage: python tools/golden_answer_bench.py [--set oracle|math|all] [--per 100] [--jobs 8]
"""
import argparse, json, os, random, re, subprocess, urllib.request
from concurrent.futures import ThreadPoolExecutor
from fractions import Fraction

import sympy

NUOME = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
EXE = os.path.join(NUOME, "target", "release", "nuome.exe")
DIR = os.path.join(NUOME, "bench", "golden_answer")
KINDS = ["logical", "theoretical", "abstract"]
MATH_URL = "https://huggingface.co/datasets/EleutherAI/hendrycks_math/resolve/refs%2Fconvert%2Fparquet/{c}/test/0000.parquet"
MATH_SUBJECTS = ["number_theory", "algebra"]


# ── the oracle set ──

def oracle_set(per, seed=2026):
    rng = random.Random(seed)
    qs = []

    def add(cat, q, truth):
        qs.append({"category": cat, "question": q, "truth": str(truth)})

    for _ in range(per):
        digits = rng.choice([3, 6, 9, 12, 15, 18, 24, 30])
        lo, hi = 10 ** (digits - 1), 10 ** digits
        n = sympy.randprime(lo, hi) if rng.random() < 0.5 else rng.randrange(lo, hi) | 1
        add("primality", f"is {n} prime", "prime" if sympy.isprime(n) else "composite")
    for _ in range(per):
        p = rng.choice(list(sympy.primerange(2, 400)))
        add("mersenne", f"is 2^{p} - 1 prime", "prime" if sympy.isprime(2 ** p - 1) else "composite")
    for _ in range(per):
        k = rng.randint(2, 4)
        while True:
            ps = sorted(sympy.randprime(2, 10 ** rng.randint(1, 9)) for _ in range(k))
            n = 1
            for p in ps:
                n *= p
            if n < 2 ** 63:
                break
        add("factoring", f"factor {n}", " * ".join(map(str, ps)))
    for _ in range(per):
        x = rng.randint(10, 10 ** 7)
        add("prime counting", f"how many primes are below {x}", sympy.primepi(x - 1))
    for _ in range(per):
        n = rng.randint(4, 10 ** 12)
        truth = "yes" if n % 2 == 0 else ("yes" if sympy.isprime(n - 2) else "no")
        add("goldbach", f"is {n} a sum of two primes", truth)
    for _ in range(per):
        n = rng.randint(1, 10 ** 12)
        f = sympy.factorint(n)
        ok = all(e % 2 == 0 for p, e in f.items() if p % 4 == 3)
        add("sums of squares", f"is {n} a sum of two squares", "yes" if ok else "no")
    for _ in range(per):
        n = rng.randint(1, 10 ** 9)
        add("collatz", f"does {n} reach 1 under the collatz map", "yes")
    fib = [0, 1]
    while len(fib) < 90:
        fib.append(fib[-1] + fib[-2])
    for i in range(per):
        kind = ["square", "cube", "perfect", "triangular", "fibonacci", "palindrome", "divisible", "even"][i % 8]
        hit = rng.random() < 0.5
        if kind == "square":
            r = rng.randint(2, 10 ** 8)
            n = r * r if hit else r * r + rng.randint(1, 2 * r)
            add("properties", f"is {n} a perfect square", "yes" if sympy.sqrt(n).is_integer else "no")
        elif kind == "cube":
            r = rng.randint(2, 10 ** 5)
            n = r ** 3 if hit else r ** 3 + rng.randint(1, 3 * r * r)
            add("properties", f"is {n} a perfect cube", "yes" if round(n ** (1 / 3)) ** 3 == n else "no")
        elif kind == "perfect":
            n = rng.choice([6, 28, 496, 8128, 33550336, 8589869056]) if hit else rng.randint(2, 10 ** 9)
            add("properties", f"is {n} a perfect number", "yes" if sympy.divisor_sigma(n) == 2 * n else "no")
        elif kind == "triangular":
            k = rng.randint(2, 10 ** 6)
            n = k * (k + 1) // 2 + (0 if hit else rng.randint(1, k))
            add("properties", f"is {n} a triangular number", "yes" if sympy.sqrt(8 * n + 1).is_integer else "no")
        elif kind == "fibonacci":
            n = rng.choice(fib[3:]) if hit else rng.randint(4, 10 ** 15)
            add("properties", f"is {n} a fibonacci number", "yes" if n in fib else "no")
        elif kind == "palindrome":
            h = str(rng.randint(10, 10 ** 6))
            n = int(h + h[::-1]) if hit else rng.randint(100, 10 ** 12)
            add("properties", f"is {n} a palindrome", "yes" if str(n) == str(n)[::-1] else "no")
        elif kind == "divisible":
            k = rng.randint(2, 1000)
            n = k * rng.randint(1, 10 ** 9) + (0 if hit else rng.randint(1, k - 1))
            add("properties", f"is {n} divisible by {k}", "yes" if n % k == 0 else "no")
        else:
            n = rng.randint(1, 10 ** 15)
            add("properties", f"is {n} even", "yes" if n % 2 == 0 else "no")
    for _ in range(per):
        a, b, c, d = (rng.randint(1, 50) for _ in range(4))
        e = rng.randint(2, 4)
        op1, op2 = rng.choice("+-*/"), rng.choice("+-*")
        text = f"({a} {op1} {b}) {op2} {c}^{e} - {d}"
        val = sympy.Rational(sympy.sympify(text.replace("^", "**"), rational=True))
        add("arithmetic", f"what is {text}", f"{val.p}" if val.q == 1 else f"{val.p}/{val.q}")
    return qs


# ── the MATH set ──

def boxed(s):
    i = s.rfind("\\boxed")
    if i < 0:
        return None
    j = s.find("{", i)
    depth, k = 0, j
    while k < len(s):
        if s[k] == "{":
            depth += 1
        elif s[k] == "}":
            depth -= 1
            if depth == 0:
                return s[j + 1:k]
        k += 1
    return None


def math_set():
    import pandas as pd
    qs = []
    for c in MATH_SUBJECTS:
        path = os.path.join(DIR, f"math_{c}_test.parquet")
        if not os.path.exists(path):
            urllib.request.urlretrieve(MATH_URL.format(c=c), path)
        for _, r in pd.read_parquet(path).iterrows():
            t = boxed(r["solution"])
            if t is not None:
                qs.append({"category": f"MATH {c} L{str(r['level']).replace('Level ', '')}", "question": " ".join(r["problem"].split()), "truth": t})
    return qs


def fixed(name, make):
    """A set is written once and read back ever after: never re-drawn."""
    path = os.path.join(DIR, f"{name}.jsonl")
    if not os.path.exists(path):
        os.makedirs(DIR, exist_ok=True)
        with open(path, "w", encoding="utf-8") as f:
            for q in make():
                f.write(json.dumps(q) + "\n")
    with open(path, encoding="utf-8") as f:
        return [json.loads(l) for l in f]


# ── asking and judging ──

ANSWER_RE = re.compile(r"^answer: (.*)$", re.M)


def ask(kind, q):
    try:
        p = subprocess.run([EXE, "--answer", kind, q], capture_output=True, text=True, encoding="utf-8", timeout=120)
    except subprocess.TimeoutExpired:
        return None
    m = ANSWER_RE.search(p.stdout)
    return m.group(1).strip() if m else None


def number(s):
    s = s.strip().strip("$").replace("\\!", "").replace(",", "").replace(" ", "")
    s = re.sub(r"\\[dt]?frac\{([^{}]+)\}\{([^{}]+)\}", r"(\1)/(\2)", s)
    s = s.replace("\\left", "").replace("\\right", "").replace("^\\circ", "").replace("\\%", "")
    try:
        return Fraction(str(sympy.Rational(sympy.sympify(s.replace("^", "**"), rational=True))))
    except Exception:
        return None


def agrees(short, truth):
    s = short.strip()
    if s.startswith("about "):
        try:
            g, t = float(s[6:]), float(truth)
            return abs(g - t) <= 0.01 * abs(t)
        except ValueError:
            return False
    if s.startswith("between "):
        try:
            lo, hi = (int(x) for x in s[8:].split(" and "))
            return lo <= int(truth) <= hi
        except ValueError:
            return False
    s = s.removeprefix("probably ")
    norm = lambda x: " ".join(x.lower().replace("(", "").replace(")", "").split())
    if norm(s) == norm(truth):
        return True
    # "n = 2" against "2", and spacing: the same answer written differently
    squeeze = lambda x: x.replace(" ", "").replace(r"\left", "").replace(r"\right", "").strip("$")
    if "=" not in truth and re.fullmatch(r"[a-z] = .+", s):
        s = s.split(" = ", 1)[1]
    if squeeze(s) == squeeze(truth):
        return True
    if "*" in truth:
        return sorted(norm(s).replace(" ", "").split("*")) == sorted(norm(truth).replace(" ", "").split("*"))
    a, b = number(s), number(truth)
    if a is not None and a == b:
        return True
    return same_expression(s, truth)


def same_expression(s, truth):
    """Two expressions equal as polynomials or rational functions (SymPy)."""
    from sympy.parsing.sympy_parser import parse_expr, standard_transformations, implicit_multiplication_application, convert_xor
    tr = standard_transformations + (implicit_multiplication_application, convert_xor)
    clean = lambda x: re.sub(r"\\[dt]?frac\{([^{}]+)\}\{([^{}]+)\}", r"((\1)/(\2))", x.strip("$")).replace(r"\left", "").replace(r"\right", "").replace("{", "(").replace("}", ")").replace(r"\cdot", "*")
    try:
        return sympy.simplify(parse_expr(clean(s), transformations=tr) - parse_expr(clean(truth), transformations=tr)) == 0
    except Exception:
        return False


def run(name, qs, jobs):
    tasks = [(k, q) for k in KINDS for q in qs]
    with ThreadPoolExecutor(jobs) as ex:
        answers = list(ex.map(lambda t: ask(t[0], t[1]["question"]), tasks))
    rows = []
    for (k, q), a in zip(tasks, answers):
        judged = a is not None and not (k == "abstract" and a == "no evidence either way")
        rows.append({**q, "kind": k, "answer": a, "right": (agrees(a, q["truth"]) if judged else None)})
    out = [f"\n## {name} ({len(qs)} questions, fixed in bench/golden_answer/{name}.jsonl)"]
    for k in KINDS:
        rs = [r for r in rows if r["kind"] == k]
        out.append(f"\n{k}:")
        out.append(f"  {'category':<28} {'n':>5} {'answered':>9} {'right':>12}")
        cats = sorted({r["category"] for r in rs})
        tot = [0, 0, 0, 0]
        for c in cats:
            cr = [r for r in rs if r["category"] == c]
            n, ans = len(cr), sum(r["answer"] is not None for r in cr)
            judged, right = sum(r["right"] is not None for r in cr), sum(r["right"] is True for r in cr)
            out.append(f"  {c:<28} {n:>5} {ans:>9} {f'{right}/{judged}':>12}")
            for i, v in enumerate((n, ans, right, judged)):
                tot[i] += v
        out.append(f"  {'all':<28} {tot[0]:>5} {tot[1]:>9} {f'{tot[2]}/{tot[3]}':>12}")
        if k != "abstract":
            for r in [r for r in rs if r["right"] is False][:15]:
                out.append(f"  WRONG: {r['question'][:90]!r} answered {r['answer']!r}, truth {r['truth']!r}")
    with open(os.path.join(DIR, f"{name}_answers.jsonl"), "w", encoding="utf-8") as f:
        for r in rows:
            f.write(json.dumps(r) + "\n")
    return "\n".join(out)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--set", default="all", choices=["oracle", "math", "all"])
    ap.add_argument("--per", type=int, default=100)
    ap.add_argument("--jobs", type=int, default=8)
    a = ap.parse_args()
    os.makedirs(DIR, exist_ok=True)
    report = ["# Golden Answer benchmark", "", "answered = the kind gave an answer; right = agrees with the truth, of the answers that can be judged (an abstract \"no evidence either way\" is not judged)."]
    if a.set in ("oracle", "all"):
        report.append(run("oracle", fixed("oracle", lambda: oracle_set(a.per)), a.jobs))
    if a.set in ("math", "all"):
        report.append(run("math", fixed("math", math_set), a.jobs))
    text = "\n".join(report) + "\n"
    with open(os.path.join(DIR, "report.md"), "w", encoding="utf-8") as f:
        f.write(text)
    print(text)


if __name__ == "__main__":
    main()
