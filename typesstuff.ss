module Main where

testfunc a :: a -> a -> ((a,Int),) -> Int
testfunc x y z = extern

closure1 :: Int -> Int -> Int
closure1 x y = extern

main2 a b :: a -> b
main2 x =  testfunc "a" "a" (("a",5),)

main = closure1 (5 +5)