//! Distributional integration: map agent categories to performance distributions.
//!
//! Bridges categorical-agents abstractions with distributional concepts:
//! - **DistributionalFunctor**: maps agent categories to their performance
//!   distributions, analogous to a functor from a category of agents to a
//!   category of distributions.
//! - **Wasserstein-like distance**: measures how far an agent has drifted
//!   from its assigned category's distribution. This is the 1-Wasserstein
//!   distance (Earth Mover's Distance) computed on sorted samples.
//! - **Drift detection**: identifies when an agent's performance has diverged
//!   sufficiently from its category norm to warrant reclassification.
//!
//! The categorical perspective: categories are objects, agents are arrows
//! mapping to distribution objects. Drift is the failure of an arrow to
//! preserve structure.

// ---------------------------------------------------------------------------
// Drift report
// ---------------------------------------------------------------------------

/// Report on how far an agent has drifted from its category.
#[derive(Debug, Clone)]
pub struct DriftReport {
    /// Wasserstein-1 distance between the agent and category mean distribution.
    pub wasserstein_distance: f64,
    /// Per-dimension drift (absolute difference of sorted values).
    pub per_dimension_drift: Vec<f64>,
    /// Whether the drift exceeds the threshold.
    pub is_drifted: bool,
    /// The threshold used for drift detection.
    pub threshold: f64,
}

// ---------------------------------------------------------------------------
// DistributionalFunctor
// ---------------------------------------------------------------------------

/// A functor mapping categories (indexed by usize) to their performance
/// distributions (represented as sorted reference samples).
///
/// This provides the categorical structure for distributional analysis:
/// - **Objects**: categories, each associated with a distribution.
/// - **Morphisms**: agents mapped to categories via `assign_category`.
/// - **Natural transformations**: drift detection via `detect_drift`.
#[derive(Debug, Clone)]
pub struct DistributionalFunctor {
    /// Category distributions: `category_distributions[cat][dim]` is a sorted
    /// sample of performance values for category `cat` on dimension `dim`.
    /// Each category has the same number of dimensions (inner Vecs).
    pub category_distributions: Vec<Vec<Vec<f64>>>,
    /// Drift threshold for detecting when an agent has left its category.
    pub drift_threshold: f64,
}

impl DistributionalFunctor {
    /// Create a new functor with the given category distributions.
    ///
    /// Each category distribution is a `Vec<Vec<f64>>` where the outer Vec
    /// indexes dimensions and the inner Vec is a list of sample values.
    pub fn new(category_distributions: Vec<Vec<Vec<f64>>>, drift_threshold: f64) -> Self {
        // Pre-sort each dimension's samples for efficient Wasserstein computation
        let mut distributions = category_distributions;
        for cat in &mut distributions {
            for dim in cat {
                dim.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            }
        }
        Self {
            category_distributions: distributions,
            drift_threshold,
        }
    }

    /// Number of categories.
    pub fn num_categories(&self) -> usize {
        self.category_distributions.len()
    }

    /// Number of dimensions per category.
    pub fn num_dimensions(&self) -> usize {
        self.category_distributions.first().map_or(0, |c| c.len())
    }

    // ----- Wasserstein-1 distance -----------------------------------------

    /// Compute the 1-Wasserstein distance between two sorted sample arrays.
    ///
    /// For equal-length sorted arrays, this is simply the mean absolute
    /// difference between corresponding elements.
    pub fn wasserstein_1(a: &[f64], b: &[f64]) -> f64 {
        if a.is_empty() || b.is_empty() {
            return f64::INFINITY;
        }
        // Sort copies
        let mut sa = a.to_vec();
        let mut sb = b.to_vec();
        sa.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));
        sb.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));

        // If different lengths, interpolate by truncating to shorter
        let n = sa.len().min(sb.len());
        let mut total = 0.0;
        for i in 0..n {
            total += (sa[i] - sb[i]).abs();
        }
        total / n as f64
    }

    /// Compute the mean (centroid) of a set of samples.
    pub fn mean(samples: &[f64]) -> f64 {
        if samples.is_empty() {
            return 0.0;
        }
        samples.iter().sum::<f64>() / samples.len() as f64
    }

    /// Compute the category mean vector (one mean per dimension).
    pub fn category_mean(&self, category: usize) -> Vec<f64> {
        if category >= self.category_distributions.len() {
            return vec![];
        }
        self.category_distributions[category]
            .iter()
            .map(|dim_samples| Self::mean(dim_samples))
            .collect()
    }

    // ----- assign_category ------------------------------------------------

    /// Assign an agent to the nearest category based on Wasserstein distance.
    ///
    /// The agent's performance vector is compared against each category's
    /// mean distribution. The category with the smallest total Wasserstein
    /// distance across all dimensions is chosen.
    pub fn assign_category(&self, agent_performance: &[f64]) -> usize {
        if self.category_distributions.is_empty() {
            return 0;
        }
        let mut best_category = 0;
        let mut best_distance = f64::INFINITY;

        for (cat_idx, cat) in self.category_distributions.iter().enumerate() {
            let mut total_distance = 0.0;
            for (dim, samples) in cat.iter().enumerate() {
                if dim < agent_performance.len() {
                    // Compare agent value against category distribution
                    let agent_val = agent_performance[dim];
                    // Distance from agent value to the distribution's mean
                    let dist = (agent_val - Self::mean(samples)).abs();
                    total_distance += dist;
                }
            }
            if total_distance < best_distance {
                best_distance = total_distance;
                best_category = cat_idx;
            }
        }
        best_category
    }

    // ----- detect_drift ---------------------------------------------------

    /// Detect how far an agent has drifted from a category mean.
    ///
    /// Computes a Wasserstein-like distance between the agent's performance
    /// vector and the category's mean vector, along with per-dimension drift.
    pub fn detect_drift(&self, agent: &[f64], category_mean: &[f64]) -> DriftReport {
        let n = agent.len().min(category_mean.len());
        if n == 0 {
            return DriftReport {
                wasserstein_distance: 0.0,
                per_dimension_drift: vec![],
                is_drifted: false,
                threshold: self.drift_threshold,
            };
        }

        // Sort both for Wasserstein computation
        let mut agent_sorted = agent.to_vec();
        let mut mean_sorted = category_mean.to_vec();
        agent_sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        mean_sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let mut per_dim = Vec::with_capacity(n);
        let mut total = 0.0;
        for i in 0..n {
            let d = (agent_sorted[i] - mean_sorted[i]).abs();
            per_dim.push(d);
            total += d;
        }
        let wasserstein = total / n as f64;

        DriftReport {
            wasserstein_distance: wasserstein,
            per_dimension_drift: per_dim,
            is_drifted: wasserstein > self.drift_threshold,
            threshold: self.drift_threshold,
        }
    }

    // ----- fmap (functor map) ---------------------------------------------

    /// Apply a transformation to all category distributions (functor map).
    ///
    /// This is the functorial action: mapping a morphism in the base category
    /// (a function f64 → f64) over the distributions.
    pub fn fmap<F: Fn(f64) -> f64>(&self, f: F) -> DistributionalFunctor {
        let new_dists: Vec<Vec<Vec<f64>>> = self
            .category_distributions
            .iter()
            .map(|cat| {
                cat.iter()
                    .map(|dim_samples| {
                        let mut new_samples: Vec<f64> =
                            dim_samples.iter().map(|&x| f(x)).collect();
                        new_samples.sort_by(|a, b| {
                            a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
                        });
                        new_samples
                    })
                    .collect()
            })
            .collect();
        DistributionalFunctor {
            category_distributions: new_dists,
            drift_threshold: self.drift_threshold,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_functor() -> DistributionalFunctor {
        // Category 0: low performers (values around 1-3)
        // Category 1: high performers (values around 8-10)
        DistributionalFunctor::new(
            vec![
                // Category 0: 3 dimensions
                vec![
                    vec![1.0, 2.0, 3.0], // dim 0
                    vec![1.5, 2.0, 2.5], // dim 1
                    vec![2.0, 2.5, 3.0], // dim 2
                ],
                // Category 1
                vec![
                    vec![8.0, 9.0, 10.0],
                    vec![8.5, 9.0, 9.5],
                    vec![9.0, 9.5, 10.0],
                ],
            ],
            2.0, // drift threshold
        )
    }

    #[test]
    fn test_assign_category_low_performer() {
        let functor = make_test_functor();
        let agent = vec![2.0, 2.0, 2.5]; // close to category 0
        assert_eq!(functor.assign_category(&agent), 0);
    }

    #[test]
    fn test_assign_category_high_performer() {
        let functor = make_test_functor();
        let agent = vec![9.0, 9.0, 9.5]; // close to category 1
        assert_eq!(functor.assign_category(&agent), 1);
    }

    #[test]
    fn test_assign_category_empty_functor() {
        let functor = DistributionalFunctor::new(vec![], 1.0);
        assert_eq!(functor.assign_category(&[1.0, 2.0]), 0);
    }

    #[test]
    fn test_detect_drift_no_drift() {
        let functor = make_test_functor();
        let agent = vec![2.0, 2.0, 2.5];
        let cat_mean = functor.category_mean(0);
        let report = functor.detect_drift(&agent, &cat_mean);
        assert!(!report.is_drifted, "agent close to category 0 mean should not drift");
    }

    #[test]
    fn test_detect_drift_yes_drift() {
        let functor = make_test_functor();
        // Agent is far from category 0 mean
        let agent = vec![20.0, 20.0, 20.0];
        let cat_mean = vec![2.0, 2.0, 2.5];
        let report = functor.detect_drift(&agent, &cat_mean);
        assert!(report.is_drifted, "agent far from category should drift");
        assert!(report.wasserstein_distance > 2.0);
    }

    #[test]
    fn test_detect_drift_empty() {
        let functor = make_test_functor();
        let report = functor.detect_drift(&[], &[]);
        assert!(!report.is_drifted);
        assert_eq!(report.wasserstein_distance, 0.0);
    }

    #[test]
    fn test_wasserstein_1_identical() {
        let a = vec![1.0, 2.0, 3.0];
        assert_eq!(DistributionalFunctor::wasserstein_1(&a, &a), 0.0);
    }

    #[test]
    fn test_wasserstein_1_shifted() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![2.0, 3.0, 4.0];
        let dist = DistributionalFunctor::wasserstein_1(&a, &b);
        assert!((dist - 1.0).abs() < 1e-10, "shifted by 1 should give distance 1");
    }

    #[test]
    fn test_category_mean() {
        let functor = make_test_functor();
        let mean0 = functor.category_mean(0);
        // dim 0: (1+2+3)/3 = 2.0
        assert!((mean0[0] - 2.0).abs() < 1e-10);
        // dim 1: (1.5+2+2.5)/3 = 2.0
        assert!((mean0[1] - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_fmap_scales_distributions() {
        let functor = make_test_functor();
        let scaled = functor.fmap(|x| x * 2.0);
        // Category 0 dim 0 was [1,2,3] → should now be [2,4,6]
        assert!((scaled.category_distributions[0][0][0] - 2.0).abs() < 1e-10);
        assert!((scaled.category_distributions[0][0][2] - 6.0).abs() < 1e-10);
    }

    #[test]
    fn test_per_dimension_drift_populated() {
        let functor = make_test_functor();
        let agent = vec![5.0, 5.0, 5.0];
        let cat_mean = vec![2.0, 2.0, 2.0];
        let report = functor.detect_drift(&agent, &cat_mean);
        assert_eq!(report.per_dimension_drift.len(), 3);
        // After sorting: agent sorted [5,5,5], mean sorted [2,2,2] → all dims drift = 3.0
        for d in &report.per_dimension_drift {
            assert!((d - 3.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_num_categories_and_dimensions() {
        let functor = make_test_functor();
        assert_eq!(functor.num_categories(), 2);
        assert_eq!(functor.num_dimensions(), 3);
    }

    #[test]
    fn test_mean_empty() {
        assert_eq!(DistributionalFunctor::mean(&[]), 0.0);
    }

    #[test]
    fn test_wasserstein_empty_inputs() {
        assert!(DistributionalFunctor::wasserstein_1(&[], &[1.0]).is_infinite());
    }
}
