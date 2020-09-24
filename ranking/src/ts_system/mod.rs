mod nodes;
mod normal;

use super::compute_ratings::{Player, Rating, RatingSystem, MU_NEWBIE};

use nodes::{FuncNode, GreaterNode, LeqNode, ProdNode, SumNode, TreeNode, ValueNode};
use normal::Gaussian;

use std::cell::RefCell;
use std::rc::Rc;

type TSMessage = nodes::Message;
type TSPlayer<'a> = (&'a mut Player, Gaussian);
type TSTeam<'a> = Vec<TSPlayer<'a>>;
type TSContestPlace<'a> = Vec<TSTeam<'a>>;
type TSContest<'a> = Vec<TSContestPlace<'a>>;

// TrueSkillStPB rating system
pub struct TrueSkillSPBSystem {
    // epsilon used for ties
    eps: f64,
    // performance sigma
    beta: f64,
    // epsilon used for convergence loop
    convergence_eps: f64,
    // defines sigma growth per second
    sigma_growth: f64,
}

impl Default for TrueSkillSPBSystem {
    fn default() -> Self {
        Self {
            eps: 0.10,
            beta: MU_NEWBIE / 6., // sigma/2
            convergence_eps: 2e-4,
            sigma_growth: 5.,
        }
    }
}

fn gen_team_message<T, K: Clone>(places: &[Vec<T>], default: &K) -> Vec<Vec<K>> {
    places
        .iter()
        .map(|place| vec![default.clone(); place.len()])
        .collect()
}

fn gen_player_message<T, K: Clone>(places: &[Vec<Vec<T>>], default: &K) -> Vec<Vec<Vec<K>>> {
    places
        .iter()
        .map(|place| {
            place
                .iter()
                .map(|team| vec![default.clone(); team.len()])
                .collect()
        })
        .collect()
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
    for i in (0..ld.len()).rev() {
        ld[i].infer();
        l[i].infer();
    }
}

fn check_convergence(
    a: &[Rc<RefCell<(TSMessage, TSMessage)>>],
    b: &[(TSMessage, TSMessage)],
) -> f64 {
    if a.len() != b.len() {
        return std::f64::INFINITY;
    }

    a.iter()
        .map(|ai| ai.borrow())
        .zip(b.iter())
        .flat_map(|(ai, bi)| {
            vec![
                ai.0.mu - bi.0.mu,
                ai.0.sigma - bi.0.sigma,
                ai.1.mu - bi.1.mu,
                ai.1.sigma - bi.1.sigma,
            ]
        })
        .map(f64::abs)
        .max_by(|x, y| x.partial_cmp(y).expect("Difference became NaN"))
        .unwrap_or(0.)
}

impl TrueSkillSPBSystem {
    fn inference(&self, contest: &mut TSContest) {
        if contest.is_empty() {
            return;
        }

        // could be optimized, written that way for simplicity
        // TODO: invent better variable names
        let mut s = gen_player_message(contest, &ProdNode::new());
        let mut perf = gen_player_message(contest, &ProdNode::new());
        let mut p = gen_player_message(contest, &ProdNode::new());
        let mut t = gen_team_message(contest, &ProdNode::new());
        let mut u = gen_team_message(contest, &LeqNode::new(self.eps));
        let mut l = vec![ProdNode::new(); contest.len()];
        let mut d = vec![GreaterNode::new(2. * self.eps); contest.len() - 1];
        let mut sp = vec![];
        let mut pt = vec![];
        let mut tul = vec![];
        let mut ld = vec![];
        let mut players = vec![];
        let mut conv = vec![];
        let mut old_conv = vec![];

        for i in 0..contest.len() {
            for j in 0..contest[i].len() {
                for k in 0..contest[i][j].len() {
                    let new_edge = s[i][j][k].add_edge();

                    new_edge.upgrade().unwrap().borrow_mut().0 = contest[i][j][k].1.clone();

                    sp.push(SumNode::new(&mut [
                        &mut p[i][j][k],
                        &mut s[i][j][k],
                        &mut perf[i][j][k],
                    ]));
                    RefCell::borrow_mut(perf[i][j][k].get_edges_mut().last_mut().unwrap()).1 =
                        Gaussian {
                            mu: 0.,
                            sigma: self.beta,
                        };

                    players.push((i, j, k, new_edge));
                }

                let mut tt: Vec<&mut dyn ValueNode> = vec![&mut t[i][j]];
                tt.extend(p[i][j].iter_mut().map(|pp| pp as &mut dyn ValueNode));

                pt.push(SumNode::new(&mut tt));
                tul.push(SumNode::new(&mut [&mut l[i], &mut t[i][j], &mut u[i][j]]));
                conv.push(t[i][j].get_edges().last().unwrap().clone());
            }

            if i != 0 {
                let mut tmp: Vec<&mut dyn ValueNode> = Vec::with_capacity(3);
                let (pref, suf) = l.split_at_mut(i);
                tmp.push(pref.last_mut().unwrap());
                tmp.push(suf.first_mut().unwrap());
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

        for (i, j, k, mess) in players {
            let val = mess.upgrade().unwrap();
            let (prior, performance) = &*val.borrow();
            let (player, gaussian) = &mut contest[i][j][k];

            *gaussian = prior * performance;
            player.approx_posterior.mu = gaussian.mu;
            player.approx_posterior.sig = gaussian.sigma;
        }
    }
}

impl RatingSystem for TrueSkillSPBSystem {
    fn win_probability(&self, player: &Rating, foe: &Rating) -> f64 {
        0.5 // TODO
    }

    fn round_update(&self, standings: Vec<(&mut Player, usize, usize)>) {
        let mut contest = TSContest::new();

        for i in 1..standings.len() {
            assert!(standings[i - 1].1 <= standings[i].1);
        }

        // The multiplier of 1 here assumes time between contests is a constant "time unit"
        let noise = 1. * self.sigma_growth.powi(2);
        let mut prev = usize::MAX;
        for (user, lo, _hi) in standings {
            if lo != prev {
                contest.push(vec![]);
            }
            let gaussian = Gaussian {
                mu: user.approx_posterior.mu,
                sigma: (user.approx_posterior.sig.powi(2) + noise).sqrt(),
            };
            contest.last_mut().unwrap().push(vec![(user, gaussian)]);
            prev = lo;
        }

        // do inference
        self.inference(&mut contest);
    }
}
