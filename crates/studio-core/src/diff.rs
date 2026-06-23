//! A line-based LCS diff — pure, used for the pre-commit preview (current rules vs a proposed
//! edit). The `.aion` format retains only the *current* rules plus a chain of version hashes, so
//! historical rule bodies aren't available to diff; the authoring diff is always current→proposed.

/// What happened to a line going from old → new.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DiffTag {
    Same,
    Add,
    Del,
}

/// One line of a diff.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct DiffLine {
    pub tag: DiffTag,
    pub text: String,
}

/// Line-based LCS diff of `old` → `new`, preserving order.
pub fn diff_lines(old: &str, new: &str) -> Vec<DiffLine> {
    let a: Vec<&str> = old.lines().collect();
    let b: Vec<&str> = new.lines().collect();
    let dp = lcs_table(&a, &b);
    backtrack(&a, &b, &dp)
}

/// `dp[i][j]` = length of the LCS of `a[i..]` and `b[j..]`.
fn lcs_table(a: &[&str], b: &[&str]) -> Vec<Vec<usize>> {
    let (n, m) = (a.len(), b.len());
    let mut dp = vec![vec![0usize; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            dp[i][j] = if a[i] == b[j] {
                dp[i + 1][j + 1] + 1
            } else {
                dp[i + 1][j].max(dp[i][j + 1])
            };
        }
    }
    dp
}

fn backtrack(a: &[&str], b: &[&str], dp: &[Vec<usize>]) -> Vec<DiffLine> {
    let mut out = Vec::new();
    let (n, m) = (a.len(), b.len());
    let (mut i, mut j) = (0, 0);
    while i < n && j < m {
        if a[i] == b[j] {
            out.push(line(DiffTag::Same, a[i]));
            i += 1;
            j += 1;
        } else if dp[i + 1][j] == dp[i][j] {
            // skipping a[i] keeps the optimal LCS (dp[i][j] == max(dp[i+1][j], dp[i][j+1])),
            // so delete it; otherwise dp[i][j+1] is the max and we add b[j].
            out.push(line(DiffTag::Del, a[i]));
            i += 1;
        } else {
            out.push(line(DiffTag::Add, b[j]));
            j += 1;
        }
    }
    while i < n {
        out.push(line(DiffTag::Del, a[i]));
        i += 1;
    }
    while j < m {
        out.push(line(DiffTag::Add, b[j]));
        j += 1;
    }
    out
}

fn line(tag: DiffTag, text: &str) -> DiffLine {
    DiffLine {
        tag,
        text: text.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tags(d: &[DiffLine]) -> Vec<DiffTag> {
        d.iter().map(|l| l.tag).collect()
    }

    #[test]
    fn identical_is_all_same() {
        let d = diff_lines("a\nb\nc", "a\nb\nc");
        assert_eq!(tags(&d), [DiffTag::Same, DiffTag::Same, DiffTag::Same]);
        assert_eq!(d[1].text, "b");
    }

    #[test]
    fn insertion_and_deletion() {
        // insert "x" after "a"
        assert_eq!(
            tags(&diff_lines("a\nb", "a\nx\nb")),
            [DiffTag::Same, DiffTag::Add, DiffTag::Same]
        );
        // delete "b"
        assert_eq!(
            tags(&diff_lines("a\nb\nc", "a\nc")),
            [DiffTag::Same, DiffTag::Del, DiffTag::Same]
        );
    }

    #[test]
    fn replacement_shows_del_then_add() {
        // change the middle line: LCS keeps a,c; b→y is del b, add y
        let d = diff_lines("a\nb\nc", "a\ny\nc");
        assert_eq!(
            tags(&d),
            [DiffTag::Same, DiffTag::Del, DiffTag::Add, DiffTag::Same]
        );
        assert_eq!(d[1].text, "b");
        assert_eq!(d[2].text, "y");
    }

    #[test]
    fn empty_to_content_is_all_add() {
        assert_eq!(tags(&diff_lines("", "a\nb")), [DiffTag::Add, DiffTag::Add]);
        assert_eq!(tags(&diff_lines("a\nb", "")), [DiffTag::Del, DiffTag::Del]);
    }

    #[test]
    fn interleaved_lcs_is_exact() {
        // a..e vs a,x,c,y,e — the LCS must thread the middle (a,c,e); any wrong dp index access
        // produces a different alignment, so this pins the index arithmetic in the table+backtrack.
        let d = diff_lines("a\nb\nc\nd\ne", "a\nx\nc\ny\ne");
        use DiffTag::{Add, Del, Same};
        assert_eq!(tags(&d), [Same, Del, Add, Same, Del, Add, Same]);
        let text = |t: DiffTag| -> Vec<String> {
            d.iter()
                .filter(|l| l.tag == t)
                .map(|l| l.text.clone())
                .collect()
        };
        assert_eq!(text(Same), ["a", "c", "e"]);
        assert_eq!(text(Del), ["b", "d"]);
        assert_eq!(text(Add), ["x", "y"]);
    }

    #[test]
    fn longer_common_run_is_preferred() {
        // LCS of "a b c" and "a c b c" is "a b c" (length 3), not "a c" — pins that the table
        // maximizes correctly (a `+`→`*`/`-` on an index would shrink/misplace the run).
        let d = diff_lines("a\nb\nc", "a\nc\nb\nc");
        use DiffTag::{Add, Same};
        assert_eq!(tags(&d), [Same, Add, Same, Same]);
        assert_eq!(d.iter().filter(|l| l.tag == Add).count(), 1);
    }

    #[test]
    fn same_count_equals_lcs_length_exhaustively() {
        // The number of Same lines must equal the true LCS length, for EVERY sequence pair up to
        // length 4 over {a,b,c}. Independently brute-forced here, so any wrong dp index access that
        // picks a sub-optimal path on some input is caught.
        fn lcs(a: &[u8], b: &[u8]) -> usize {
            match (a.split_first(), b.split_first()) {
                (Some((x, ar)), Some((y, br))) => {
                    if x == y {
                        1 + lcs(ar, br)
                    } else {
                        lcs(ar, b).max(lcs(a, br))
                    }
                }
                _ => 0,
            }
        }
        let join = |s: &[u8]| -> String {
            s.iter()
                .map(|c| (*c as char).to_string())
                .collect::<Vec<_>>()
                .join("\n")
        };
        let mut corpus: Vec<Vec<u8>> = vec![vec![]];
        let mut frontier = vec![vec![]];
        for _ in 0..4 {
            let mut next = Vec::new();
            for s in &frontier {
                for &c in b"abc" {
                    let mut t = s.clone();
                    t.push(c);
                    next.push(t);
                }
            }
            corpus.extend(next.iter().cloned());
            frontier = next;
        }
        for a in &corpus {
            for b in &corpus {
                let d = diff_lines(&join(a), &join(b));
                let same = d.iter().filter(|l| l.tag == DiffTag::Same).count();
                assert_eq!(same, lcs(a, b), "a={a:?} b={b:?}");
            }
        }
    }
}
