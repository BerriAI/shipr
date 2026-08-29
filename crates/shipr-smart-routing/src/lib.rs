use clap::ValueEnum;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Quality {
    Fast,
    Balanced,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Budget {
    Cheapest,
    Cheap,
    Flexible,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouterPolicy {
    pub quality: Quality,
    pub budget: Budget,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutingDecision {
    pub policy: RouterPolicy,
    pub mode: &'static str,
    pub task_kind: &'static str,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelChoice {
    pub name: &'static str,
    pub rationale: &'static str,
    pub estimated_cost: &'static str,
}

pub fn resolve_routing_policy(
    task: &str,
    quality_override: Option<Quality>,
    budget_override: Option<Budget>,
) -> RoutingDecision {
    let (mut policy, task_kind, reason) = infer_smart_policy(task);
    let mut mode = "smart";
    let mut reason_text = reason;

    if let Some(quality) = quality_override {
        policy.quality = quality;
        mode = "manual+smart";
        reason_text.push_str("; quality override provided");
    }

    if let Some(budget) = budget_override {
        policy.budget = budget;
        mode = "manual+smart";
        reason_text.push_str("; budget override provided");
    }

    RoutingDecision {
        policy,
        mode,
        task_kind,
        reason: reason_text,
    }
}

pub fn select_model(policy: &RouterPolicy) -> ModelChoice {
    match (policy.quality, policy.budget) {
        (Quality::Fast, Budget::Cheapest) => ModelChoice {
            name: "fast-mini",
            rationale: "max speed / lowest cost",
            estimated_cost: "$",
        },
        (Quality::High, Budget::Flexible) => ModelChoice {
            name: "reasoning-pro",
            rationale: "deeper coding reasoning",
            estimated_cost: "$$$",
        },
        _ => ModelChoice {
            name: "coder-balanced",
            rationale: "best quality/cost mix",
            estimated_cost: "$$",
        },
    }
}

fn infer_smart_policy(task: &str) -> (RouterPolicy, &'static str, String) {
    let normalized_task = task.to_lowercase();

    let small_edit_keywords = [
        "typo", "readme", "docs", "rename", "format", "lint", "comment", "copy",
    ];
    if let Some(keyword) = first_keyword_match(&normalized_task, &small_edit_keywords) {
        return (
            RouterPolicy {
                quality: Quality::Fast,
                budget: Budget::Cheapest,
            },
            "small edit",
            format!(
                "detected lightweight task signal ('{}'), routing to fastest/cheapest tier",
                keyword
            ),
        );
    }

    let deep_work_keywords = [
        "refactor",
        "architecture",
        "migrate",
        "investigate",
        "root cause",
        "race condition",
        "concurrency",
        "security",
    ];
    if let Some(keyword) = first_keyword_match(&normalized_task, &deep_work_keywords) {
        return (
            RouterPolicy {
                quality: Quality::High,
                budget: Budget::Cheap,
            },
            "deep engineering change",
            format!(
                "detected complex task signal ('{}'), raising quality while staying cost-aware",
                keyword
            ),
        );
    }

    (
        RouterPolicy {
            quality: Quality::Balanced,
            budget: Budget::Cheap,
        },
        "general coding task",
        "using balanced default for quality/cost mix".to_string(),
    )
}

fn first_keyword_match<'a>(text: &str, keywords: &'a [&str]) -> Option<&'a str> {
    keywords
        .iter()
        .copied()
        .find(|keyword| text.contains(keyword))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_small_edits_to_cheapest_fast_tier() {
        let decision = resolve_routing_policy("fix readme typo", None, None);
        assert_eq!(decision.policy.quality, Quality::Fast);
        assert_eq!(decision.policy.budget, Budget::Cheapest);
        assert_eq!(decision.mode, "smart");
    }

    #[test]
    fn routes_complex_work_to_high_quality_cheap_budget() {
        let decision = resolve_routing_policy("investigate race condition", None, None);
        assert_eq!(decision.policy.quality, Quality::High);
        assert_eq!(decision.policy.budget, Budget::Cheap);
        assert_eq!(decision.task_kind, "deep engineering change");
    }

    #[test]
    fn applies_manual_override_without_losing_smart_context() {
        let decision =
            resolve_routing_policy("investigate race condition", None, Some(Budget::Cheapest));
        assert_eq!(decision.policy.quality, Quality::High);
        assert_eq!(decision.policy.budget, Budget::Cheapest);
        assert_eq!(decision.mode, "manual+smart");
        assert!(decision.reason.contains("budget override"));
    }
}
