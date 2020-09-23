use super::normal::Gaussian;
use super::normal::{ONE, ZERO};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

pub type Message = Gaussian;

pub trait TreeNode {
    fn infer(&mut self);
}

pub trait ValueNode: TreeNode {
    fn add_edge(&mut self) -> Weak<RefCell<(Message, Message)>>;
}

pub trait FuncNode: TreeNode {
    fn new(neighbours: &mut [&mut dyn ValueNode]) -> Self;
}

#[derive(Clone)]
pub struct ProdNode {
    edges: Vec<Rc<RefCell<(Message, Message)>>>,
}

#[derive(Clone)]
pub struct LeqNode {
    eps: f64,
    edge: Rc<RefCell<(Message, Message)>>,
}

#[derive(Clone)]
pub struct GreaterNode {
    eps: f64,
    edge: Rc<RefCell<(Message, Message)>>,
}

#[derive(Clone)]
pub struct SumNode {
    out_edge: Weak<RefCell<(Message, Message)>>,
    sum_edges: Vec<Weak<RefCell<(Message, Message)>>>,
}

impl TreeNode for ProdNode {
    fn infer(&mut self) {
        fn get_prefix_prods(from: &[Rc<RefCell<(Message, Message)>>]) -> Vec<Message> {
            let mut prefix_prods = vec![ONE; from.len() + 1];

            for i in 1..prefix_prods.len() {
                prefix_prods[i] = &prefix_prods[i - 1] * &RefCell::borrow(&&from[i - 1]).0;
            }

            prefix_prods
        }
        let prefix_prods = get_prefix_prods(self.edges.as_slice());
        self.edges.reverse();
        let mut suffix_prods = get_prefix_prods(self.edges.as_slice());
        self.edges.reverse();
        suffix_prods.reverse();
        let suffix_prods = suffix_prods;

        for i in 0..self.edges.len() {
            RefCell::borrow_mut(&self.edges[i]).1 = &prefix_prods[i] * &suffix_prods[i + 1];
        }
    }
}

impl ValueNode for ProdNode {
    fn add_edge(&mut self) -> Weak<RefCell<(Message, Message)>> {
        self.edges.push(Rc::new(RefCell::new((ONE, ZERO))));
        Rc::downgrade(&self.edges.last().unwrap())
    }
}

impl ProdNode {
    pub fn get_edges_mut(&mut self) -> &mut Vec<Rc<RefCell<(Message, Message)>>> {
        &mut self.edges
    }

    pub fn get_edges(&self) -> &Vec<Rc<RefCell<(Message, Message)>>> {
        &self.edges
    }

    pub fn new() -> Self {
        ProdNode { edges: Vec::new() }
    }
}

impl TreeNode for LeqNode {
    fn infer(&mut self) {
        let ans;
        {
            ans = RefCell::borrow(&self.edge).0.leq_eps(self.eps);
        }
        RefCell::borrow_mut(&self.edge).1 = ans;
    }
}

impl ValueNode for LeqNode {
    fn add_edge(&mut self) -> Weak<RefCell<(Message, Message)>> {
        Rc::downgrade(&self.edge)
    }
}

impl LeqNode {
    pub fn new(eps: f64) -> LeqNode {
        LeqNode {
            eps,
            edge: Rc::new(RefCell::new((ZERO, ZERO))),
        }
    }
}

impl TreeNode for GreaterNode {
    fn infer(&mut self) {
        let ans;
        {
            ans = RefCell::borrow(&self.edge).0.greater_eps(self.eps);
        }
        RefCell::borrow_mut(&self.edge).1 = ans;
    }
}

impl ValueNode for GreaterNode {
    fn add_edge(&mut self) -> Weak<RefCell<(Message, Message)>> {
        Rc::downgrade(&self.edge)
    }
}

impl GreaterNode {
    pub fn new(eps: f64) -> GreaterNode {
        GreaterNode {
            eps,
            edge: Rc::new(RefCell::new((ZERO, ZERO))),
        }
    }
}

impl FuncNode for SumNode {
    fn new(neighbours: &mut [&mut dyn ValueNode]) -> Self {
        assert!(neighbours.len() >= 1);

        let mut sum_edges = Vec::with_capacity(neighbours.len() - 1);
        for i in 1..neighbours.len() {
            sum_edges.push(neighbours[i].add_edge());
        }

        SumNode {
            out_edge: neighbours.first_mut().unwrap().add_edge(),
            sum_edges,
        }
    }
}

impl TreeNode for SumNode {
    fn infer(&mut self) {
        fn get_prefix_sums(from: &[Weak<RefCell<(Message, Message)>>]) -> Vec<Message> {
            let mut prefix_sums = vec![ZERO; from.len() + 1];

            for i in 1..prefix_sums.len() {
                prefix_sums[i] =
                    &prefix_sums[i - 1] + &RefCell::borrow(&Weak::upgrade(&from[i - 1]).unwrap()).1;
            }

            prefix_sums
        }

        let prefix_sums = get_prefix_sums(self.sum_edges.as_slice());
        self.sum_edges.reverse();
        let mut suffix_sums = get_prefix_sums(self.sum_edges.as_slice());
        self.sum_edges.reverse();
        suffix_sums.reverse();
        let suffix_sums = suffix_sums;

        RefCell::borrow_mut(&self.out_edge.upgrade().unwrap()).0 =
            prefix_sums.last().unwrap().clone();

        for i in 0..self.sum_edges.len() {
            RefCell::borrow_mut(&self.sum_edges[i].upgrade().unwrap()).0 =
                &RefCell::borrow(&self.out_edge.upgrade().unwrap()).1
                    - &prefix_sums[i]
                    - &suffix_sums[i + 1];
        }
    }
}
