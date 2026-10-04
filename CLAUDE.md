# Nuome: working instructions

Rules for anyone (person or agent) who changes Nuome.

## A task without a module gets a module

When Nuome meets a kind of task that no module handles, the fix is a new
module for that kind of task, not a refusal, a detour through another
module, or a loose reading that guesses. Before building, check that no
module or instruction already covers it (`src/lib.rs` lists the modules).

Example: theorem-shaped questions (a remainder of a power by Fermat's little
theorem, the number of divisors by τ(n) = ∏(eᵢ + 1), sums of roots by
Vieta, a factorial mod p by Wilson) need their own module for theoretical
answers; the exact calculator is not that module.

## Golden Answer

`src/golden_answer.rs` (and `src/golden_exact.rs`) answer a mathematical
question in the kind the asker chooses:

- **logical**: derived from the question itself, every step checkable;
- **theoretical**: a claim from already known results, each one named;
- **abstract**: an answer that claims nothing. It is never scored right or
  wrong.

An answer never slips into another kind; when a kind has no answer it says
so and why.

Logical and theoretical answers are given only to a question read whole,
word for word around its math. There is no loose reading (key words plus
the first number): it answered questions that were not asked.

## Measuring

- Fixed sets only: `bench/golden_answer/oracle.jsonl` (truths from SymPy),
  `math.jsonl` (the MATH test split), `math_train.jsonl` (the MATH train
  split). A set is written once and never re-drawn.
- Develop on the train split. The MATH test split is the held-out measure:
  run it at the end, and change nothing in Golden Answer because of what it
  shows (only the judge's notation, and say so).
- Report per kind and category with n and the set's name:
  `python tools/golden_answer_bench.py --set all`.
- A wrong logical or theoretical answer is a bug: they claim to be right.

## Git

Commit locally. Push only when the owner asks.
