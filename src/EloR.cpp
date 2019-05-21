// author: Aram Ebtekar
#include <iostream>
#include <iomanip>
#include <fstream>
#include <cassert>
#include <cmath>
#include <sstream>
#include <string>
#include <vector>
#include <map>
#include <unordered_map>
#include <algorithm>
using namespace std;

const array<int,910-17> contests = {1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 18, 19, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 40, 41, 42, 43, 46, 47, 48, 49, 51, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 65, 66, 67, 68, 69, 70, 71, 73, 75, 74, 80, 77, 78, 79, 81, 82, 84, 83, 85, 86, 88, 87, 90, 89, 92, 91, 94, 93, 96, 95, 97, 102, 101, 104, 103, 105, 106, 108, 107, 110, 109, 112, 111, 114, 113, 116, 115, 117, 118, 120, 122, 121, 124, 123, 127, 126, 129, 128, 131, 133, 132, 136, 135, 137, 139, 138, 140, 141, 143, 142, 144, 146, 145, 148, 149, 151, 150, 152, 155, 154, 157, 156, 160, 161, 165, 166, 169, 163, 168, 167, 173, 174, 164, 175, 181, 176, 177, 180, 183, 178, 186, 189, 187, 192, 191, 194, 193, 195, 197, 196, 199, 198, 200, 202, 201, 203, 205, 204, 211, 208, 214, 213, 215, 216, 218, 217, 219, 221, 220, 222, 224, 223, 225, 227, 226, 228, 230, 229, 231, 233, 232, 240, 234, 236, 235, 237, 241, 239, 238, 242, 244, 243, 245, 246, 249, 248, 247, 250, 252, 251, 253, 254, 255, 256, 259, 258, 260, 257, 262, 261, 263, 265, 264, 266, 268, 270, 269, 271, 273, 272, 275, 274, 276, 278, 277, 279, 281, 280, 282, 284, 283, 285, 287, 286, 289, 288, 294, 296, 295, 292, 298, 297, 299, 293, 300, 304, 303, 308, 309, 305, 313, 315, 314, 316, 318, 317, 320, 319, 322, 321, 327, 325, 324, 331, 330, 329, 332, 334, 333, 335, 326, 336, 338, 337, 339, 341, 340, 342, 344, 343, 347, 346, 349, 348, 350, 352, 351, 353, 355, 354, 357, 356, 358, 359, 361, 360, 363, 362, 365, 364, 366, 368, 367, 369, 370, 371, 373, 372, 376, 375, 378, 377, 379, 381, 380, 382, 384, 383, 385, 387, 389, 388, 400, 401, 403, 402, 404, 406, 405, 408, 407, 415, 414, 416, 418, 417, 412, 413, 421, 420, 419, 424, 426, 425, 427, 430, 429, 432, 431, 434, 433, 435, 438, 437, 439, 441, 436, 443, 442, 445, 444, 447, 446, 448, 450, 449, 451, 452, 454, 453, 456, 455, 458, 457, 459, 460, 462, 461, 463, 465, 464, 466, 467, 469, 468, 471, 472, 475, 474, 477, 476, 478, 480, 479, 483, 482, 486, 489, 488, 487, 490, 492, 493, 495, 494, 497, 496, 499, 498, 500, 504, 501, 506, 505, 507, 508, 509, 510, 512, 513, 514, 516, 515, 518, 519, 521, 520, 528, 527, /*524,*/ 529, 525, 526, 534, 536, 535, 533, /*532,*/ 538, 540, 542, /*541,*/ 544, 543, 545, 546, 548, 547, 550, 549, 551, 552, 554, 553, 556, 555, 557, 558, 560, 559, /*562, 566,*/ 567, 569, 568, 570, 572, 571, 574, 573, 577, 576, 579, 578, 580, 581, 583, 582, 584, 586, 585, 588, 587, 591, 590, 592, 593, 596, 599, 602, 601, 604, 603, 606, 605, 608, 607, 610, 611, 615, 614, 613, 617, 618, 621, 624, 623, 625, 626, 629, 633, 635, 634, 627, 631, 651, 650, 655, 645, 653, 648, 649, 658, 657, /*639,*/ 659, 662, 669, 668, /*641,*/ 667, 666, 670, 674, 673, /*643,*/ 672, 671, 675, 676, 677, 680, 679, 681, 682, 686, 685, 688, 687, /*695,*/ 689, 697, 696, 699, 698, 701, 700, 705, 704, 706, 707, 709, 708, 711, 714, 713, 716, 715, 719, 721, 722, 723, 724, 727, 731, 732, 725, 733, 734, 738, 737, 729, 740, 739, 736, 735, 742, 741, 743, 745, 744, 746, 747, 749, 752, 748, 750, 754, 757, 755, 758, 760, 759, 756, 764, 763, 766, 765, 767, 768, 776, 777, 779, 778, 780, 782, 781, 785, 791, 790, /*771,*/ 787, 786, 789, 788, 796, 801, 800, /*772,*/ 798, 793, 805, 804, 807, 806, /*773,*/ 799, 794, 810, 809, 811, 812, 814, 816, 815, 822, /*823,*/ 828, 827, 831, 830, 832, 834, 833, 835, 839, 841, 840, 844, 843, 842, 849, 848, 851, 850, 854, 853, 859, 862, 855, 864, 867, 866, 865, 868, 869, 872, 871, 870, 876, 875, 877, 879, 878, 892, 891, 894, 893, 895, 897, 896, 900, 903, 898, 899, 902, 901, 907, 906, 904, 909, 911, 908, 912, 913, 915, 914, 918, 917, 919, 920, 922, 934, 933, 932, 938, 939, 935, 940, 937, 936, 944, 931, 930, 946, 950, 949, 948, 947, /*923,*/ 954, 955, 957, 956, /*924,*/ 959, 961, 960, 962, 964, 963, 965, 967, 966, /*925,*/ 976, 975, 977, 980, 978, 979, 984, 983, 982, 985, 981, 987, 986, 988, 990, 989, 994, 993, 992, 999, 991, 996, 995, 1000, 998, 997, 1003, 1004, 1005, 1008, 1007, 1009, 1006, 1011, 1010, 1013, 1012, 1015, 1016, 1020, 1019, /*951,*/ 1023, 1027, 1025, 1029, 1028, 1037, 1040, 1039, 1038, 1036, 1041, 1042, 1051, 1047, 1034, 1058, 1053, 1030, 1060, 1059, 1033, 1065, 1066, 1064, 1063, 1054, 1072, 1071, 1031, 1068, 1067, 1073, 1043, 1075, 1074, 1044, 1055, 1076, 1062, 1077, 1061, 1080, 1056, 1082, 1088, 1084, 1083, 1093, 1081, 1092, 1087, 1086, 1085, 1095, 1096, 1091, 1097, 1099, 1098, 1102, 1101, 1100, 1105, 1104, 1103, 1108, 1107, 1111, 1110, 1114, 1113, 1109, 1117, 1118, 1131, 1130, 1129, 1112, 1132, 1133, 1138, 1137, 1136, 1141, 1139, 1140, 1143, 1142, 1144, 1119, 1153, 1154, 1151, 1146, 1155, 1152, 1157, 1150, 1149, 1162, 1161, 1147, 1163, 1159, 1158, 1165, 1167, 1166};
const int NUM_TITLES = 10;
const array<int,NUM_TITLES> bounds = {-999,1000,1250,1500,1750,2000,2150,2300,2500,2800};
const array<string,NUM_TITLES> titles = {"Ne","Pu","Sp","Ex","CM","Ma","IM","GM","IG","LG"};
const double sig_limit = 100; // limiting uncertainty for a player who competed a lot
const double sig_perf = 250; // variation in individual performances
const double sig_newbie = 350; // uncertainty for a new player
const double sig_noise = sqrt( 1.0 / (1.0/sig_limit/sig_limit - 1.0/sig_perf/sig_perf)
                               - sig_limit*sig_limit );

struct Rating {
    double mu; // mean of skill belief
    double sig; // uncertainty of skill belief
    Rating(double m, double s) : mu(m), sig(s) {} // create player
};

ostream& operator<<(ostream& os, const Rating& r) {
    os << int(r.mu) << "+/-" << int(r.sig);
    return os;
}

// returns something near the mean if the ratings are consistent; near the median if they're far apart
// offC and offM are constant and slope offsets, respectively
double robustMean(const vector<Rating>& ratings, double offC = 0, double offM = 0) {
    double lo = -1000, hi = 5000;
    while (hi - lo > 1e-9) {
        double mid = (lo + hi) / 2;
        double sum = offC + offM * mid;
        for (const Rating& r : ratings)
            sum += tanh((mid-r.mu)/r.sig) / r.sig;
        if (sum > 0)
            hi = mid;
        else
            lo = mid;
    }
    return (lo + hi) / 2;
}

// ratings is a list of the participants, ordered from first to last place
// returns: performance of the player in ratings[id] who tied against ratings[lo..hi]
double performance(vector<Rating> ratings, int id, int lo, int hi) {
    int N = ratings.size();
    assert(0 <= lo && lo <= id && id <= hi && hi <= N-1);
    double offset = 0;
    for (int i = 0; i < lo; ++i)
        offset += 1.0 / ratings[i].sig;
    for (int i = hi+1; i < N; ++i)
        offset -= 1.0 / ratings[i].sig;
    ratings.push_back(ratings[id]);
    return robustMean(ratings, offset);
}

struct Player {
    vector<Rating> perfs;
    Rating strongPrior; // future optimization: if perfs gets too long, merge results into strongPrior
    Rating posterior;
    int prevRating, maxRating, prevContest;
    
    // apply noise to one variable for which we have many estimates
    void add_noise_uniform() {
        double decay = sqrt(1.0 + sig_noise*sig_noise/posterior.sig/posterior.sig);
        strongPrior.sig *= decay;
        for (Rating& r : perfs)
            r.sig *= decay;
    }
    
    void updatePosterior() {
        double sigInvSq = 1.0 / strongPrior.sig / strongPrior.sig;
        double mu = robustMean(perfs, -strongPrior.mu*sigInvSq, sigInvSq);
        for (const Rating& r : perfs)
            sigInvSq += 1.0 / r.sig / r.sig;
        posterior = Rating(mu, 1.0 / sqrt(sigInvSq));
    }
    
    int conservativeRating() const // displayed rating {
        return int(posterior.mu - 2*(posterior.sig - sig_limit) + 0.5);
    }
    Player() : maxRating(0), strongPrior(1500,sig_newbie), posterior(1500,sig_newbie) { }
};

void simulateCodeforcesHistory()
{
    map<string, Player> players;

    // 2011 ends at round 139, 2013 ends at round 379, 2015 ends at round 612
    for (int roundNum : contests) {
        // read the standings
        stringstream ssFileName;
        ssFileName << "../standings/" << roundNum << ".txt";
        ifstream standingsFile(ssFileName.str());
        int N; string title;
        standingsFile >> N;
        getline(standingsFile, title);
        cerr << "Processing Codeforces Round " << roundNum;
        cerr << " with " << N << " rated contestants..." << endl;

        vector<string> names(N);
        vector<int> lo(N), hi(N);
        vector<Rating> compRatings;
        compRatings.reserve(N);
        for (int i = 0; i < N; ++i) {
            standingsFile >> names[i] >> lo[i] >> hi[i];
            --lo[i]; --hi[i];

            Player& player = players[names[i]];
            Rating& r = player.posterior;
            double compVar = r.sig*r.sig + sig_noise*sig_noise + sig_perf*sig_perf;
            compRatings.emplace_back(r.mu, sqrt(compVar));
        }
        standingsFile.close();

        // begin rating updates
        for (int i = 0; i < N; ++i) {
            Player& player = players[names[i]];
            player.add_noise_uniform();

            double perf = performance(compRatings, i, lo[i], hi[i]);
            player.perfs.emplace_back(perf, sig_perf);

            player.prevRating = player.conservativeRating();
            player.updatePosterior();
            player.maxRating = max(player.maxRating, player.conservativeRating());
            player.prevContest = roundNum;
        }
        // end rating updates
    }
    // output the result
    double sumRatings = 0;
    vector<tuple<int,string,int,int,int,int> > conservativeRatings;
    for (pair<const string,Player>& entry : players) {
        Player& player = entry.second;
        Rating& r = player.posterior;
        conservativeRatings.push_back(make_tuple(player.conservativeRating(), entry.first,
            player.maxRating, player.prevRating, player.perfs.back().mu, player.prevContest));
        sumRatings += r.mu;
    }
    cout << "Mean rating.mu = " << (sumRatings/players.size()) << endl;
    sort(conservativeRatings.begin(), conservativeRatings.end());
    reverse(conservativeRatings.begin(), conservativeRatings.end());

    array<int,NUM_TITLES> titleCount = {};
    int titleID = NUM_TITLES - 1;
    for (tuple<int,string,int,int,int,int>& entry: conservativeRatings) {
        while (get<0>(entry) < bounds[titleID]) {
            --titleID;
        }
        ++titleCount[titleID];
    }
    for (titleID = NUM_TITLES - 1; titleID >= 0; --titleID) {
        cout << bounds[titleID] << " " << titles[titleID] << " x " << titleCount[titleID] << endl;
    }
    for (tuple<int,string,int,int,int,int>& entry: conservativeRatings) {
        int delta = get<0>(entry) - get<3>(entry);
        cout << setw(4) << get<0>(entry) << "(";
        cout << setw(4) << get<2>(entry) << ")";
        cout << setw(24) << get<1>(entry);
        cout << " | contest/" << setw(4) << get<5>(entry);
        cout << ": perf =" << setw(5) << get<4>(entry);
        cout << ", delta =" << setw(4) << delta << endl;
    }
}

void testRobustness() {
    Player player;
    for (int i = 0; i < 1000; ++i) {
        player.add_noise_uniform();
        player.perfs.emplace_back(1000, sig_perf);
        player.updatePosterior();
    }
    double mean = 1000;
    double w = (sig_limit*sig_limit + sig_noise*sig_noise) / (sig_limit*sig_limit + sig_noise*sig_noise + sig_perf*sig_perf);
    for (int i = 0; i < 31; ++i) {
        cout << int(mean+0.5) << ",";
        //cout << i << ' ' << player.conservativeRating() << endl;
        mean += w * (3000 - mean);
        player.add_noise_uniform();
        player.perfs.emplace_back(3000, sig_perf);
        player.updatePosterior();
    }
}

int main()
{
    //testRobustness();
    simulateCodeforcesHistory(); // takes about 40 mins on my PC
}
