//! Policy evaluation — the "check before it acts" loop. A pluggable [`Evaluator`] runs a proposed
//! action against the policy's current rules and returns a decision plus a per-rule trace.
//!
//! The default rule format is JSON (no extra dependency): a policy is `{ "rules": [ { "id",
//! "when": { field: { "op", "value" } }, "decision" } ] }`, evaluated first-match-wins over an
//! input map. Values are typed scalars — number, string, or boolean. `eq`/`ne` work across types
//! (a type mismatch is simply unequal); the ordering ops `lt`/`le`/`gt`/`ge` apply to numbers and
//! (lexicographically) to strings, and are false for anything else.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::authoring;
use crate::error::{Result, StudioError};
use crate::workspace::{PolicyId, Workspace};

/// A typed value in a rule condition or an action input. Order matters for `untagged`: a JSON
/// `true` is a Bool, a number is a Num, everything else a Str.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum Scalar {
    Bool(bool),
    Num(f64),
    Str(String),
}

impl Scalar {
    fn describe(&self) -> String {
        match self {
            Scalar::Bool(b) => b.to_string(),
            Scalar::Num(n) => n.to_string(),
            Scalar::Str(s) => format!("\"{s}\""),
        }
    }
}

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
    /// Apply the operator to two scalars. Semantics (total — defined for every type pair):
    /// - `eq`/`ne`: equality within a type (numbers by value, strings/booleans exactly); operands
    ///   of different types are never equal (so `eq` is false, `ne` is true).
    /// - `lt`/`le`/`gt`/`ge`: ordered comparison, defined only for number↔number (IEEE; any `NaN`
    ///   compares as unordered) and string↔string (lexicographic). Every other pairing — booleans,
    ///   or a type mismatch — is unordered and therefore false.
    fn apply(self, lhs: &Scalar, rhs: &Scalar) -> bool {
        match self {
            Op::Eq => lhs == rhs,
            Op::Ne => lhs != rhs,
            Op::Lt => matches!(order(lhs, rhs), Some(Ordering::Less)),
            Op::Le => matches!(order(lhs, rhs), Some(Ordering::Less | Ordering::Equal)),
            Op::Gt => matches!(order(lhs, rhs), Some(Ordering::Greater)),
            Op::Ge => matches!(order(lhs, rhs), Some(Ordering::Greater | Ordering::Equal)),
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

/// Total-ish ordering for the comparable scalar pairs; `None` for type mismatches and booleans.
fn order(lhs: &Scalar, rhs: &Scalar) -> Option<Ordering> {
    match (lhs, rhs) {
        (Scalar::Num(a), Scalar::Num(b)) => a.partial_cmp(b),
        (Scalar::Str(a), Scalar::Str(b)) => Some(a.cmp(b)),
        _ => None,
    }
}

#[derive(Debug, Clone, Deserialize)]
struct Condition {
    op: Op,
    value: Scalar,
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
    fn evaluate(&self, rules: &str, input: &BTreeMap<String, Scalar>) -> Result<Decision>;
}

/// The built-in JSON first-match-wins evaluator.
pub struct DefaultEvaluator;

impl Evaluator for DefaultEvaluator {
    fn evaluate(&self, rules: &str, input: &BTreeMap<String, Scalar>) -> Result<Decision> {
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
fn check(rule: &Rule, input: &BTreeMap<String, Scalar>) -> (bool, String) {
    for (field, cond) in &rule.when {
        let Some(lhs) = input.get(field) else {
            return (false, format!("{field} is absent"));
        };
        if !cond.op.apply(lhs, &cond.value) {
            return (
                false,
                format!(
                    "{field}={} {} {} is false",
                    lhs.describe(),
                    cond.op.symbol(),
                    cond.value.describe()
                ),
            );
        }
    }
    (true, "all conditions hold".to_string())
}

/// Evaluate a proposed action against a policy's current rules with the default evaluator.
pub fn simulate(
    ws: &Workspace,
    id: &PolicyId,
    input: &BTreeMap<String, Scalar>,
) -> Result<Decision> {
    let rules = authoring::current_rules(ws, id)?;
    DefaultEvaluator.evaluate(&rules, input)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn num(n: f64) -> Scalar {
        Scalar::Num(n)
    }
    fn s(v: &str) -> Scalar {
        Scalar::Str(v.to_string())
    }

    fn input(pairs: &[(&str, Scalar)]) -> BTreeMap<String, Scalar> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    const RULES: &str = r#"{
      "rules": [
        { "id": "small",  "when": { "amount": { "op": "le", "value": 50 } },  "decision": "allow" },
        { "id": "medium", "when": { "amount": { "op": "le", "value": 500 } }, "decision": "allow_with_approval" },
        { "id": "large",  "when": { "amount": { "op": "gt", "value": 500 } }, "decision": "deny" }
      ]
    }"#;

    fn eval(amount: f64) -> Decision {
        DefaultEvaluator
            .evaluate(RULES, &input(&[("amount", num(amount))]))
            .unwrap()
    }

    #[test]
    fn first_match_wins_and_boundaries_are_exact() {
        assert_eq!(eval(25.0).decision, "allow");
        assert_eq!(eval(50.0).decision, "allow"); // le 50 includes 50
        assert_eq!(eval(50.0001).decision, "allow_with_approval");
        assert_eq!(eval(500.0).decision, "allow_with_approval");
        assert_eq!(eval(500.0001).decision, "deny"); // gt 500
        assert_eq!(eval(75.0).matched_rule.as_deref(), Some("medium"));
    }

    #[test]
    fn trace_records_until_match_with_symbol() {
        let d = eval(75.0);
        assert_eq!(d.trace.len(), 2);
        assert!(!d.trace[0].matched && d.trace[0].rule_id == "small");
        assert!(d.trace[1].matched);
        assert!(d.trace[0].note.contains("amount=75") && d.trace[0].note.contains("<="));
    }

    #[test]
    fn string_equality_and_ordering() {
        let rules = r#"{"rules":[
          {"id":"vip","when":{"tier":{"op":"eq","value":"vip"}},"decision":"allow"},
          {"id":"hi","when":{"tier":{"op":"gt","value":"m"}},"decision":"review"}
        ]}"#;
        let e = |t: &str| {
            DefaultEvaluator
                .evaluate(rules, &input(&[("tier", s(t))]))
                .unwrap()
        };
        assert_eq!(e("vip").decision, "allow"); // eq
        assert_eq!(e("zeta").decision, "review"); // "zeta" > "m" lexicographically, not vip
        assert_eq!(e("alpha").decision, "no_match"); // "alpha" < "m", not vip
    }

    #[test]
    fn boolean_equality_and_no_ordering() {
        let rules = r#"{"rules":[
          {"id":"flagged","when":{"fraud":{"op":"eq","value":true}},"decision":"deny"},
          {"id":"ord","when":{"fraud":{"op":"gt","value":false}},"decision":"never"}
        ]}"#;
        let e = |b: bool| {
            DefaultEvaluator
                .evaluate(rules, &input(&[("fraud", Scalar::Bool(b))]))
                .unwrap()
        };
        assert_eq!(e(true).decision, "deny"); // eq true
                                              // gt on booleans is never true → "ord" cannot match; fraud=false also != true so first rule misses
        assert_eq!(e(false).decision, "no_match");
    }

    #[test]
    fn type_mismatch_is_unequal_and_unordered() {
        // numeric op against a string input, and vice-versa, never matches
        let rnum = r#"{"rules":[{"id":"r","when":{"x":{"op":"ge","value":10}},"decision":"hit"}]}"#;
        assert_eq!(
            DefaultEvaluator
                .evaluate(rnum, &input(&[("x", s("20"))]))
                .unwrap()
                .decision,
            "no_match" // "20" (string) is not >= 10 (number): unordered
        );
        let req = r#"{"rules":[{"id":"r","when":{"x":{"op":"eq","value":"5"}},"decision":"hit"}]}"#;
        assert_eq!(
            DefaultEvaluator
                .evaluate(req, &input(&[("x", num(5.0))]))
                .unwrap()
                .decision,
            "no_match" // number 5 != string "5"
        );
    }

    #[test]
    fn each_operator_is_exact_at_its_boundary() {
        let decide = |op: &str, value: f64, x: f64| -> String {
            let rules = format!(
                r#"{{"rules":[{{"id":"r","when":{{"x":{{"op":"{op}","value":{value}}}}},"decision":"hit"}}]}}"#
            );
            DefaultEvaluator
                .evaluate(&rules, &input(&[("x", num(x))]))
                .unwrap()
                .decision
        };
        assert_eq!(decide("lt", 10.0, 9.0), "hit");
        assert_eq!(decide("lt", 10.0, 10.0), "no_match");
        assert_eq!(decide("le", 10.0, 10.0), "hit");
        assert_eq!(decide("le", 10.0, 10.001), "no_match");
        assert_eq!(decide("gt", 10.0, 10.0), "no_match");
        assert_eq!(decide("gt", 10.0, 10.001), "hit");
        assert_eq!(decide("ge", 10.0, 10.0), "hit");
        assert_eq!(decide("ge", 10.0, 9.999), "no_match");
        assert_eq!(decide("eq", 10.0, 10.0), "hit");
        assert_eq!(decide("eq", 10.0, 10.001), "no_match");
        assert_eq!(decide("ne", 10.0, 9.0), "hit");
        assert_eq!(decide("ne", 10.0, 10.0), "no_match");
    }

    #[test]
    fn missing_field_and_non_json_handled() {
        let d = DefaultEvaluator.evaluate(RULES, &BTreeMap::new()).unwrap();
        assert_eq!(d.decision, "no_match");
        assert!(d.trace[0].note.contains("absent"));
        assert!(matches!(
            DefaultEvaluator.evaluate("nope: not json", &input(&[("amount", num(1.0))])),
            Err(StudioError::Invalid(_))
        ));
    }

    /// Independent specification of operator semantics — written separately from `Op::apply` so the
    /// rule-of-product test below cross-checks the implementation against the spec, not itself.
    fn oracle(op: Op, a: &Scalar, b: &Scalar) -> bool {
        use std::cmp::Ordering;
        let equal = match (a, b) {
            (Scalar::Num(x), Scalar::Num(y)) => x == y, // NaN == NaN is false (IEEE)
            (Scalar::Str(x), Scalar::Str(y)) => x == y,
            (Scalar::Bool(x), Scalar::Bool(y)) => x == y,
            _ => false, // different types are never equal
        };
        let ord = match (a, b) {
            (Scalar::Num(x), Scalar::Num(y)) => x.partial_cmp(y),
            (Scalar::Str(x), Scalar::Str(y)) => Some(x.cmp(y)),
            _ => None, // booleans and type mismatches are unordered
        };
        match op {
            Op::Eq => equal,
            Op::Ne => !equal,
            Op::Lt => ord == Some(Ordering::Less),
            Op::Le => ord == Some(Ordering::Less) || ord == Some(Ordering::Equal),
            Op::Gt => ord == Some(Ordering::Greater),
            Op::Ge => ord == Some(Ordering::Greater) || ord == Some(Ordering::Equal),
        }
    }

    #[test]
    fn rule_of_product_operator_type_matrix() {
        // Every operator × every (lhs, rhs) over a matrix spanning all three types, both orderings,
        // equal/unequal, and NaN — checked against the independent oracle. 6 × 7 × 7 = 294 cases.
        let operands = [
            Scalar::Num(1.0),
            Scalar::Num(2.0),
            Scalar::Num(f64::NAN),
            Scalar::Str("a".into()),
            Scalar::Str("b".into()),
            Scalar::Bool(false),
            Scalar::Bool(true),
        ];
        let ops = [Op::Eq, Op::Ne, Op::Lt, Op::Le, Op::Gt, Op::Ge];
        let mut checked = 0;
        for op in ops {
            for a in &operands {
                for b in &operands {
                    assert_eq!(op.apply(a, b), oracle(op, a, b), "{op:?} {a:?} {b:?}");
                    checked += 1;
                }
            }
        }
        assert_eq!(checked, 6 * 7 * 7);
    }

    #[test]
    fn ordering_is_reflexive_irreflexive_and_consistent() {
        // Spot the laws the matrix relies on, so a regression names the broken law directly.
        let (lo, hi) = (Scalar::Num(1.0), Scalar::Num(2.0));
        assert!(Op::Le.apply(&lo, &lo) && Op::Ge.apply(&lo, &lo)); // reflexive
        assert!(!Op::Lt.apply(&lo, &lo) && !Op::Gt.apply(&lo, &lo)); // irreflexive
        assert!(Op::Lt.apply(&lo, &hi) && Op::Gt.apply(&hi, &lo)); // antisymmetric
        assert!(!Op::Lt.apply(&hi, &lo) && !Op::Gt.apply(&lo, &hi));
        let nan = Scalar::Num(f64::NAN);
        for op in [Op::Lt, Op::Le, Op::Gt, Op::Ge, Op::Eq] {
            assert!(!op.apply(&nan, &nan), "{op:?} on NaN must be false");
        }
        assert!(Op::Ne.apply(&nan, &nan)); // NaN != NaN
    }
}
