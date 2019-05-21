// author: Aram Ebtekar
#include <iostream>
#include <fstream>
#include <string>
#include <vector>
#include <map>
#include <cassert>
#include <cmath>
#include <sstream>
#include <unordered_set>
using namespace std;

struct Outcome
{
    int lo, hi;
    string name;
};

bool readBetween(string leftBound, int offset, string rightBound, string text, string& result)
{
    int start = text.find(leftBound);
    if (start == string::npos)
        return false;
    start += offset;
    int finish = text.find(rightBound, start);
    if (finish == string::npos)
        return false;
    result = text.substr(start, finish-start);
    return true;
}

int main(int argc, char* argv[])
{
    if (argc != 2)
    {
        cerr << "usage: " << argv[0] << " infile > outfile" << endl;
        return 1;
    }
    ifstream standingsFile;
    standingsFile.open(argv[1]);
    vector<Outcome> outcomes;
    unordered_set<string> seenNames;
    string line;
    while (getline(standingsFile, line))
    if (line.substr(0, 18) == "<tr participantId=")
    {
        Outcome o;
        getline(standingsFile, line);
        string rankString;
        if (readBetween(">", 1, "<", line, rankString))
            o.lo = atoi(rankString.c_str());
        else
            standingsFile >> o.lo;
        o.hi = 1 + outcomes.size();
        do
            getline(standingsFile, line);
        while (!readBetween("/profile/", 9, "\"", line, o.name));
        if (seenNames.find(o.name) != seenNames.end())
            break;
        seenNames.insert(o.name);
        outcomes.push_back(o);
    }
    standingsFile.close();
    for (int i = outcomes.size()-2; i >= 0; --i)
    {
        assert(outcomes[i].lo <= outcomes[i+1].lo);
        if (outcomes[i].lo == outcomes[i+1].lo)
            outcomes[i].hi = outcomes[i+1].hi;
    }
    cout << outcomes.size() << endl;
    for (Outcome& o : outcomes)
    {
        cout << o.name << ' ' << o.lo << ' ' << o.hi << endl;
    }
}
