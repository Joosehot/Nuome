"""Check the chains fish-invent wrote (out/fish_invent_chains.tsv) with other tools:
floor(g(A, n)) recomputed with mpmath at the digits A is known to, the primes by SymPy."""
import math, sys
import mpmath, sympy

def g(shape, A, n):
    f = math.factorial
    m = mpmath.mpf
    table = {
        'A (n 4)!': lambda: A * f(4 * n), 'A (n 3)!': lambda: A * f(3 * n), 'A (n)!^(4)': lambda: A * m(f(n)) ** 4,
        'A (n)!^(3)': lambda: A * m(f(n)) ** 3, 'A ((n + n))!': lambda: A * f(2 * n), 'A (n)!^(2)': lambda: A * m(f(n)) ** 2,
        'A^(n n)': lambda: A ** (n * n), 'A ((n)!)^(n)': lambda: A * m(f(n)) ** n, '(A (n)!)^(n)': lambda: (A * f(n)) ** n,
        '(A + (n)!)^(n)': lambda: (A + f(n)) ** n, 'A (n n)!': lambda: A * f(n * n), 'n A^(n)': lambda: n * A ** n,
        'A^(2^(n))': lambda: A ** (2 ** n), 'A^(n^(3))': lambda: A ** (n ** 3), '(n)! A^(n)': lambda: f(n) * A ** n,
        'A n^(n)': lambda: A * m(n) ** n, '(n + A (n)!)': lambda: n + A * f(n), '(n + A^((n)!))': lambda: n + A ** f(n),
        'A^(3^(n))': lambda: A ** (3 ** n), 'A (2^(n))!': lambda: A * f(2 ** n), '(4 + A (n)!)': lambda: 4 + A * f(n),
        'n A^((n)!)': lambda: n * A ** f(n), 'A^(2 (n)!)': lambda: A ** (2 * f(n)), '(3 + A^((n)!))': lambda: 3 + A ** f(n),
        'A 2^((n)!)': lambda: A * m(2) ** f(n),
        '(A (n)!)^(4)': lambda: (A * f(n)) ** 4, '(A (n)!)^(3)': lambda: (A * f(n)) ** 3, '(A (n)!)^(2)': lambda: (A * f(n)) ** 2,
        'A (n)!^(n)': lambda: A * m(f(n)) ** n, '((A + (n)!))^(n)': lambda: (A + f(n)) ** n, '(n A)^(n)': lambda: (n * A) ** n,
    }
    return table[shape]()

rows = [l.rstrip('\n').split('\t') for l in open('out/fish_invent_chains.tsv', encoding='utf-8')]
for shape, a, primes in rows:
    primes = [int(x) for x in primes.split()]
    mpmath.mp.dps = len(a) + 50
    A = mpmath.mpf(a)
    ok_formula = ok_prime = 0
    first_bad = None
    known = True
    for n, p in enumerate(primes, 1):
        try:
            v = int(mpmath.floor(g(shape, A, n)))
        except KeyError:
            known = False
            break
        if v == p:
            ok_formula += 1
        elif first_bad is None:
            first_bad = (n, v, p)
        if sympy.isprime(p):
            ok_prime += 1
    if not known:
        print(f"{shape:<18} (shape not in the checker's table)")
        continue
    print(f"{shape:<18} A {a[:14]}... ({len(a) - 2} digits): formula {ok_formula}/{len(primes)} match, primes {ok_prime}/{len(primes)} by sympy" + (f"  FIRST MISMATCH n={first_bad[0]}: got {str(first_bad[1])[:20]}, chain {str(first_bad[2])[:20]}" if first_bad else ""))
