//! Plugin Delay Compensation (PDC) calculation.

use super::node::NodeId;
use std::collections::{HashMap, HashSet};

/// Calculates delay compensation across DAG branches to ensure phase alignment at summing points.
pub struct PluginDelayCompensation;

impl PluginDelayCompensation {
    /// Computes required compensation delay in samples for each node in a graph.
    ///
    /// # Arguments
    /// * `node_latencies`: Mapping of `NodeId` -> intrinsic latency in samples.
    /// * `edges`: Directed edges `(from, to)`.
    /// * `terminal_nodes`: Sinks / summing junctions where signals combine or exit.
    pub fn calculate_compensation(
        node_latencies: &HashMap<NodeId, usize>,
        edges: &[(NodeId, NodeId)],
        terminal_nodes: &[NodeId],
    ) -> HashMap<NodeId, usize> {
        let mut adj: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
        let mut in_degree: HashMap<NodeId, usize> = HashMap::new();
        let mut all_nodes: HashSet<NodeId> = HashSet::new();

        for &(from, to) in edges {
            adj.entry(from).or_default().push(to);
            *in_degree.entry(to).or_insert(0) += 1;
            all_nodes.insert(from);
            all_nodes.insert(to);
        }

        for &id in node_latencies.keys() {
            all_nodes.insert(id);
            in_degree.entry(id).or_insert(0);
        }

        // Find sources (in_degree == 0)
        let sources: Vec<NodeId> = all_nodes
            .iter()
            .copied()
            .filter(|&id| in_degree.get(&id).copied().unwrap_or(0) == 0)
            .collect();

        // Calculate longest path latency from sources to each node
        let mut max_latency_to_node: HashMap<NodeId, usize> = HashMap::new();
        for &src in &sources {
            let lat = node_latencies.get(&src).copied().unwrap_or(0);
            max_latency_to_node.insert(src, lat);
        }

        // Topological traversal
        let mut queue: Vec<NodeId> = sources.clone();
        let mut in_deg_tracker = in_degree.clone();

        while let Some(u) = queue.pop() {
            let u_lat = max_latency_to_node.get(&u).copied().unwrap_or(0);
            if let Some(neighbors) = adj.get(&u) {
                for &v in neighbors {
                    let v_node_lat = node_latencies.get(&v).copied().unwrap_or(0);
                    let candidate = u_lat + v_node_lat;
                    let current = max_latency_to_node.entry(v).or_insert(0);
                    if candidate > *current {
                        *current = candidate;
                    }

                    if let Some(deg) = in_deg_tracker.get_mut(&v) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push(v);
                        }
                    }
                }
            }
        }

        // Overall max latency to terminal nodes
        let target_terminal_latency = terminal_nodes
            .iter()
            .map(|t| max_latency_to_node.get(t).copied().unwrap_or(0))
            .max()
            .unwrap_or(0);

        // Compensation needed per node path
        let mut compensation_delays: HashMap<NodeId, usize> = HashMap::new();
        for &t in terminal_nodes {
            let path_lat = max_latency_to_node.get(&t).copied().unwrap_or(0);
            let comp = target_terminal_latency.saturating_sub(path_lat);
            compensation_delays.insert(t, comp);
        }

        // Also check intermediate converging junctions
        for (&node, parents) in &in_degree {
            if *parents > 1 {
                // Find all incoming paths to this junction and ensure they arrive simultaneously
                let max_in = edges
                    .iter()
                    .filter(|(_, to)| *to == node)
                    .map(|(from, _)| max_latency_to_node.get(from).copied().unwrap_or(0))
                    .max()
                    .unwrap_or(0);

                for (from, to) in edges {
                    if *to == node {
                        let from_lat = max_latency_to_node.get(from).copied().unwrap_or(0);
                        let branch_comp = max_in.saturating_sub(from_lat);
                        if branch_comp > 0 {
                            *compensation_delays.entry(*from).or_insert(0) += branch_comp;
                        }
                    }
                }
            }
        }

        compensation_delays
    }
}
