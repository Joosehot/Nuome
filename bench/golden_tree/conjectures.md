# Golden's conjectures

From the number paths Golden Tree kept (seeds 1 to 20, none a shorter one lengthened, best first). For each rule f, the cycles the numbers 1 to 10000 fall into make the conjecture: every positive integer reaches one of them. Then Nuome decides it as far as it can: the forcing follows every residue class x = N*m + r symbolically until it descends; the search follows every n up to 10000000 until it falls below n or meets a cycle. Proved: every class descends and every n up to the bound reaches the cycles. Refuted: a cycle the conjecture does not name. Open: neither; open means Nuome did not decide it, and whether anyone has asked the same question is not checked. The drift is a heuristic (the mean growth in bits of one iteration over evenly spread residues).

## 10 open conjectures

1. Under f(x) = [x xor 14 ; x >> 1 ; if x ≡ 1 (mod 3): 5x], every positive integer reaches one of the cycles (5), (20 → 65 → 39), (35 → 110 → 48 → 155 → … , 21 members), (47 → 80), (582 → 1460 → 3665 → 1839 → 912 → 2315 → 1154), (1254 → 3140 → 7865 → 19655 → … , 65 members).
   open: true for every n up to 10000000; descent forced for 2990162 of 3145728 residue classes mod 3145728 (95.05%); drift -0.226 bits an iteration.

2. Under f(x) = [if x ≡ 0 (mod 4): 7x ; if x ≡ 0 (mod 2): x >> 2 ; x xor 1], every positive integer reaches the cycle (0 → 1).
   open: true for every n up to 10000000; descent forced for 4087808 of 4194304 residue classes mod 4194304 (97.46%); drift -0.298 bits an iteration.

3. Under f(x) = [if x ≡ 0 (mod 4): 7x ; x >> 2 ; if x ≡ 0 (mod 4): 2x], every positive integer reaches the cycle (0).
   open: true for every n up to 10000000; descent forced for 4151400 of 4194304 residue classes mod 4194304 (98.98%); drift -1.048 bits an iteration.

4. Under f(x) = [if x ≡ 0 (mod 2): 5x ; if x ≡ 0 (mod 5): x << 1 ; x >> 2], every positive integer reaches one of the cycles (0), (2 → 5).
   open: true for every n up to 21567; descent forced for 2253450 of 2621440 residue classes mod 2621440 (85.96%); 21568: its orbit passes 2^120 or 100 000 steps: a candidate for an orbit that never comes down; drift -0.239 bits an iteration.

5. Under f(x) = [if x ≡ 0 (mod 4): 7x ; x >> 1], every positive integer reaches the cycle (0).
   open: true for every n up to 10000000; descent forced for 4069736 of 4194304 residue classes mod 4194304 (97.03%); drift -0.298 bits an iteration.

6. Under f(x) = [x << 1 ; if x ≡ 1 (mod 5): 7x ; x >> 2], every positive integer reaches the cycle (0).
   open: true for every n up to 2973507; descent forced for 2522224 of 2621440 residue classes mod 2621440 (96.22%); 2973508: its orbit passes 2^120 or 100 000 steps: a candidate for an orbit that never comes down; drift -0.439 bits an iteration.

7. Under f(x) = [if x ≡ 0 (mod 2): x >> 2 ; if x ≡ 3 (mod 4): 7x ; x + 1], every positive integer reaches the cycle (1 → 2).
   open: true for every n up to 10000000; descent forced for 4112604 of 4194304 residue classes mod 4194304 (98.05%); drift +0.053 bits an iteration.

8. Under f(x) = [if x ≡ 1 (mod 2): x << 3 ; if x ≡ 0 (mod 2): x << 1 ; x >> 3 ; if x ≡ 1 (mod 3): 7x], every positive integer reaches one of the cycles (0), (3 → 6 → 7 → 14), (57 → 114 → 196 → 343 → … , 29 members), (75 → 150 → 259 → 518 → … , 13 members), (101 → 1414 → 353 → 4942 → … , 13 members).
   open: true for every n up to 2348786; descent forced for 2864824 of 3145728 residue classes mod 3145728 (91.07%); 2348787: its orbit passes 2^120 or 100 000 steps: a candidate for an orbit that never comes down; drift +0.436 bits an iteration.

9. Under f(x) = [if x ≡ 2 (mod 3): x xor 4 ; if x ≡ 0 (mod 2): 3x ; x >> 1], every positive integer reaches one of the cycles (0), (4 → 6 → 9), (16 → 24 → 36 → 54 → 81 → 40 → 60 → 90 → 135 → 67 → 33).
   open: true for every n up to 10000000; descent forced for 3063744 of 3145728 residue classes mod 3145728 (97.39%); drift -0.208 bits an iteration.

10. Under f(x) = [x << 1 ; if x ≡ 2 (mod 4): 3x ; x >> 2], every positive integer reaches one of the cycles (1), (5 → 7 → 10), (17 → 25 → 37 → 55 → 82 → 41 → 61 → 91 → 136 → 68 → 34).
   open: true for every n up to 10000000; descent forced for 4101082 of 4194304 residue classes mod 4194304 (97.78%); drift -0.208 bits an iteration.

## Decided on the way: 0 proved, 0 refuted

1 rules made no conjecture: an orbit of a number up to 10000 blew past 2^120.
