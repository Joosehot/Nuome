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
  MATH algebra L1                135         7          7/7
  MATH algebra L2                201         9          8/9
  MATH algebra L3                261         5          5/5
  MATH algebra L4                283         5          5/5
  MATH algebra L5                307         1          1/1
  MATH number_theory L1           30         0          0/0
  MATH number_theory L2           92         0          0/0
  MATH number_theory L3          122         0          0/0
  MATH number_theory L4          142         0          0/0
  MATH number_theory L5          154         0          0/0
  all                           1727        27        26/27
  WRONG: 'Evaluate $(-125)^{4/3}$.' answered '(-125)^(4/3)', truth '625'

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
  MATH algebra L1                135       135        17/21
  MATH algebra L2                201       201        12/24
  MATH algebra L3                261       261        10/24
  MATH algebra L4                283       283         7/25
  MATH algebra L5                307       307         2/12
  MATH number_theory L1           30        30         3/10
  MATH number_theory L2           92        92         4/26
  MATH number_theory L3          122       122         1/25
  MATH number_theory L4          142       142         1/26
  MATH number_theory L5          154       154         0/26
  all                           1727      1727       57/219
