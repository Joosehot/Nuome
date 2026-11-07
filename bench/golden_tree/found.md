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

46120 paths weighed in 0.4 s.

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
