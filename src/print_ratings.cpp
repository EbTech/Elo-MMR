// Copy-paste a spreadsheet column of CF handles as input to this program, then
// paste this program's output into the spreadsheet's ratings column.
#include <iostream>
#include <fstream>
#include <string>
#include <map>

using namespace std;

int main() {
    ifstream standingsFile("../CFratings.txt");
    bool header = true;
    map<string, string> ratings;
    string line;
    
    while (getline(standingsFile, line)) {
        if (header) {
            header &= line[0] != '-';
        } else {
            string rating = line.substr(0, 4);
            string handle = line.substr(10, 24);
            ratings[handle] = rating;
        }
    }
    
    string handle;
    while (cin >> handle) {
        while (handle.length() < 24) {
            handle = " " + handle;
        }
        cout << ratings[handle] << endl;
    }
}
