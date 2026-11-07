# Problems Golden invented that bridge to an open question

Every object of the family solves the problem, so "impossible" at n would say the object does not exist on n points. Open means Nuome did not decide it. Whether anyone has asked the same question, and how fruitful it is, is not checked.

## 1

For which n is there a tournament on n points with F(x, y) = 0 for every two distinct points x, y, where F(x, y) = (|(beat_both(x,y) - (n + -3))| * (both_beat(x,y) - max(beats(x,y), beat_both(x,y))))?

  bridge: every doubly regular tournament solves this (its readings to n = 4003; Paley's objects at n = [7, 11, 19, 23] checked whole)

  kin, neither the family nor the family with one point added: n = 9 with 1 symmetries, n = 11 with 1 symmetries, n = 13 with 1 symmetries, n = 15 with 1 symmetries, n = 17 with 1 symmetries, n = 19 with 1 symmetries, n = 21 with 1 symmetries, n = 23 with 1 symmetries

  open question moved within reach: none; the family is settled to n = 31

  the smallest kin, n = 9: degrees [4, 4, 4, 4, 4, 4, 4, 4, 4]; rows [30, 60, 232, 368, 452, 401, 163, 267, 71]

  reading: beats(x,y) is 1 when x beats y, out(x) counts whom x beats, both_beat(x,y) whom both beat, beat_both(x,y) who beats both, paths(x,y) the z with x beats z beats y

  built and checked: n = 7, 9, 11, 13, 15, 17, 19, 21, 23

  impossible, every structure ruled out: n = 4, 5, 6, 8, 10, 12, 14, 16, 18, 20, 22, 24

  not solved: the parts grew too many at 9 points; parts of each size from 1: [1, 1, 2, 5, 16, 71, 534, 7419, 196977]

  the smallest built, n = 7: 0>1 0>2 0>3 1>2 1>4 1>5 2>3 2>4 2>6 3>1 3>5 3>6 4>0 4>3 4>5 5>0 5>2 5>6 6>0 6>1 6>4

  the search's verdicts tried again one structure at a time, with no search: 4 of the n's above agree (the larger have too many structures)

## 2

For which n is there a tournament on n points with F(x, y) = 0 for every two distinct points x, y, where F(x, y) = (|((4 * both_beat(x,y)) - (n + -3))| * (min(9, both_beat(x,y)) - max(beats(x,y), max(|beat_both(x,y)|, (out(y) * -3)))))?

  bridge: every doubly regular tournament solves this (its readings to n = 4003; Paley's objects at n = [7, 11, 19, 23] checked whole)

  kin, neither the family nor the family with one point added: n = 9 with 1 symmetries, n = 11 with 1 symmetries, n = 13 with 1 symmetries, n = 15 with 1 symmetries, n = 17 with 1 symmetries, n = 19 with 1 symmetries, n = 21 with 1 symmetries, n = 23 with 1 symmetries

  open question moved within reach: none; the family is settled to n = 31

  the smallest kin, n = 9: degrees [4, 4, 4, 4, 4, 4, 4, 4, 4]; rows [30, 60, 232, 368, 452, 401, 163, 267, 71]

  reading: beats(x,y) is 1 when x beats y, out(x) counts whom x beats, both_beat(x,y) whom both beat, beat_both(x,y) who beats both, paths(x,y) the z with x beats z beats y

  built and checked: n = 7, 9, 11, 13, 15, 17, 19, 21, 23

  impossible, every structure ruled out: n = 4, 5, 6, 8, 10, 12, 14, 16, 18, 20, 22

  open: n = 24 (the sieve's search, its climb and the parts did not decide)

  not solved: the parts grew too many at 9 points; parts of each size from 1: [1, 1, 2, 5, 16, 71, 534, 7419, 196977]

  the smallest built, n = 7: 0>1 0>2 0>3 1>2 1>4 1>5 2>3 2>4 2>6 3>1 3>5 3>6 4>0 4>3 4>5 5>0 5>2 5>6 6>0 6>1 6>4

  forced at n = 24: 0 of 552 free cells; the choice left is the other 552

  the search's verdicts tried again one structure at a time, with no search: 4 of the n's above agree (the larger have too many structures)

## 3

For which n is there a graph on n points with F(x, y) = 0 for every two distinct points x, y, where F(x, y) = (|((2 * deg(x)) - (n + -1))| * (if adj(x,y) != 0 then |((common(x,y) * common(x,y)) - ((if ((|deg(x)| + (deg(x) + deg(x))) - (n + -1)) != 0 then deg(x) else -2) + (common(x,y) - -2)))| else |((4 * common(x,y)) - 4)|))?

  bridge: every conference graph solves this (its readings to n = 4003; Paley's objects at n = [5, 13, 17, 29] checked whole)

  kin, neither the family nor the family with one point added: n = 7 with 4 symmetries, n = 9 with 16 symmetries, n = 10 with 120 symmetries, n = 11 with 28800 symmetries, n = 13 with 1152 symmetries, n = 17 with 5760 symmetries, n = 21 with 2903040 symmetries

  open question moved within reach: none; the family is settled to n = 49

  the smallest kin, n = 7: degrees [3, 3, 3, 3, 3, 2, 3]; rows [14, 21, 35, 81, 74, 68, 56]

  reading: adj(x,y) is 1 when x and y are joined, common(x,y) counts the points joined to both

  built and checked: n = 7, 9, 10, 11, 13, 17, 21

  impossible, every structure ruled out: n = 3, 4, 6, 8, 12, 14, 16, 18, 20, 22, 24

  open: n = 15, 19, 23 (the sieve's search, its climb and the parts did not decide)

  not solved: the parts grew too many at 9 points; parts of each size from 1: [1, 2, 4, 11, 42, 220, 1556, 17774, 349148]

  the smallest built, n = 7: 0-1 0-2 0-3 1-2 1-4 2-5 3-4 3-6 4-6 5-6

  forced at n = 15: 0 of 210 free cells; the choice left is the other 210

  the search's verdicts tried again one structure at a time, with no search: 4 of the n's above agree (the larger have too many structures)

## 4

For which n is there a graph on n points with F(x, y) = 0 for every two distinct points x, y, where F(x, y) = (|((2 * deg(x)) - (n + -1))| * (if adj(x,y) != 0 then |((common(x,y) * common(x,y)) - ((if ((2 * deg(x)) - deg(y)) != 0 then ((2 * deg(x)) - (n + -1)) else -2) + (common(x,y) - -2)))| else |((4 * common(x,y)) - 4)|))?

  bridge: every conference graph solves this (its readings to n = 4003; Paley's objects at n = [5, 13, 17, 29] checked whole)

  kin, neither the family nor the family with one point added: n = 7 with 4 symmetries, n = 9 with 16 symmetries, n = 11 with 28800 symmetries, n = 13 with 1152 symmetries, n = 17 with 5760 symmetries, n = 21 with 2903040 symmetries

  open question moved within reach: none; the family is settled to n = 49

  the smallest kin, n = 7: degrees [3, 3, 3, 3, 3, 2, 3]; rows [14, 21, 35, 81, 74, 68, 56]

  reading: adj(x,y) is 1 when x and y are joined, common(x,y) counts the points joined to both

  built and checked: n = 7, 9, 11, 13, 17, 21

  impossible, every structure ruled out: n = 3, 4, 6, 8, 10, 12, 14, 16, 18, 20, 22, 24

  open: n = 15, 19, 23 (the sieve's search, its climb and the parts did not decide)

  not solved: the parts grew too many at 9 points; parts of each size from 1: [1, 2, 4, 11, 42, 220, 1556, 17774, 349148]

  the smallest built, n = 7: 0-1 0-2 0-3 1-2 1-4 2-5 3-4 3-6 4-6 5-6

  forced at n = 15: 0 of 210 free cells; the choice left is the other 210

  the search's verdicts tried again one structure at a time, with no search: 4 of the n's above agree (the larger have too many structures)

## 5

For which n is there a graph on n points with F(x, y) = 0 for every two distinct points x, y, where F(x, y) = ((|((2 * deg(x)) - (n + -1))| * |(common(x,y) - (n + -5))|) * (n - common(x,y)))?

  bridge: every conference graph solves this (its readings to n = 4003; Paley's objects at n = [5, 13, 17, 29] checked whole)

  kin, neither the family nor the family with one point added: n = 7 with 72 symmetries, n = 9 with 16 symmetries, n = 13 with 1152 symmetries, n = 17 with 5760 symmetries, n = 21 with 2903040 symmetries

  open question moved within reach: none; the family is settled to n = 49

  the smallest kin, n = 7: degrees [6, 3, 3, 3, 3, 3, 3]; rows [126, 13, 11, 7, 97, 81, 49]

  reading: adj(x,y) is 1 when x and y are joined, common(x,y) counts the points joined to both

  built and checked: n = 7, 9, 13, 17, 21

  impossible, every structure ruled out: n = 3, 4, 6, 8, 10, 12, 14, 16, 18, 20, 22, 24

  open: n = 11, 15, 19, 23 (the sieve's search, its climb and the parts did not decide)

  not solved: the parts grew too many at 9 points; parts of each size from 1: [1, 2, 4, 11, 42, 220, 1556, 17774, 349148]

  the smallest built, n = 7: 0-1 0-2 0-3 0-4 0-5 0-6 1-2 1-3 2-3 4-5 4-6 5-6

  forced at n = 11: 0 of 110 free cells; the choice left is the other 110

  the search's verdicts tried again one structure at a time, with no search: 4 of the n's above agree (the larger have too many structures)

## 6

For which n is there a graph on n points with F(x, y) = 0 for every two distinct points x, y, where F(x, y) = (|((2 * deg(x)) - (n + -1))| * ((4 * common(x,y)) - (4 + |((deg(y) * common(x,y)) - (n + -5))|)))?

  bridge: every conference graph solves this (its readings to n = 4003; Paley's objects at n = [5, 13, 17, 29] checked whole)

  kin, neither the family nor the family with one point added: n = 7 with 72 symmetries, n = 9 with 1152 symmetries, n = 13 with 1152 symmetries, n = 17 with 1 symmetries

  open question moved within reach: none; the family is settled to n = 49

  the smallest kin, n = 7: degrees [6, 3, 3, 3, 3, 3, 3]; rows [126, 13, 11, 7, 97, 81, 49]

  reading: adj(x,y) is 1 when x and y are joined, common(x,y) counts the points joined to both

  built and checked: n = 7, 9, 13, 17

  impossible, every structure ruled out: n = 3, 4, 6, 8, 10, 12, 14, 16, 18, 20, 22, 24

  open: n = 11, 15, 19, 21, 23 (the sieve's search, its climb and the parts did not decide)

  not solved: the parts grew too many at 9 points; parts of each size from 1: [1, 2, 4, 11, 42, 220, 1556, 17774, 349148]

  the smallest built, n = 7: 0-1 0-2 0-3 0-4 0-5 0-6 1-2 1-3 2-3 4-5 4-6 5-6

  forced at n = 11: 0 of 110 free cells; the choice left is the other 110

  the search's verdicts tried again one structure at a time, with no search: 4 of the n's above agree (the larger have too many structures)

## 7

For which n is there a tournament on n points with F(x, y) = 0 for every two distinct points x, y, where F(x, y) = ((((4 * beat_both(x,y)) - (n + -3)) * |out(x)|) * (out(y) - (both_beat(x,y) + both_beat(x,y))))?

  bridge: every doubly regular tournament solves this (its readings to n = 4003; Paley's objects at n = [7, 11, 19, 23] checked whole)

  kin, neither the family nor the family with one point added: n = 7 with 3 symmetries, n = 11 with 5 symmetries

  open question moved within reach: none; the family is settled to n = 31

  the smallest kin, n = 7: degrees [4, 4, 3, 0, 3, 4, 3]; rows [30, 108, 56, 0, 74, 89, 13]

  reading: beats(x,y) is 1 when x beats y, out(x) counts whom x beats, both_beat(x,y) whom both beat, beat_both(x,y) who beats both, paths(x,y) the z with x beats z beats y

  built and checked: n = 4, 7, 8, 11, 12, 16, 20

  impossible, every structure ruled out: n = 5, 6, 9, 10, 13, 14, 17, 18

  open: n = 15, 19, 21, 22 (the sieve's search, its climb and the parts did not decide)

  not solved: the parts grew too many at 9 points; parts of each size from 1: [1, 1, 2, 5, 16, 71, 534, 7419, 196977]

  the smallest built, n = 4: 0>1 0>2 1>2 1>3 3>0 3>2

  forced at n = 15: 0 of 210 free cells; the choice left is the other 210

  the search's verdicts tried again one structure at a time, with no search: 4 of the n's above agree (the larger have too many structures)

## 8

For which n is there a tournament on n points with F(x, y) = 0 for every two distinct points x, y, where F(x, y) = (|((4 * beat_both(x,y)) - (n + -3))| * (min(both_beat(x,y), 2) - max(beats(x,y), beat_both(x,y))))?

  bridge: every doubly regular tournament solves this (its readings to n = 4003; Paley's objects at n = [7, 11, 19, 23] checked whole)

  kin, neither the family nor the family with one point added: n = 9 with 9 symmetries

  open question moved within reach: none; the family is settled to n = 31

  the smallest kin, n = 9: degrees [4, 4, 4, 4, 4, 4, 4, 4, 4]; rows [30, 108, 184, 368, 450, 209, 389, 267, 39]

  reading: beats(x,y) is 1 when x beats y, out(x) counts whom x beats, both_beat(x,y) whom both beat, beat_both(x,y) who beats both, paths(x,y) the z with x beats z beats y

  built and checked: n = 7, 9, 11, 15

  impossible, every structure ruled out: n = 4, 5, 6, 8, 10, 12, 13, 14, 16, 17, 18

  open: n = 19, 20 (the sieve's search, its climb and the parts did not decide)

  not solved: the parts grew too many at 9 points; parts of each size from 1: [1, 1, 2, 5, 16, 71, 534, 7419, 196977]

  the smallest built, n = 7: 0>1 0>2 0>3 1>2 1>4 1>5 2>3 2>4 2>6 3>1 3>5 3>6 4>0 4>3 4>5 5>0 5>2 5>6 6>0 6>1 6>4

  forced at n = 19: 0 of 342 free cells; the choice left is the other 342

  the search's verdicts tried again one structure at a time, with no search: 4 of the n's above agree (the larger have too many structures)
