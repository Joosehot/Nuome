# Nuome

**Ask a math question in plain words, get a worked solution where every step names its rule and every answer is proven.**

Nuome is a [HotMotor](https://github.com/Joosehot/HotMotor) engine: the same design as [FeelRight](https://github.com/Joosehot/FeelRight), [Senne](https://github.com/Joosehot/Senne) and [Aras](https://github.com/Joosehot/Aras), pointed at school math. No neural network runs while it works. The same sentence always gives a byte-identical solution. Every step traces to a rule and to the words that asked for it. A question it can't answer gets an explanation instead of a plausible-looking wrong answer.

```
$ nuome "solve 3(x - 2) + 4 = 2x - 1"
Solve 3(x - 2) + 4 = 2x - 1 for x

      3(x - 2) + 4 = 2x - 1
  1.  Multiply 3 into the brackets.   [distribute]
      3x - 6 + 4 = 2x - 1
  2.  Work out -6 + 4 = -2.   [fold]
      3x - 2 = 2x - 1
  3.  Subtract 2x from both sides.   [move_term]
      3x - 2 - 2x = -1
  4.  Collect the x terms.   [collect]
        3x - 2x = (3 - 2)x = x
      x - 2 = -1
  5.  Add 2 to both sides.   [move_term]
      x = -1 + 2
  6.  Work out -1 + 2 = 1.   [fold]
      x = 1

Answer: x = 1
  ok satisfies: x = 1: both sides are 1
  ok complete: counted from the linear equation itself: exactly one real solution
  ok working: every line of the working holds at every answer (6 steps)
```

## What v0 does

| task | words | examples |
|---|---|---|
| evaluate | what is, calculate, work out | `what is 3/4 + 1/6`, `15% of 80`, `sqrt 72`, `x^2 + 3x when x = -2` |
| simplify / expand | simplify, expand, multiply out | `(x + 1)^2 - x^2`, `(x + 2)(x - 3)`, `3x + 2y - x + 4y` |
| factor | factor, factorise | `2x^2 - 8`, `2x^2 + 7x + 3`, `x^4 - 16`, `4x^2 - 25y^2` |
| solve | solve, find x if | linear, quadratic, cubic with rational roots, simple rational equations |
| differentiate | differentiate, derivative of, d/dx | power, product, quotient and chain rules; sin, cos, tan, sec, e^x, a^x, ln, sqrt, arcsin, arctan; `at x = 2`; `second derivative of x^4` |
| integrate | integrate, integral of, antiderivative, `dx` | `3x^2 + 2x - 5`, `cos(3x)`, `x(x^2 + 1)^3` (substitution), `x e^x`, `ln x` (parts, LIATE), `1/(x^2 - 1)` (partial fractions), `cos^2 x`; `x^2 from 0 to 3` |
| limit | limit of ... as x approaches, lim x->2, tends to | `(x^2 - 4)/(x - 2)` at 2 (factor and cancel), `sin(3x)/(2x)` at 0, `(3x^2 + 1)/(x^2 - 5)` at infinity, L'Hopital |
| tangent | tangent to, tangent line to | `tangent to y = x^3 - 2x at x = 2` -> y = 10x - 16 |
| trig | (evaluate, simplify, solve) | `cos(45 degrees)`, `sin^2 x + cos^2 x`, `2 sin x cos x`, `solve 2cos x - 1 = 0`, `solve sin x = -1/2 for x between 0 and 2pi` |

Math can be typed (`2x^2 - 3x + 1 = 0`) or spoken (`x squared minus 4 equals 0`). `2x` means 2·x, `sin 2x` means sin(2x), and `-x^2` means -(x²).

## How it works

```
sentence -> lexicon -> parser -> beam search over solution paths -> checks -> worked solution
                                   rules offer steps, the profile and judges score them
```

1. **Lexicon** (`lexicon.rs`): a closed vocabulary. An unknown word is an error with the nearest known word (`solfe` → did you mean "solve"?), never a guess. `x` is always a letter, so `3 x 4` is refused with a hint to write `*`.
2. **Parser** (`parser.rs`): the sentence becomes a request (task, problem, letter, given values, method, profile words, decimal places), and each value remembers the words that produced it.
3. **Rules** (`src/rules/`, 51 files): one piece of math knowledge per file. Each offers moves, meaning steps a person would write down, with variants: the quadratic formula vs factoring vs square roots vs completing the square; `(a + b)^2` by the identity or as a product; arithmetic one operation at a time or all at once. When the same step applies in several places (the power rule on every term, "add 2 to both sides" in both alternatives), it is one step. Which rules each task may use is in `rules.toml`, so factoring never sees `distribute`, the rule that would undo it.
4. **Profile**: every variant is scored on brevity · clarity · elegance. `quickly`, `step by step`, `for a beginner` and `elegantly` shift the profile. The same question gets a different, still correct, solution:

   | x^2 - 4x + 1 = 0 | method |
   |---|---|
   | default | quadratic formula, 3 steps |
   | `elegantly` | complete the square, then square roots |
   | `expand (x + 3)^2 for a beginner` | write it as (x + 3)(x + 3), multiply out, collect |
   | `expand (x + 3)^2 quickly` | (a + b)^2 = a^2 + 2ab + b^2, one step |

5. **Judges** (`scoring.rs`) look at a step in the context of the whole path: don't make fractions early, expand brackets before moving terms, do pending arithmetic first, don't leave the letter with a minus sign, don't switch methods halfway, and work the alternatives in order.
6. **Beam search** (`search.rs`) extends every path by every move, drops any step that returns to an earlier state, and ranks paths by score minus estimated distance to done. Ties break on a fixed order, so the result is deterministic.
7. **Checks** (`checks.rs`) prove the answer independently of the steps that produced it:
   - **Solve:** every answer satisfies the original equation, exactly in rationals where possible. The real solutions are **counted from the equation's own polynomial** (rational root theorem + discriminant), so a lost root fails the check. Every line of the working holds at every answer.
   - **Evaluate:** the result equals the problem computed directly, exactly.
   - **Factor:** the product multiplies back out, and no factor has a rational root or a common factor left.
   - **Differentiate:** the result matches the slope measured numerically (a second or third derivative: the n-th central difference).
   - **Integrate:** differentiating the answer numerically gives back the integrand; every step keeps the derivative. A definite integral also equals Simpson's rule on the integrand (2000 panels), and an integrand undefined inside the interval is refused as improper.
   - **Limit:** the function evaluated ever closer to the point from both sides (or ever farther out) settles on the answer. Sides that disagree are a refusal that says so.
   - **Tangent:** the line touches the curve at the point with the curve's measured slope.
   - **Trig equations:** every answer satisfies the equation (a general solution for k = -3..3), and a fine scan of the interval, or of one turn, finds no solution the answer missed.
   - **Every step** of an expression keeps its value at the sample points.

   A path that fails a check is never shown; the next finalist is tried. If nothing passes, Nuome says why.

All numbers are exact rationals (`q.rs`). A result that would overflow is refused, never rounded.

## Honest refusals

```
$ nuome "solve x^5 + x + 1 = 0"
error: none of my rules can solve x^5 + x + 1 = 0
  hint: Nuome v0 solves linear, quadratic and simple rational equations; other kinds refuse rather than guess

$ nuome "factor x^4 + 4"
error: no way to factor x^4 + 4 passes the checks
  hint: factored: can't prove x^4 + 4 doesn't split into quadratics

$ nuome "solve x + 1 = 0 using the quadratic formula"
error: no way to solve x + 1 = 0 passes the checks
  hint: method: "using the quadratic formula" doesn't apply to this problem
```

`x^4 + 4 = (x^2 - 2x + 2)(x^2 + 2x + 2)`, and v0 has no rule for that split, so it says so instead of calling x^4 + 4 prime.

## Usage

```
nuome "solve x^2 - 5x + 6 = 0"
nuome "differentiate x^3 sin x step by step"
nuome "solve x^2 - 2 = 0 to 4 decimal places"
nuome --unicode "solve x^2 - 4x + 1 = 0"      # √, ·, ², ±
nuome --latex "factor x^2 - 9"
nuome --explain "solve x^2 + 3x = 4"          # tokens, request, profile, finalists, judges, checks
nuome -f examples/quadratic.txt               # one question per line
nuome --vocabulary                            # every word it knows
nuome --rules my-rules.toml "..."             # your own taste
```

## Taste lives in rules.toml

Every weight, every variant's axes, every judge, the beam width, the sample points and the rule set of each task are in `rules.toml`, validated at load (a misspelt rule or variant fails immediately). Change a number and the engine's taste changes, with no code change.

## Tests

```
cargo test                   # 62 unit tests (each rule: one case where it applies, one where it doesn't)
                             # + golden tests: examples/*.txt -> examples/out/*.txt
NUOME_BLESS=1 cargo test     # accept intended changes to the goldens
```

The golden test runs each example file three times to prove determinism and compares the result to the committed transcript. It also requires every question to pass every check and every line of `refusals.txt` to be refused.

## Next

- equation systems, inequalities
- stationary points (f'(x) = 0, classified by f''), one-sided limits, improper integrals, e^x sin x (parts that come back round)
- complex roots (v0 answers "no real solution")
- rational expression simplification: (x^2 - 1)/(x - 1) → x + 1 with the x ≠ 1 caveat
- factoring quartics into quadratics (x^4 + 4)
- a web page that shows the steps with LaTeX

## License

PolyForm Noncommercial 1.0.0, see [LICENSE](LICENSE). © 2026 Joose Hotari.
