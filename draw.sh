#!/bin/bash

SCRIPTDIR=`dirname $0`

cd $SCRIPTDIR

cd dots
dot *.dot -Tpng -O
