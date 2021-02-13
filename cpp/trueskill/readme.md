another trueskill implementation; c++ with wrappers for python

This is a port of [dougz/trueskill](https://github.com/dougz/trueskill)
from python to c++;

running
=======

You have two options:

1. run ``make`` in the root directory and launch the program
   via ``bin/runner``
2. run ``python setup.py build_ext -i`` to create the ``.so``
   file, and run the python test sample via ``python ptest.py``

benchmark
=========

I timed each version via timeit while running the ps command on a
1s interval, and I included the last entry for each program. I used
my ``ptest.py`` file, and a modified version of ``sample.py`` in
the dougz implementation.

---------------
dougz pure python implementation:

    USER       PID %CPU %MEM    VSZ   RSS TTY      STAT START   TIME COMMAND
    jbueski+ 15974  100  2.3 927368 433584 pts/8   Rl+  16:36   1:31 python sample.py
    
    10,000 iterations in 91.2056720257s
     Alice: mu=33.208  sigma=6.348
       Bob: mu=27.401  sigma=5.787
     Chris: mu=22.599  sigma=5.787
    Darren: mu=16.793  sigma=6.348

~110/s

---------------
this version:

    USER       PID %CPU %MEM    VSZ   RSS TTY      STAT START   TIME COMMAND
    jbueski+ 19989  100  0.7 159044 131628 pts/17  R+   16:41   0:39 python ptest.py
    
    1,000,000 iterations in 39.7102029324s
    [(33.20778932686388, 6.347937213962031, 1),
     (27.4014978831654, 5.787057811781698, 2),
     (22.598576351284233, 5.787115941230072, 3),
     (16.793374093106948, 6.348053082283166, 4)]

~25,182/s

---------------
Above we can see a roughly 200x speedup from the c++ version.

note
====
I have not written any c++ in over 5 years. This port of a python
program was an attempt to refresh my memory on writing c++. It is
in no way optimized, and for that matter I can guarantee it is
not the best way to implement it. I did however run it through
valgrind and removed all memory leaks, so that's something.
