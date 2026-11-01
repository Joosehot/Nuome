# Problems Golden invented, solved by Nuome

Each was undecided by the sieve's short search when Golden kept it; then Nuome worked on it, with nothing from outside. Open means Nuome did not decide it. Whether anyone has asked or settled the same question is not checked.

## 1

For which n is there a tournament on n points with F(x, y) = 0 for every two distinct points x, y, where F(x, y) = ((-1 + both_beat(x,y)) * (-1 + beat_both(x,y)))?

  reading: beats(x,y) is 1 when x beats y, out(x) counts whom x beats, both_beat(x,y) whom both beat, beat_both(x,y) who beats both, paths(x,y) the z with x beats z beats y

  built and checked: n = 4, 6, 7

  impossible, every structure ruled out: n = 3, 5, 8, 9, 10

  SOLVED for every n: no part of 11 points can be in such a structure, so none on 11 points or more; parts of each size from 1: [1, 1, 2, 5, 16, 55, 99, 76, 52, 30, 0]

  the smallest built, n = 4: 0>1 0>2 0>3 1>2 2>3 3>1

  the search's verdicts tried again one structure at a time, with no search: 4 of the n's above agree (the larger have too many structures)

## 2

For which n is there a tournament on n points with F(x, y) = 0 for every two distinct points x, y, where F(x, y) = ((if both_beat(x,y) != 0 then (paths(x,y) * beat_both(x,y)) else -2) * (-1 + beat_both(x,y)))?

  reading: beats(x,y) is 1 when x beats y, out(x) counts whom x beats, both_beat(x,y) whom both beat, beat_both(x,y) who beats both, paths(x,y) the z with x beats z beats y

  built and checked: n = 4, 6, 7

  impossible, every structure ruled out: n = 3, 5, 8

  SOLVED for every n: no part of 9 points can be in such a structure, so none on 9 points or more; parts of each size from 1: [1, 1, 2, 5, 16, 18, 9, 6, 0]

  the smallest built, n = 4: 0>1 0>2 0>3 1>2 2>3 3>1

  the search's verdicts tried again one structure at a time, with no search: 4 of the n's above agree (the larger have too many structures)

## 3

For which n is there a tournament on n points with F(x, y) = 0 for every two distinct points x, y, where F(x, y) = ((1 - both_beat(x,y)) * ((beat_both(x,y) + 3) - max(both_beat(x,y), 4)))?

  reading: beats(x,y) is 1 when x beats y, out(x) counts whom x beats, both_beat(x,y) whom both beat, beat_both(x,y) who beats both, paths(x,y) the z with x beats z beats y

  built and checked: n = 4, 6, 7

  impossible, every structure ruled out: n = 3, 5, 8, 9, 10, 11, 12, 13

  open: n = 14, 15 (the sieve's search, its climb and the parts did not decide)

  not solved: the parts grew too many at 9 points; parts of each size from 1: [1, 1, 2, 5, 16, 71, 534, 7419, 196977]

  the smallest built, n = 4: 0>1 0>2 0>3 1>2 2>3 3>1

  forced at n = 14: 0 of 182 free cells; the choice left is the other 182

  the search's verdicts tried again one structure at a time, with no search: 4 of the n's above agree (the larger have too many structures)

## 4

For which n is there a tournament on n points with F(x, y) = 0 for every two distinct points x, y, where F(x, y) = ((1 - both_beat(x,y)) * ((beat_both(x,y) + 3) - max(paths(x,y), 4)))?

  reading: beats(x,y) is 1 when x beats y, out(x) counts whom x beats, both_beat(x,y) whom both beat, beat_both(x,y) who beats both, paths(x,y) the z with x beats z beats y

  built and checked: n = 4, 6, 7

  impossible, every structure ruled out: n = 3, 5, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17

  open: n = 18, 19 (the sieve's search, its climb and the parts did not decide)

  not solved: the parts grew too many at 9 points; parts of each size from 1: [1, 1, 2, 5, 16, 71, 534, 7419, 196977]

  the smallest built, n = 4: 0>1 0>2 0>3 1>2 2>3 3>1

  forced at n = 18: 0 of 306 free cells; the choice left is the other 306

  the search's verdicts tried again one structure at a time, with no search: 4 of the n's above agree (the larger have too many structures)

## 5

For which n is there a tournament on n points with F(x, y) = 0 for every two distinct points x, y, where F(x, y) = ((2 * (-1 + beat_both(x,y))) * (beats(x,y) * (-1 + both_beat(x,y))))?

  reading: beats(x,y) is 1 when x beats y, out(x) counts whom x beats, both_beat(x,y) whom both beat, beat_both(x,y) who beats both, paths(x,y) the z with x beats z beats y

  built and checked: n = 4, 6, 7

  impossible, every structure ruled out: n = 3, 5, 8, 9, 10

  SOLVED for every n: no part of 11 points can be in such a structure, so none on 11 points or more; parts of each size from 1: [1, 1, 2, 5, 16, 55, 99, 76, 52, 30, 0]

  the smallest built, n = 4: 0>1 0>2 0>3 1>2 2>3 3>1

  the search's verdicts tried again one structure at a time, with no search: 4 of the n's above agree (the larger have too many structures)

## 6

For which n is there a tournament on n points with F(x, y) = 0 for every two distinct points x, y, where F(x, y) = (max((out(y) * beat_both(x,y)), out(y)) - max((out(y) * both_beat(x,y)), (out(y) * beat_both(x,y))))?

  reading: beats(x,y) is 1 when x beats y, out(x) counts whom x beats, both_beat(x,y) whom both beat, beat_both(x,y) who beats both, paths(x,y) the z with x beats z beats y

  built and checked: n = 4, 6, 7, 9, 11, 13, 15, 17

  impossible, every structure ruled out: n = 3, 5, 8

  open: n = 10, 12, 14, 16, 18, 19 (the sieve's search, its climb and the parts did not decide)

  not solved: the parts grew too many at 9 points; parts of each size from 1: [1, 1, 2, 5, 16, 71, 534, 7419, 196977]

  the smallest built, n = 4: 0>1 0>2 0>3 1>2 2>3 3>1

  forced at n = 10: 0 of 90 free cells; the choice left is the other 90

  the search's verdicts tried again one structure at a time, with no search: 4 of the n's above agree (the larger have too many structures)

## 7

For which n is there a tournament on n points with F(x, y) = 0 for every two distinct points x, y, where F(x, y) = (max((out(x) * beat_both(x,y)), (out(y) * both_beat(x,y))) - max((out(y) * both_beat(x,y)), out(x)))?

  reading: beats(x,y) is 1 when x beats y, out(x) counts whom x beats, both_beat(x,y) whom both beat, beat_both(x,y) who beats both, paths(x,y) the z with x beats z beats y

  built and checked: n = 4, 6, 7, 9

  impossible, every structure ruled out: n = 3, 5, 8, 10

  open: n = 11, 12 (the sieve's search, its climb and the parts did not decide)

  not solved: the parts grew too many at 9 points; parts of each size from 1: [1, 1, 2, 5, 16, 71, 534, 7419, 196977]

  the smallest built, n = 4: 0>1 0>2 1>2 1>3 3>0 3>2

  forced at n = 11: 0 of 110 free cells; the choice left is the other 110

  the search's verdicts tried again one structure at a time, with no search: 4 of the n's above agree (the larger have too many structures)

## 8

For which n is there a tournament on n points with F(x, y) = 0 for every two distinct points x, y, where F(x, y) = (max((out(y) * both_beat(x,y)), out(y)) - max((out(y) * both_beat(x,y)), (out(y) * beat_both(x,y))))?

  reading: beats(x,y) is 1 when x beats y, out(x) counts whom x beats, both_beat(x,y) whom both beat, beat_both(x,y) who beats both, paths(x,y) the z with x beats z beats y

  built and checked: n = 4, 6, 7, 9, 11, 13, 15, 17, 19

  impossible, every structure ruled out: n = 3, 5, 8, 10

  open: n = 12, 14, 16, 18, 20, 21 (the sieve's search, its climb and the parts did not decide)

  not solved: the parts grew too many at 9 points; parts of each size from 1: [1, 1, 2, 5, 16, 71, 534, 7419, 196977]

  the smallest built, n = 4: 0>1 0>2 0>3 1>2 2>3 3>1

  forced at n = 12: 0 of 132 free cells; the choice left is the other 132

  the search's verdicts tried again one structure at a time, with no search: 4 of the n's above agree (the larger have too many structures)

## 9

For which n is there a tournament on n points with F(x, y) = 0 for every two distinct points x, y, where F(x, y) = (((4 - both_beat(x,y)) * (-1 + beat_both(x,y))) * (-1 * (-1 + both_beat(x,y))))?

  reading: beats(x,y) is 1 when x beats y, out(x) counts whom x beats, both_beat(x,y) whom both beat, beat_both(x,y) who beats both, paths(x,y) the z with x beats z beats y

  built and checked: n = 4, 6, 7, 13

  impossible, every structure ruled out: n = 3, 5, 8, 9, 10, 11

  open: n = 12, 14, 15 (the sieve's search, its climb and the parts did not decide)

  not solved: the parts grew too many at 9 points; parts of each size from 1: [1, 1, 2, 5, 16, 71, 534, 7419, 187857]

  the smallest built, n = 4: 0>1 0>2 0>3 1>2 2>3 3>1

  forced at n = 12: 0 of 132 free cells; the choice left is the other 132

  the search's verdicts tried again one structure at a time, with no search: 4 of the n's above agree (the larger have too many structures)

## 10

For which n is there a tournament on n points with F(x, y) = 0 for every two distinct points x, y, where F(x, y) = min(((1 - both_beat(x,y)) * ((beat_both(x,y) + 3) - max(out(x), 4))), paths(x,y))?

  reading: beats(x,y) is 1 when x beats y, out(x) counts whom x beats, both_beat(x,y) whom both beat, beat_both(x,y) who beats both, paths(x,y) the z with x beats z beats y

  built and checked: n = 4, 6, 7, 11

  impossible, every structure ruled out: n = 3, 5, 8, 9, 10, 12, 13, 14, 15

  open: n = 16, 17 (the sieve's search, its climb and the parts did not decide)

  not solved: the parts grew too many at 9 points; parts of each size from 1: [1, 1, 2, 5, 16, 71, 534, 7419, 196977]

  the smallest built, n = 4: 0>1 0>2 0>3 1>2 2>3 3>1

  forced at n = 16: 0 of 240 free cells; the choice left is the other 240

  the search's verdicts tried again one structure at a time, with no search: 4 of the n's above agree (the larger have too many structures)

## 11

For which n is there a graph on n points with F(x, y) = 0 for every two distinct points x, y, where F(x, y) = ((if common(x,y) != 0 then deg(y) else -1) - (|(deg(y) + (-2 + n))| - deg(y)))?

  reading: adj(x,y) is 1 when x and y are joined, common(x,y) counts the points joined to both

  built and checked: n = 6, 8, 10, 12, 14, 16, 18

  impossible, every structure ruled out: n = 3, 4, 5, 7, 9, 11, 13, 15, 17, 19

  open: n = 20, 21 (the sieve's search, its climb and the parts did not decide)

  not solved: the parts grew too many at 9 points; parts of each size from 1: [1, 2, 4, 11, 42, 220, 1556, 17774, 349148]

  the smallest built, n = 6: 0-1 0-2 0-3 0-4 1-2 1-3 1-5 2-4 2-5 3-4 3-5 4-5

  forced at n = 20: 0 of 380 free cells; the choice left is the other 380

  the search's verdicts tried again one structure at a time, with no search: 4 of the n's above agree (the larger have too many structures)

## 12

For which n is there a tournament on n points with F(x, y) = 0 for every two distinct points x, y, where F(x, y) = (min((1 - beat_both(x,y)), (3 + (1 - both_beat(x,y)))) * (1 - both_beat(x,y)))?

  reading: beats(x,y) is 1 when x beats y, out(x) counts whom x beats, both_beat(x,y) whom both beat, beat_both(x,y) who beats both, paths(x,y) the z with x beats z beats y

  built and checked: n = 4, 6, 7

  impossible, every structure ruled out: n = 3, 5, 8, 9, 10

  SOLVED for every n: no part of 11 points can be in such a structure, so none on 11 points or more; parts of each size from 1: [1, 1, 2, 5, 16, 55, 93, 58, 28, 16, 0]

  the smallest built, n = 4: 0>1 0>2 0>3 1>2 2>3 3>1

  the search's verdicts tried again one structure at a time, with no search: 4 of the n's above agree (the larger have too many structures)

## 13

For which n is there a tournament on n points with F(x, y) = 0 for every two distinct points x, y, where F(x, y) = (min((1 - beat_both(x,y)), (beat_both(x,y) + (1 - both_beat(x,y)))) * (1 - both_beat(x,y)))?

  reading: beats(x,y) is 1 when x beats y, out(x) counts whom x beats, both_beat(x,y) whom both beat, beat_both(x,y) who beats both, paths(x,y) the z with x beats z beats y

  built and checked: n = 4, 6, 7

  impossible, every structure ruled out: n = 3, 5, 8, 9, 10

  SOLVED for every n: no part of 11 points can be in such a structure, so none on 11 points or more; parts of each size from 1: [1, 1, 2, 5, 16, 55, 99, 76, 52, 30, 0]

  the smallest built, n = 4: 0>1 0>2 0>3 1>2 2>3 3>1

  the search's verdicts tried again one structure at a time, with no search: 4 of the n's above agree (the larger have too many structures)

## 14

For which n is there a tournament on n points with F(x, y) = 0 for every two distinct points x, y, where F(x, y) = (min((1 - beat_both(x,y)), (out(y) + (beats(x,y) - out(x)))) * (1 - both_beat(x,y)))?

  reading: beats(x,y) is 1 when x beats y, out(x) counts whom x beats, both_beat(x,y) whom both beat, beat_both(x,y) who beats both, paths(x,y) the z with x beats z beats y

  built and checked: n = 4, 6, 7

  impossible, every structure ruled out: n = 3, 5, 8, 9, 10

  SOLVED for every n: no part of 11 points can be in such a structure, so none on 11 points or more; parts of each size from 1: [1, 1, 2, 5, 16, 55, 99, 76, 52, 30, 0]

  the smallest built, n = 4: 0>1 0>2 0>3 1>2 2>3 3>1

  the search's verdicts tried again one structure at a time, with no search: 4 of the n's above agree (the larger have too many structures)

## 15

For which n is there a tournament on n points with F(x, y) = 0 for every two distinct points x, y, where F(x, y) = min(min(((1 - both_beat(x,y)) * (1 - beat_both(x,y))), 1), |(paths(x,y) + beats(x,y))|)?

  reading: beats(x,y) is 1 when x beats y, out(x) counts whom x beats, both_beat(x,y) whom both beat, beat_both(x,y) who beats both, paths(x,y) the z with x beats z beats y

  built and checked: n = 4, 6, 7

  impossible, every structure ruled out: n = 3, 5, 8, 9, 10

  SOLVED for every n: no part of 11 points can be in such a structure, so none on 11 points or more; parts of each size from 1: [1, 1, 2, 5, 16, 55, 99, 76, 52, 30, 0]

  the smallest built, n = 4: 0>1 0>2 0>3 1>2 2>3 3>1

  the search's verdicts tried again one structure at a time, with no search: 4 of the n's above agree (the larger have too many structures)

## 16

For which n is there a tournament on n points with F(x, y) = 0 for every two distinct points x, y, where F(x, y) = min(min(((1 - both_beat(x,y)) * (1 - beat_both(x,y))), min(beats(x,y), 3)), (4 - both_beat(x,y)))?

  reading: beats(x,y) is 1 when x beats y, out(x) counts whom x beats, both_beat(x,y) whom both beat, beat_both(x,y) who beats both, paths(x,y) the z with x beats z beats y

  built and checked: n = 4, 6, 7

  impossible, every structure ruled out: n = 3, 5, 8, 9, 10, 11, 12

  open: n = 13, 14 (the sieve's search, its climb and the parts did not decide)

  not solved: the parts grew too many at 9 points; parts of each size from 1: [1, 1, 2, 5, 16, 71, 516, 6451, 131724]

  the smallest built, n = 4: 0>1 0>2 0>3 1>2 2>3 3>1

  forced at n = 13: 0 of 156 free cells; the choice left is the other 156

  the search's verdicts tried again one structure at a time, with no search: 4 of the n's above agree (the larger have too many structures)

## 17

For which n is there a tournament on n points with F(x, y) = 0 for every two distinct points x, y, where F(x, y) = min(((1 - both_beat(x,y)) * ((beat_both(x,y) + 3) - max(both_beat(x,y), 4))), min(beats(x,y), out(y)))?

  reading: beats(x,y) is 1 when x beats y, out(x) counts whom x beats, both_beat(x,y) whom both beat, beat_both(x,y) who beats both, paths(x,y) the z with x beats z beats y

  built and checked: n = 4, 6, 7, 8, 9

  impossible, every structure ruled out: n = 3, 5, 10, 11, 12, 13

  open: n = 14, 15 (the sieve's search, its climb and the parts did not decide)

  not solved: the parts grew too many at 9 points; parts of each size from 1: [1, 1, 2, 5, 16, 71, 534, 7419, 196977]

  the smallest built, n = 4: 0>1 0>2 0>3 1>2 2>3 3>1

  forced at n = 14: 0 of 182 free cells; the choice left is the other 182

  the search's verdicts tried again one structure at a time, with no search: 4 of the n's above agree (the larger have too many structures)

## 18

For which n is there a tournament on n points with F(x, y) = 0 for every two distinct points x, y, where F(x, y) = (((|beat_both(x,y)| - (-1 + both_beat(x,y))) * (-1 + beat_both(x,y))) * ((-1 + both_beat(x,y)) * -1))?

  reading: beats(x,y) is 1 when x beats y, out(x) counts whom x beats, both_beat(x,y) whom both beat, beat_both(x,y) who beats both, paths(x,y) the z with x beats z beats y

  built and checked: n = 4, 6, 7, 8

  impossible, every structure ruled out: n = 3, 5, 9, 10, 11

  open: n = 12, 13 (the sieve's search, its climb and the parts did not decide)

  not solved: the parts grew too many at 9 points; parts of each size from 1: [1, 1, 2, 5, 16, 71, 534, 7419, 196977]

  the smallest built, n = 4: 0>1 0>2 0>3 1>2 2>3 3>1

  forced at n = 12: 0 of 132 free cells; the choice left is the other 132

  the search's verdicts tried again one structure at a time, with no search: 4 of the n's above agree (the larger have too many structures)

## 19

For which n is there a tournament on n points with F(x, y) = 0 for every two distinct points x, y, where F(x, y) = ((n * (-1 + beat_both(x,y))) * (((3 * both_beat(x,y)) - (paths(x,y) + both_beat(x,y))) * (-1 + both_beat(x,y))))?

  reading: beats(x,y) is 1 when x beats y, out(x) counts whom x beats, both_beat(x,y) whom both beat, beat_both(x,y) who beats both, paths(x,y) the z with x beats z beats y

  built and checked: n = 4, 6, 7

  impossible, every structure ruled out: n = 3, 5, 8, 9, 10, 11, 12, 13, 14

  open: n = 15, 16 (the sieve's search, its climb and the parts did not decide)

  not solved: the parts grew too many at 9 points; parts of each size from 1: [1, 1, 2, 5, 16, 71, 534, 7419, 196977]

  the smallest built, n = 4: 0>1 0>2 0>3 1>2 2>3 3>1

  forced at n = 15: 0 of 210 free cells; the choice left is the other 210

  the search's verdicts tried again one structure at a time, with no search: 4 of the n's above agree (the larger have too many structures)

## 20

For which n is there a tournament on n points with F(x, y) = 0 for every two distinct points x, y, where F(x, y) = (((if ((beat_both(x,y) - 3) + 2) != 0 then ((beats(x,y) * -1) + paths(x,y)) else 0) * 3) * (beat_both(x,y) - out(y)))?

  reading: beats(x,y) is 1 when x beats y, out(x) counts whom x beats, both_beat(x,y) whom both beat, beat_both(x,y) who beats both, paths(x,y) the z with x beats z beats y

  built and checked: n = 4, 5, 7

  impossible, every structure ruled out: n = 3, 6, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 20

  open: n = 19, 21, 22 (the sieve's search, its climb and the parts did not decide)

  not solved: the parts grew too many at 9 points; parts of each size from 1: [1, 1, 2, 5, 16, 71, 534, 7419, 196977]

  the smallest built, n = 4: 0>1 0>2 0>3 1>2 2>3 3>1

  forced at n = 19: 0 of 342 free cells; the choice left is the other 342

  the search's verdicts tried again one structure at a time, with no search: 4 of the n's above agree (the larger have too many structures)

## 21

For which n is there a graph on n points with F(x, y) = 0 for every two distinct points x, y, where F(x, y) = ((if common(x,y) != 0 then deg(y) else 0) - (|(deg(y) + (-2 + n))| - min((deg(y) + (deg(y) - deg(x))), deg(y))))?

  reading: adj(x,y) is 1 when x and y are joined, common(x,y) counts the points joined to both

  built and checked: n = 6, 8, 10, 12, 14, 16, 18, 20

  impossible, every structure ruled out: n = 3, 4, 5, 7, 9, 11, 13, 15, 17, 19

  open: n = 21, 22 (the sieve's search, its climb and the parts did not decide)

  not solved: the parts grew too many at 9 points; parts of each size from 1: [1, 2, 4, 11, 42, 220, 1556, 17774, 349148]

  the smallest built, n = 6: 0-1 0-2 0-3 0-4 1-2 1-3 1-5 2-4 2-5 3-4 3-5 4-5

  forced at n = 21: 0 of 420 free cells; the choice left is the other 420

  the search's verdicts tried again one structure at a time, with no search: 4 of the n's above agree (the larger have too many structures)
