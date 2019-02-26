module Test.Lambda where

import Std.Util

lambdaCreate = do
                a <- 1
                \x, y -> x + y + a

lambdaCreate2 a = do
                \x, y -> x + y + a


run = do 
        assert ((do lambdaCreate) 2 3 == 6)
        assert ((lambdaCreate2 5) 2 3 == 10)