mod normal;
mod nodes;

use super::contest_config::Contest;
use super::compute_ratings::{RatingSystem, Player, Rating};

use nodes::{ProdNode, LeqNode, GreaterNode, SumNode, TreeNode, ValueNode, FuncNode};
use normal::Gaussian;

use rayon::prelude::*;
use std::cell::{RefCell, RefMut};
use std::cmp::max;
use std::collections::{HashMap, VecDeque};
use std::collections::hash_map::Entry;

use std::rc::{Rc, Weak};
use std::f64::INFINITY;

type TSPlayerRating = Gaussian;
type TSMessage = nodes::Message;
type TSPlayer = String;
type TSTeam = Vec<TSPlayer>;
type TSContestPlace = Vec<TSTeam>;
type TSContest = Vec<TSContestPlace>;
type TSRating = HashMap<TSPlayer, TSPlayerRating>;

// TrueSkillStPB rating system
pub struct TrueSkillSPBSystem {
    // epsilon used for ties
    eps: f64,
    // default player rating
    mu: f64,
    // default player sigma
    sigma: f64,
    // performance sigma
    beta: f64,
    // epsilon used for convergence loop
    convergence_eps: f64,
    // defines sigma growth per second
    sigma_growth: f64,
    // history of ratings for all players
    rating: TSRating,
}

impl Default for TrueSkillSPBSystem {
    fn default() -> Self {
        Self {
            eps: 0.70,
            mu: 1500.,
            sigma: 1500. / 3., // mu/3
            beta: 1500. / 6., // sigma/2
            convergence_eps: 2e-4,
            sigma_growth: 5.,
            rating: TSRating::new(),
        }
    }
}

// fn update_rating(old: &TSRating, new: &mut TSRatingHistory, contest: &TSContest, when: usize) {
//     for place in &contest[..] {
//         for team in &place[..] {
//             for player in &team[..] {
//                 new.entry(player.clone()).or_insert(Vec::new()).push((old.get(player).unwrap().clone(), when));
//             }
//         }
//     }
// }


fn gen_team_message<T, K: Clone>(places: &Vec<Vec<T>>, default: &K) -> Vec<Vec<K>> {
    let mut ret: Vec<Vec<K>> = Vec::with_capacity(places.len());

    for place in places {
        ret.push(vec![default.clone(); place.len()]);
    }

    ret
}


fn gen_player_message<T, K: Clone>(places: &Vec<Vec<Vec<T>>>, default: &K) -> Vec<Vec<Vec<K>>> {
    let mut ret = Vec::with_capacity(places.len());

    for place in places {
        ret.push(Vec::with_capacity(place.len()));

        for team in place {
            ret.last_mut().unwrap().push(vec![default.clone(); team.len()]);
        }
    }

    ret
}


fn infer1(who: &mut Vec<impl TreeNode>) {
    for item in who {
        item.infer();
    }
}


fn infer2(who: &mut Vec<Vec<impl TreeNode>>) {
    for item in who {
        infer1(item);
    }
}


fn infer3(who: &mut Vec<Vec<Vec<impl TreeNode>>>) {
    for item in who {
        infer2(item);
    }
}


fn infer_ld(ld: &mut Vec<impl TreeNode>, l: &mut Vec<impl TreeNode>) {
    for i in 0..ld.len() {
        l[i].infer();
        ld[i].infer();
    }
    l.last_mut().unwrap().infer();
    for j in 0..ld.len() {
        let i = ld.len() - 1 - j;
        ld[i].infer();
        l[i].infer();
    }
}


fn check_convergence(a: &Vec<Rc<RefCell<(TSMessage, TSMessage)>>>,
                     b: &Vec<(TSMessage, TSMessage)>) -> f64 {
    if a.len() != b.len() {
        return INFINITY;
    }

    let mut ret = 0.;

    for i in 0..a.len() {
        ret = f64::max(ret,
                       f64::max(
                           f64::max(f64::abs(RefCell::borrow(&a[i]).0.mu - b[i].0.mu),
                                    f64::abs(RefCell::borrow(&a[i]).0.sigma - b[i].0.sigma)),
                           f64::max(f64::abs(RefCell::borrow(&a[i]).1.mu - b[i].1.mu),
                                    f64::abs(RefCell::borrow(&a[i]).1.sigma - b[i].1.sigma)),
                       ));
    }

    ret
}

impl TrueSkillSPBSystem {
    fn inference(&mut self, contest: &TSContest) {
        if contest.is_empty() {
            return;
        }

        // could be optimized, written that way for simplicity
        let mut s = gen_player_message(contest, &ProdNode::new());
        let mut perf = gen_player_message(contest, &ProdNode::new());
        let mut p = gen_player_message(contest, &ProdNode::new());
        let mut t = gen_team_message(contest, &ProdNode::new());
        let mut u = gen_team_message(contest, &LeqNode::new(self.eps));
        let mut l = vec![ProdNode::new(); contest.len()];
        let mut d = vec![GreaterNode::new(2. * self.eps); contest.len() - 1];
        let mut sp = Vec::new();
        let mut pt = Vec::new();
        let mut tul = Vec::new();
        let mut ld = Vec::new();
        let mut players = Vec::new();
        let mut conv = Vec::new();
        let mut old_conv = Vec::new();

        for i in 0..contest.len() {
            for j in 0..contest[i].len() {
                for k in 0..contest[i][j].len() {
                    players.push((contest[i][j][k].clone(), s[i][j][k].add_edge()));
                    RefCell::borrow_mut(&players.last().unwrap().1.upgrade().unwrap()).0 =
                        self.rating.get(&players.last().unwrap().0).unwrap().clone();

                    let mut tmp: Vec<&mut dyn ValueNode> = Vec::with_capacity(3);
                    tmp.push(&mut p[i][j][k]);
                    tmp.push(&mut s[i][j][k]);
                    tmp.push(&mut perf[i][j][k]);
                    sp.push(SumNode::new(&mut tmp));
                    RefCell::borrow_mut(perf[i][j][k].get_edges_mut().last_mut().unwrap()).1 = Gaussian { mu: 0., sigma: self.beta };
                }

                let mut tt: Vec<&mut dyn ValueNode> = vec![&mut t[i][j]];
                for pp in &mut p[i][j] {
                    tt.push(pp);
                }
                pt.push(SumNode::new(&mut tt));
                let mut tmp: Vec<&mut dyn ValueNode> = Vec::with_capacity(3);
                tmp.push(&mut l[i]);
                tmp.push(&mut t[i][j]);
                tmp.push(&mut u[i][j]);
                tul.push(SumNode::new(&mut tmp));
                conv.push(t[i][j].get_edges().last().unwrap().clone());
            }

            if i != 0 {
                let mut tmp: Vec<&mut dyn ValueNode> = Vec::with_capacity(3);
                let (a, b) = l.split_at_mut(i);
                tmp.push(a.last_mut().unwrap());
                tmp.push(b.first_mut().unwrap());
                tmp.push(&mut d[i - 1]);
                ld.push(SumNode::new(&mut tmp));
            }
        }

        infer3(&mut s);
        infer1(&mut sp);
        infer3(&mut p);
        infer1(&mut pt);
        infer2(&mut t);
        infer1(&mut tul);
        infer2(&mut u);
        infer1(&mut tul);

        let mut rounds = 0;

        while check_convergence(&conv, &old_conv) >= self.convergence_eps {
            old_conv.clear();
            for item in &conv {
                old_conv.push(RefCell::borrow(item).clone());
            }
            rounds += 1;

            infer_ld(&mut ld, &mut l);
            infer1(&mut d);
            infer_ld(&mut ld, &mut l);
            infer1(&mut tul);
            infer2(&mut u);
            infer1(&mut tul);
        }

        eprintln!("Rounds until convergence: {}", rounds);

        infer2(&mut t);
        infer1(&mut pt);
        infer3(&mut p);
        infer1(&mut sp);
        infer3(&mut s);

        for (name, mess) in &players {
            let prior;
            let performance;

            prior = RefCell::borrow(&Weak::upgrade(mess).unwrap()).0.clone();
            performance = RefCell::borrow(&Weak::upgrade(mess).unwrap()).1.clone();

            *self.rating.get_mut(name).unwrap() = prior * performance;
        }
    }
}

impl RatingSystem for TrueSkillSPBSystem {
    fn round_update(&mut self, mut standings: Vec<(&mut Player, usize, usize)>) {
        let mut contest = TSContest::new();

        for i in 1..standings.len() {
            assert!(standings[i - 1].1 <= standings[i].1);
        }

        let mut prev = usize::MAX;

        for (user, lo, _hi) in &standings {
            if *lo != prev {
                contest.push(Vec::new());
            }
            contest.last_mut().unwrap().push(vec![user.name.clone()]);

            prev = *lo;
        }

        // load rating
        for place in &contest[..] {
            for team in &place[..] {
                for player in &team[..] {
                    match self.rating.entry(player.clone()) {
                        Entry::Occupied(o) => {
                            let g = o.into_mut();
                            // The multiplier of 1 here assumes time between contests is a constant "time unit"
                            g.sigma = f64::min(self.sigma, (g.sigma.powi(2) + 1. * self.sigma_growth.powi(2)).sqrt());
                        },
                        Entry::Vacant(v) => {
                            v.insert(Gaussian {mu: self.mu, sigma: self.sigma});
                        }
                    }
                }
            }
        }

        // do inference
        self.inference(&contest);

        // update the ratings
        for (user, _, _) in standings {
            match self.rating.entry(user.name.clone()) {
                Entry::Occupied(o) => {
                    let g = o.into_mut();
                    let player_mu = &mut user.approx_posterior.mu;
                    let player_sig = &mut user.approx_posterior.sig;
                    *player_mu = g.mu;
                    *player_sig = g.sigma;
                },
                Entry::Vacant(_) => {
                    println!("Player {} not found in rating system.", user.name.clone());
                }
            }
        }
    }
}