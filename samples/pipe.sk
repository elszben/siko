module Test.Pipe where

import Std.Util

plus_one x = x + 1

pipetest x = x |> plus_one

run = assert (pipetest 5 == 6)