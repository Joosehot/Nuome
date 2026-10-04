# Golden Answer benchmark

answered = the kind gave an answer; right = agrees with the truth, of the answers that can be judged (an abstract "no evidence either way" is not judged).

## oracle (900 questions, fixed in bench/golden_answer/oracle.jsonl)

logical:
  category                         n  answered        right
  arithmetic                     100       100      100/100
  collatz                        100       100      100/100
  factoring                      100       100      100/100
  goldbach                       100       100      100/100
  mersenne                       100        55        55/55
  primality                      100        71        71/71
  prime counting                 100       100      100/100
  properties                     100       100      100/100
  sums of squares                100       100      100/100
  all                            900       826      826/826

theoretical:
  category                         n  answered        right
  arithmetic                     100         0          0/0
  collatz                        100       100      100/100
  factoring                      100       100      100/100
  goldbach                       100       100      100/100
  mersenne                       100        30        30/30
  primality                      100        81        81/81
  prime counting                 100       100      100/100
  properties                     100         0          0/0
  sums of squares                100       100      100/100
  all                            900       611      611/611

abstract:
  category                         n  answered        right
  arithmetic                     100       100      100/100
  collatz                        100       100      100/100
  factoring                      100       100      100/100
  goldbach                       100       100      100/100
  mersenne                       100       100      100/100
  primality                      100       100      100/100
  prime counting                 100       100      100/100
  properties                     100       100      100/100
  sums of squares                100       100      100/100
  all                            900       900      900/900

## math (1727 questions, fixed in bench/golden_answer/math.jsonl)

logical:
  category                         n  answered        right
  MATH algebra L1                135        16        16/16
  MATH algebra L2                201        21        21/21
  MATH algebra L3                261        19        19/19
  MATH algebra L4                283        12        12/12
  MATH algebra L5                307         2          2/2
  MATH number_theory L1           30         4          4/4
  MATH number_theory L2           92         9          9/9
  MATH number_theory L3          122        13        13/13
  MATH number_theory L4          142        11        11/11
  MATH number_theory L5          154         4          4/4
  all                           1727       111      111/111

theoretical:
  category                         n  answered        right
  MATH algebra L1                135         0          0/0
  MATH algebra L2                201         0          0/0
  MATH algebra L3                261         0          0/0
  MATH algebra L4                283         0          0/0
  MATH algebra L5                307         0          0/0
  MATH number_theory L1           30         0          0/0
  MATH number_theory L2           92         0          0/0
  MATH number_theory L3          122         0          0/0
  MATH number_theory L4          142         0          0/0
  MATH number_theory L5          154         0          0/0
  all                           1727         0          0/0

abstract:
  category                         n  answered        right
  MATH algebra L1                135       135        20/24
  MATH algebra L2                201       201        21/32
  MATH algebra L3                261       261        17/31
  MATH algebra L4                283       283        12/30
  MATH algebra L5                307       307         2/12
  MATH number_theory L1           30        30         4/11
  MATH number_theory L2           92        92        11/31
  MATH number_theory L3          122       122        13/33
  MATH number_theory L4          142       142        12/37
  MATH number_theory L5          154       154         4/30
  all                           1727      1727      116/271
