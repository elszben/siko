module Test.If where

iffer a b = if True 
          then if a < b then 5 else 5
          else 5

run = iffer 3 4