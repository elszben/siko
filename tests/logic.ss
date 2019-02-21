module Test.Logic where

import Std.Util

run = do
        assert (True && True)
        assert (True || False)
        assert True
        assert !False
        assert (True == True)
        assert (False == False)
        assert (True == !False)
        assert (True != False)
