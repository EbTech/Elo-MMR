#!/bin/bash

g++ -std=c++11 -o CFreader CFreader.cpp

for round in $(seq 599 606)
do
    rm Standings/$round.*
    #hash2=0
    for page in $(seq 1 50)
    do
        wget -O Standings/temp1.html http://codeforces.com/contest/$round/standings/page/$page
        #hash1=$hash2
        #hash2=$(md5sum Standings/temp$page.html)
        #if [ "$hash1" = "$hash2" ]
        #then
        #    break
        #fi
        cat Standings/$round.html Standings/temp1.html > Standings/temp2.html
        cat Standings/temp2.html > Standings/$round.html
    done
    
    ./CFreader Standings/$round.html > Standings/$round.txt
done
