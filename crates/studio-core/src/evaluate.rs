//! Policy evaluation — the "check before it acts" loop. A pluggable [`Evaluator`] runs a proposed
//! action against the policy's current rules and returns a decision plus a per-rule trace.
//!
//! The default rule format is JSON (no extra dependency); a policy is `{ "rules": [ { "id",
//! "when": { field: { "op", "value" } }, "decision" } ] }`, evaluated first-match-wins over a
//! numeric input map. A different rule language can implement [`Evaluator`] without touching the
//! API or UI.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::authoring;
use crate::error::{Result, StudioError};
use crate::workspace::{PolicyId, Workspace};

/// A numeric comparison operator.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Op {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

impl Op {
    fn apply(self, lhs: f64, rhs: f64) -> bool {
        match self {
            Op::Eq => lhs == rhs,
            Op::Ne => lhs != rhs,
            Op::Lt => lhs < rhs,
            Op::Le => lhs <= rhs,
            Op::Gt => lhs > rhs,
            Op::Ge => lhs >= rhs,
        }
    }

    fn symbol(self) -> &'static str {
        match self {
            Op::Eq => "==",
            Op::Ne => "!=",
            Op::Lt => "<",
            Op::Le => "<=",
            Op::Gt => ">",
            Op::Ge => ">=",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct Condition {
    op: Op,
    value: f64,
}

#[derive(Debug, Clone, Deserialize)]
struct Rule {
    id: String,
    #[serde(default)]
    when: BTreeMap<String, Condition>,
    decision: String,
}

#[derive(Debug, Clone, Deserialize)]
struct RuleSet {
    rules: Vec<Rule>,
}

/// One step of the evaluation trace.
#[derive(Debug, Clone, Serialize)]
pub struct TraceStep {
    pub rule_id: String,
    pub matched: bool,
    pub note: String,
}

/// The outcome of evaluating an action against a policy.
#[derive(Debug, Clone, Serialize)]
pub struct Decision {
    /// The matched rule's decision, or `"no_match"` if none matched.
    pub decision: String,
    pub matched_rule: Option<String>,
    pub trace: Vec<TraceStep>,
}

/// A policy evaluator. Implement this to support a different rule language.
pub trait Evaluator {
    fn evaluate(&self, rules: &str, input: &BTreeMap<String, f64>) -> Result<Decision>;
}

/// The built-in JSON first-match-wins evaluator.
pub struct DefaultEvaluator;

impl Evaluator for DefaultEvaluator {
    fn evaluate(&self, rules: &str, input: &BTreeMap<String, f64>) -> Result<Decision> {
        let set: RuleSet = serde_json::from_str(rules).map_err(|e| {
            StudioError::Invalid(format!("rules are not in the default JSON format: {e}"))
        })?;
        let mut trace = Vec::with_capacity(set.rules.len());
        for rule in &set.rules {
            let (matched, note) = check(rule, input);
            trace.push(TraceStep {
                rule_id: rule.id.clone(),
                matched,
                note,
            });
            if matched {
                return Ok(Decision {
                    decision: rule.decision.clone(),
                    matched_rule: Some(rule.id.clone()),
                    trace,
                });
            }
        }
        Ok(Decision {
            decision: "no_match".to_string(),
            matched_rule: None,
            trace,
        })
    }
}

/// Does every condition of `rule` hold for `input`? Returns the verdict and a human note.
fn check(rule: &Rule, input: &BTreeMap<String, f64>) -> (bool, String) {
    for (field, cond) in &rule.when {
        let Some(&lhs) = input.get(field) else {
            return (false, format!("{field} is absent"));
        };
        if !cond.op.apply(lhs, cond.value) {
            return (
                false,
                format!("{field}={lhs} {} {} is false", cond.op.symbol(), cond.value),
            );
        }
    }
    (true, "all conditions hold".to_string())
}

/// Evaluate a proposed action against a policy's current rules with the default evaluator.
pub fn simulate(ws: &Workspace, id: &PolicyId, input: &BTreeMap<String, f64>) -> Result<Decision> {
    let rules = authoring::current_rules(ws, id)?;
    DefaultEvaluator.evaluate(&rules, input)
}

#[cfg(test)]
mod tests {
    use super::*;

    const RULES: &str = r#"{
      "rules": [
        { "id": "small",   "when": { "amount": { "op": "le", "value": 50 } },  "decision": "allow" },
        { "id": "medium",  "when": { "amount": { "op": "le", "value": 500 } }, "decision": "allow_with_approval" },
        { "id": "large",   "when": { "amount": { "op": "gt", "value": 500 } }, "decision": "deny" }
      ]
    }"#;

    fn input(amount: f64) -> BTreeMap<String, f64> {
        BTreeMap::from([("amount".to_string(), amount)])
    }

    fn eval(amount: f64) -> Decision {
        DefaultEvaluator.evaluate(RULES, &input(amount)).unwrap()
    }

    #[test]
    fn first_match_wins_across_decisions() {
        assert_eq!(eval(25.0).decision, "allow");
        assert_eq!(eval(25.0).matched_rule.as_deref(), Some("small"));
        assert_eq!(eval(75.0).decision, "allow_with_approval"); // skips "small", matches "medium"
        assert_eq!(eval(999.0).decision, "deny");
    }

    #[test]
    fn boundaries_are_exact() {
        assert_eq!(eval(50.0).decision, "allow"); // le 50 includes 50
        assert_eq!(eval(50.0001).decision, "allow_with_approval");
        assert_eq!(eval(500.0).decision, "allow_with_approval"); // le 500 includes 500
        assert_eq!(eval(500.0001).decision, "deny"); // gt 500
    }

    #[test]
    fn trace_records_each_rule_until_match() {
        let d = eval(75.0);
        assert_eq!(d.trace.len(), 2); // small (checked, no), medium (matched) — large not reached
        assert!(!d.trace[0].matched && d.trace[0].rule_id == "small");
        assert!(d.trace[1].matched && d.trace[1].rule_id == "medium");
        assert!(d.trace[0].note.contains("amount=75"));
    }

    #[test]
    fn every_operator_is_exercised() {
        let ops = r#"{"rules":[
          {"id":"eq","when":{"x":{"op":"eq","value":10}},"decision":"eq"},
          {"id":"ne","when":{"x":{"op":"ne","value":10}},"decision":"ne"},
          {"id":"lt","when":{"x":{"op":"lt","value":10}},"decision":"lt"},
          {"id":"ge","when":{"x":{"op":"ge","value":10}},"decision":"ge"}
        ]}"#;
        let e = |x: f64| {
            DefaultEvaluator
                .evaluate(ops, &BTreeMap::from([("x".to_string(), x)]))
                .unwrap()
                .decision
        };
        assert_eq!(e(10.0), "eq"); // eq matches first
        assert_eq!(e(3.0), "ne"); // ne true, then lt also true but ne is first
        assert_eq!(e(20.0), "ne"); // 20 != 10
    }

    #[test]
    fn missing_field_does_not_match_and_no_match_is_reported() {
        let d = DefaultEvaluator.evaluate(RULES, &BTreeMap::new()).unwrap();
        assert_eq!(d.decision, "no_match");
        assert!(d.matched_rule.is_none());
        assert!(d.trace.iter().all(|s| !s.matched));
        assert!(d.trace[0].note.contains("absent"));
    }

    #[test]
    fn non_json_rules_are_an_invalid_error() {
        let r = DefaultEvaluator.evaluate("policy: not-json\nrules: []", &input(1.0));
        assert!(matches!(r, Err(StudioError::Invalid(_))));
    }

    #[test]
    fn each_operator_is_exact_at_its_boundary() {
        // single-rule policy so the operator under test is the deciding one (no shadowing)
        let decide = |op: &str, value: f64, x: f64| -> String {
            let rules = format!(
                r#"{{"rules":[{{"id":"r","when":{{"x":{{"op":"{op}","value":{value}}}}},"decision":"hit"}}]}}"#
            );
            DefaultEvaluator
                .evaluate(&rules, &BTreeMap::from([("x".to_string(), x)]))
                .unwrap()
                .decision
        };
        assert_eq!(decide("lt", 10.0, 9.0), "hit");
        assert_eq!(decide("lt", 10.0, 10.0), "no_match"); // not < ; kills <→<=,==,>
        assert_eq!(decide("le", 10.0, 10.0), "hit");
        assert_eq!(decide("le", 10.0, 10.001), "no_match");
        assert_eq!(decide("gt", 10.0, 10.0), "no_match"); // not > ; kills >→>=
        assert_eq!(decide("gt", 10.0, 10.001), "hit");
        assert_eq!(decide("ge", 10.0, 10.0), "hit"); // kills >=→>
        assert_eq!(decide("ge", 10.0, 9.999), "no_match"); // kills >=→<
        assert_eq!(decide("eq", 10.0, 10.0), "hit");
        assert_eq!(decide("eq", 10.0, 10.001), "no_match");
        assert_eq!(decide("ne", 10.0, 9.0), "hit");
        assert_eq!(decide("ne", 10.0, 10.0), "no_match");
    }

    #[test]
    fn trace_note_includes_the_operator_symbol() {
        let rules =
            r#"{"rules":[{"id":"r","when":{"x":{"op":"le","value":50}},"decision":"hit"}]}"#;
        let d = DefaultEvaluator
            .evaluate(rules, &BTreeMap::from([("x".to_string(), 75.0)]))
            .unwrap();
        assert_eq!(d.decision, "no_match");
        assert!(d.trace[0].note.contains("<="), "note: {}", d.trace[0].note);
    }
}
