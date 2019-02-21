module Test.Import where
import Plus as Foo
import Prelude
import Foo.Bar (wee)
import Foo.Bar (wee foo)
import Foo.Bar ()
import Foo.Bar (ff) as Boo
import Boo
main = Foo.plus 5

module Plus where
plus x = x + 5
wee = 5

module Foo.Bar where
wee = 5
ff = 4
foo = 6
qq = 4


module Boo where
boo = 5