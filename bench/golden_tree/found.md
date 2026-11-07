# Paths Golden grew

Seeds 1 to 5, each 300 moves at render distance 4; the 5 best paths of each, every one iterated again on the CPU from scratch. Quiet: iterations of the quiet run; after: how complex what follows the change is (0 to 1). Starts: number paths from 2^40 - 1, 2^60 - 1, 2^40, 1000003 (0 to 3); row paths from a single cell, a block of 32 (0, 1). No rule is written into the engine.

## Seed 1

46120 paths weighed in 0.5 s.

| score | quiet | after | start | rule | checked |
|---|---|---|---|---|---|
| 2.956 | 30 | 0.90 | 1 | `x ↦ if x ≡ 0 (mod 3): x >> 2 ; 6x ; x >> 2` | the same |
| 2.820 | 60 | 0.83 | 1 | `x ↦ x << 2 ; x >> 1 ; if x ≡ 0 (mod 3): 7x ; x >> 2` | the same |
| 2.760 | 30 | 0.70 | 1 | `x ↦ if x ≡ 0 (mod 3): x >> 2 ; 3x` | the same |
| 2.742 | 20 | 0.94 | 2 | `x ↦ if x ≡ 0 (mod 3): x >> 2 ; x >> 2 ; 5x` | the same |
| 2.516 | 30 | 0.89 | 1 | `x ↦ if x ≡ 0 (mod 3): x >> 2 ; 3x ; if x ≡ 3 (mod 5): x >> 1 ; x >> 1` | the same |

## Seed 2

46120 paths weighed in 0.5 s.

| score | quiet | after | start | rule | checked |
|---|---|---|---|---|---|
| 3.179 | 40 | 0.89 | 2 | `x ↦ if x ≡ 2 (mod 3): x xor 4 ; if x ≡ 0 (mod 2): 3x ; x >> 1` | the same |
| 2.852 | 37 | 0.95 | 2 | `x ↦ x xor 15 ; x >> 1 ; if x ≡ 0 (mod 3): 7x ; if x ≡ 3 (mod 4): x + 8` | the same |
| 2.600 | 58 | 0.77 | 1 | `x ↦ x xor 15 ; x xor 6 ; if x ≡ 2 (mod 4): x >> 1 ; if x ≡ 1 (mod 4): 3x` | the same |
| 2.543 | 20 | 0.87 | 1 | `x ↦ if x ≡ 2 (mod 3): x xor 4 ; x >> 3 ; 7x` | the same |
| 2.386 | 40 | 0.89 | 2 | `x ↦ if x ≡ 2 (mod 3): x xor 4 ; if x ≡ 0 (mod 2): 3x ; x >> 1 ; if x ≡ 3 (mod 5): x << 2 ; if x ≡ 2 (mod 5): x xor 2` | the same |

## Seed 3

46120 paths weighed in 0.5 s.

| score | quiet | after | start | rule | checked |
|---|---|---|---|---|---|
| 3.315 | 40 | 0.93 | 2 | `x ↦ if x ≡ 0 (mod 2): 5x ; if x ≡ 0 (mod 5): x << 1 ; x >> 2` | the same |
| 2.824 | 45 | 0.89 | 2 | `x ↦ if x ≡ 0 (mod 2): 5x ; x >> 1 ; if x ≡ 1 (mod 4): 7x ; if x ≡ 1 (mod 2): x >> 3` | the same |
| 2.666 | 40 | 0.87 | 1 | `x ↦ if x ≡ 0 (mod 2): 5x ; if x ≡ 1 (mod 3): x >> 2 ; x xor 3 ; x >> 1` | the same |
| 2.536 | 39 | 0.71 | 1 | `x ↦ if x ≡ 0 (mod 2): 5x ; x >> 2 ; if x ≡ 3 (mod 4): x << 1` | the same |
| 2.525 | 48 | 0.90 | 2 | `x ↦ if x ≡ 0 (mod 2): 5x ; x >> 1 ; if x ≡ 1 (mod 4): 7x ; if x ≡ 1 (mod 2): x >> 3 ; if x ≡ 4 (mod 5): x >> 1` | the same |

## Seed 4

46120 paths weighed in 0.5 s.

| score | quiet | after | start | rule | checked |
|---|---|---|---|---|---|
| 2.364 | 19 | 0.82 | 2 | `x ↦ if x ≡ 1 (mod 2): x + 1 ; x >> 2 ; if x ≡ 0 (mod 2): 7x` | the same |
| 2.335 | 19 | 0.95 | 2 | `x ↦ if x ≡ 1 (mod 2): x + 1 ; x >> 2 ; if x ≡ 0 (mod 2): 7x ; if x ≡ 2 (mod 3): x + 7` | the same |
| 2.303 | 19 | 0.93 | 2 | `x ↦ if x ≡ 1 (mod 2): x + 1 ; if x ≡ 4 (mod 5): x << 2 ; if x ≡ 0 (mod 4): 9x ; x >> 3` | the same |
| 2.284 | 29 | 0.93 | 1 | `x ↦ if x ≡ 1 (mod 2): x + 1 ; x >> 2 ; if x ≡ 0 (mod 2): 7x ; if x ≡ 2 (mod 3): x + 7 ; x xor 15` | the same |
| 2.056 | 39 | 0.77 | 2 | `x ↦ if x ≡ 1 (mod 2): x + 1 ; 3x ; if x ≡ 3 (mod 4): 8x ; x >> 1 ; if x ≡ 1 (mod 4): x >> 3` | the same |

## Seed 5

46120 paths weighed in 0.5 s.

| score | quiet | after | start | rule | checked |
|---|---|---|---|---|---|
| 3.052 | 20 | 0.87 | 1 | `x ↦ x >> 3 ; 7x` | the same |
| 2.569 | 20 | 0.88 | 1 | `x ↦ x >> 3 ; 7x ; if x ≡ 3 (mod 5): x + 3` | the same |
| 2.518 | 37 | 0.84 | 2 | `x ↦ x >> 3 ; if x ≡ 1 (mod 2): x >> 1 ; if x ≡ 0 (mod 2): 3x ; x << 2` | the same |
| 2.469 | 20 | 0.84 | 1 | `x ↦ x >> 3 ; 7x ; if x ≡ 1 (mod 5): x xor 14` | the same |
| 2.315 | 20 | 0.66 | 1 | `x ↦ x >> 3 ; 5x` | the same |

Of the 25 kept, 25 are number paths and 0 row paths.

## Ten equations

From seeds 1 to 20 (each as above), the ten best that are not a shorter one lengthened; each iterated again on the CPU from scratch.

| # | equation | quiet | after | score | seed | checked |
|---|---|---|---|---|---|---|
| 1 | `x ↦ x xor 14 ; x >> 1 ; if x ≡ 1 (mod 3): 5x` | 57 | 0.90 | 3.531 | 19 | the same |
| 2 | `x ↦ if x ≡ 0 (mod 4): 7x ; if x ≡ 0 (mod 2): x >> 2 ; x xor 1` | 39 | 0.94 | 3.326 | 20 | the same |
| 3 | `x ↦ if x ≡ 0 (mod 4): 7x ; x >> 2 ; if x ≡ 0 (mod 4): 2x` | 38 | 0.94 | 3.325 | 20 | the same |
| 4 | `x ↦ if x ≡ 0 (mod 2): 5x ; if x ≡ 0 (mod 5): x << 1 ; x >> 2` | 40 | 0.93 | 3.315 | 3 | the same |
| 5 | `x ↦ if x ≡ 0 (mod 4): 7x ; x >> 1` | 39 | 0.77 | 3.296 | 20 | the same |
| 6 | `x ↦ x << 1 ; if x ≡ 1 (mod 5): 7x ; x >> 2` | 60 | 0.83 | 3.276 | 13 | the same |
| 7 | `x ↦ if x ≡ 0 (mod 2): x >> 2 ; if x ≡ 3 (mod 4): 7x ; x + 1` | 59 | 0.83 | 3.269 | 14 | the same |
| 8 | `x ↦ if x ≡ 1 (mod 2): x << 3 ; if x ≡ 0 (mod 2): x << 1 ; x >> 3 ; if x ≡ 1 (mod 3): 7x` | 75 | 0.90 | 3.202 | 8 | the same |
| 9 | `x ↦ x >> 1 ; if x ≡ 1 (mod 2): x xor 10 ; if x ≡ 2 (mod 3): 7x` | 39 | 0.90 | 3.189 | 18 | the same |
| 10 | `x ↦ if x ≡ 2 (mod 3): x xor 4 ; if x ≡ 0 (mod 2): 3x ; x >> 1` | 40 | 0.89 | 3.179 | 2 | the same |

## Ten equations in two numbers

Pair paths (x, y): sums, differences and products of each other, squares, swaps, conditions; seeds 1 to 20 grown in this world only, the ten best that are not a shorter one lengthened, each iterated again on the CPU from scratch. Starts: (1, 1), (2^40 - 1, 1), (3, 2^20), (1000003, 7) (0 to 3).

| # | equation | quiet | after | score | seed | start | checked |
|---|---|---|---|---|---|---|---|
| 1 | `(x, y) ↦ swap x, y ; x ← x mod 222 ; x ← 4x ; x ← |x − y|` | 2210 | 0.73 | 4.638 | 6 | 3 | the same |
| 2 | `(x, y) ↦ y ← y + x ; swap x, y ; if x ≡ 1 (mod 2): x ← x xor 13 ; x ← x mod 191 ; x ← |x − y| ; y ← y + x` | 7855 | 0.72 | 4.155 | 14 | 3 | the same |
| 3 | `(x, y) ↦ swap x, y ; x ← x mod 222 ; x ← 4x ; if x ≡ 3 (mod 4): x ← 5x ; x ← |x − y|` | 2210 | 0.73 | 4.058 | 6 | 3 | the same |
| 4 | `(x, y) ↦ y ← y + x ; swap x, y ; if x ≡ 1 (mod 2): x ← x xor 13 ; x ← x mod 191 ; if x ≡ 2 (mod 4): x ← x << 3 ; x ← |x − y| ; y ← y + x` | 7855 | 0.76 | 3.944 | 14 | 3 | the same |
| 5 | `(x, y) ↦ swap x, y ; x ← 9x ; x ← x mod 136 ; x ← 7x ; x ← |x − y|` | 1867 | 0.68 | 3.701 | 17 | 3 | the same |
| 6 | `(x, y) ↦ x ← 6x ; x ← x >> 2 ; x ← 8x ; x ← |x − y| ; swap x, y ; x ← x mod 231` | 853 | 0.84 | 3.614 | 12 | 2 | the same |
| 7 | `(x, y) ↦ swap x, y ; x ← x mod 225 ; x ← 6x ; if x ≡ 0 (mod 5): x ← x >> 3 ; x ← |x − y|` | 1551 | 0.67 | 3.558 | 17 | 3 | the same |
| 8 | `(x, y) ↦ y ← y + x ; swap x, y ; x ← x mod 161 ; swap x, y ; x ← |x − y| ; x ← |x − y| ; x ← |x − y|` | 6212 | 0.68 | 3.406 | 15 | 3 | the same |
| 9 | `(x, y) ↦ y ← y + x ; swap x, y ; x ← x + y ; x ← x mod 100 ; x ← 6x ; x ← |x − y|` | 3334 | 0.65 | 3.404 | 5 | 3 | the same |
| 10 | `(x, y) ↦ swap x, y ; x ← x mod 168 ; x ← 2x ; x ← |x − y|` | 5957 | 0.44 | 3.181 | 19 | 3 | the same |
