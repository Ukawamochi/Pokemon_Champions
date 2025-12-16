use crate::battle::{evaluate_state, Battle, BattleResult, PlayerAction, Side};
use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MctsMode {
    /// 同時手番の完全展開（(my_action, opp_action) のペアで枝を持つ）
    Joint,
    /// 相手行動をロールアウトでサンプリングする簡易モード（木は自分の行動のみ展開）
    MyActionOnly,
}

#[derive(Clone, Debug)]
pub struct MctsParams {
    pub iterations: Option<usize>,
    pub time_budget: Option<Duration>,
    pub rollout_horizon: usize,
    pub exploration_constant: f64,
    pub mode: MctsMode,
}

impl Default for MctsParams {
    fn default() -> Self {
        Self {
            iterations: Some(200),
            time_budget: None,
            rollout_horizon: 0,
            exploration_constant: 1.414,
            mode: MctsMode::Joint,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
struct JointAction {
    my: Option<PlayerAction>,
    opp: Option<PlayerAction>,
}

struct Node {
    state: Battle,
    visits: u64,
    total_value: f64,
    children: HashMap<JointAction, usize>,
    unexpanded: Vec<JointAction>,
}

impl Node {
    fn new(state: Battle, side: Side, params: &MctsParams) -> Self {
        let unexpanded = enumerate_actions(&state, side, params);
        Node {
            state,
            visits: 0,
            total_value: 0.0,
            children: HashMap::new(),
            unexpanded,
        }
    }
}

pub fn mcts_action(
    state: &Battle,
    side: Side,
    params: &MctsParams,
    seed: u64,
) -> Option<PlayerAction> {
    let root_actions = enumerate_actions(state, side, params);
    if root_actions.is_empty() {
        return None;
    }
    let max_iters = iteration_cap(params);
    let start = Instant::now();
    let mut nodes: Vec<Node> = Vec::new();
    nodes.push(Node::new(state.clone(), side, params));
    let mut iterations = 0usize;

    while iterations < max_iters
        && params
            .time_budget
            .map(|limit| start.elapsed() < limit)
            .unwrap_or(true)
    {
        iterations += 1;
        let iter_seed = mix_seed(seed, iterations as u64, 0);
        let mut rollout_rng = SmallRng::seed_from_u64(iter_seed);
        let mut path: Vec<usize> = Vec::new();
        let mut node_idx = 0usize;
        let mut depth = 0usize;

        loop {
            path.push(node_idx);
            if let Some(result) = nodes[node_idx].state.terminal_result() {
                let reward = outcome_score(result, side);
                backprop(&mut nodes, &path, reward);
                break;
            }

            if let Some(action) = nodes[node_idx].unexpanded.pop() {
                let child_state = next_state(
                    &nodes[node_idx].state,
                    action,
                    params,
                    &mut rollout_rng,
                    iter_seed,
                    depth + 1,
                    side,
                );
                let child_idx = nodes.len();
                nodes.push(Node::new(child_state, side, params));
                nodes[node_idx].children.insert(action, child_idx);
                path.push(child_idx);

                let reward = rollout(
                    nodes[child_idx].state.clone(),
                    side,
                    params,
                    &mut rollout_rng,
                );
                backprop(&mut nodes, &path, reward);
                break;
            }

            if nodes[node_idx].children.is_empty() {
                let reward = evaluate_state(&nodes[node_idx].state, side) as f64;
                backprop(&mut nodes, &path, reward);
                break;
            }

            let (_action, next_idx) = select_child(node_idx, &nodes, params.exploration_constant);
            if let Some(next_idx) = next_idx {
                node_idx = next_idx;
                depth += 1;
            } else {
                // no valid child (all zero visits with NaN?)
                let reward = evaluate_state(&nodes[node_idx].state, side) as f64;
                backprop(&mut nodes, &path, reward);
                break;
            }
        }
    }

    best_root_action(&nodes[0], &nodes)
}

fn iteration_cap(params: &MctsParams) -> usize {
    params.iterations.unwrap_or_else(|| {
        if params.time_budget.is_some() {
            usize::MAX
        } else {
            200
        }
    })
}

fn outcome_score(result: BattleResult, perspective: Side) -> f64 {
    match (result, perspective) {
        (BattleResult::AWins, Side::A) => 1.0,
        (BattleResult::BWins, Side::B) => 1.0,
        (BattleResult::Tie, _) => 0.5,
        _ => 0.0,
    }
}

fn enumerate_actions(state: &Battle, side: Side, params: &MctsParams) -> Vec<JointAction> {
    let my_actions = state.legal_actions(side);
    let opp_actions = state.legal_actions(side.opponent());

    match params.mode {
        MctsMode::Joint => {
            let my_list: Vec<Option<PlayerAction>> = if my_actions.is_empty() {
                vec![None]
            } else {
                my_actions.into_iter().map(Some).collect()
            };
            let opp_list: Vec<Option<PlayerAction>> = if opp_actions.is_empty() {
                vec![None]
            } else {
                opp_actions.into_iter().map(Some).collect()
            };
            let mut pairs = Vec::new();
            for my in &my_list {
                for opp in &opp_list {
                    pairs.push(JointAction { my: *my, opp: *opp });
                }
            }
            pairs
        }
        MctsMode::MyActionOnly => {
            if my_actions.is_empty() {
                vec![JointAction {
                    my: None,
                    opp: None,
                }]
            } else {
                my_actions
                    .into_iter()
                    .map(|a| JointAction {
                        my: Some(a),
                        opp: None,
                    })
                    .collect()
            }
        }
    }
}

fn next_state(
    state: &Battle,
    action: JointAction,
    params: &MctsParams,
    rng: &mut SmallRng,
    iter_seed: u64,
    depth: usize,
    perspective: Side,
) -> Battle {
    let mut next = state.clone_with_rng_seed(mix_seed(iter_seed, depth as u64, 1));
    let opp_action = match params.mode {
        MctsMode::Joint => action.opp,
        MctsMode::MyActionOnly => sample_action(&next, perspective.opponent(), rng),
    };
    next.run_turn_with_actions(action.my, opp_action);
    next
}

fn rollout(mut state: Battle, perspective: Side, params: &MctsParams, rng: &mut SmallRng) -> f64 {
    let max_turns = if params.rollout_horizon == 0 {
        500
    } else {
        params.rollout_horizon
    };
    for turn in 0..max_turns {
        if let Some(result) = state.terminal_result() {
            return outcome_score(result, perspective);
        }
        if params.rollout_horizon > 0 && turn >= params.rollout_horizon {
            break;
        }
        let my_action = sample_action(&state, perspective, rng);
        let opp_action = sample_action(&state, perspective.opponent(), rng);
        state.run_turn_with_actions(my_action, opp_action);
    }
    evaluate_state(&state, perspective) as f64
}

fn sample_action(state: &Battle, side: Side, rng: &mut SmallRng) -> Option<PlayerAction> {
    let actions = state.legal_actions(side);
    if actions.is_empty() {
        None
    } else {
        actions.choose(rng).cloned()
    }
}

fn backprop(nodes: &mut [Node], path: &[usize], reward: f64) {
    for &idx in path {
        if let Some(node) = nodes.get_mut(idx) {
            node.visits += 1;
            node.total_value += reward;
        }
    }
}

fn select_child(node_idx: usize, nodes: &[Node], c: f64) -> (Option<JointAction>, Option<usize>) {
    let node = &nodes[node_idx];
    let parent_visits = node.visits.max(1) as f64;
    let mut best: Option<(f64, JointAction, usize)> = None;
    for (action, child_idx) in &node.children {
        let child = &nodes[*child_idx];
        let visits = child.visits as f64;
        let exploitation = if child.visits == 0 {
            f64::INFINITY
        } else {
            child.total_value / visits
        };
        let exploration = if child.visits == 0 {
            0.0
        } else {
            c * (parent_visits.ln() / visits).sqrt()
        };
        let score = exploitation + exploration;
        match best {
            None => best = Some((score, *action, *child_idx)),
            Some((current, _, _)) if score > current => best = Some((score, *action, *child_idx)),
            _ => {}
        }
    }
    if let Some((_, action, idx)) = best {
        (Some(action), Some(idx))
    } else {
        (None, None)
    }
}

fn best_root_action(root: &Node, nodes: &[Node]) -> Option<PlayerAction> {
    let mut aggregates: HashMap<Option<PlayerAction>, (f64, u64)> = HashMap::new();
    for (action, &child_idx) in &root.children {
        let child = &nodes[child_idx];
        let entry = aggregates.entry(action.my).or_insert((0.0, 0));
        entry.0 += child.total_value;
        entry.1 += child.visits;
    }
    aggregates
        .into_iter()
        .filter(|(_, (_, visits))| *visits > 0)
        .max_by(|a, b| {
            let avg_a = a.1 .0 / a.1 .1 as f64;
            let avg_b = b.1 .0 / b.1 .1 as f64;
            avg_a.partial_cmp(&avg_b).unwrap_or(Ordering::Equal)
        })
        .map(|(action, _)| action)
        .unwrap_or_else(|| root.unexpanded.get(0).and_then(|a| a.my))
}

fn mix_seed(base: u64, a: u64, b: u64) -> u64 {
    let mut x = base ^ a.wrapping_mul(0x9E3779B97F4A7C15);
    x ^= b.wrapping_mul(0xC2B2AE3D27D4EB4F);
    x ^= x >> 33;
    x = x.wrapping_mul(0xff51afd7ed558ccd);
    x ^= x >> 33;
    x = x.wrapping_mul(0xc4ceb9fe1a85ec53);
    x ^ (x >> 33)
}
