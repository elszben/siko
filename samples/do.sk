module Test.Do where

import Std.Util

getstuff = 1

run = do
    a <- if True
      then 3
      else 4
    do b <- do getstuff
       c <- 2
       d <- a + b + c
       assert (d == 6)

