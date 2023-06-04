/// Diffs two parser outputs, ignoring certain kinds of expected differences.
use core::fmt;
use std::collections::HashSet;

use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Copy, Clone, PartialEq)]
enum Key<'a> {
    Idx(usize),
    Field(&'a str),
}

impl<'a> fmt::Display for Key<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Key::Idx(idx) => write!(f, "[{}]", idx),
            Key::Field(key) => write!(f, ".{}", key),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Path<'a>(Vec<Key<'a>>);

impl<'a> fmt::Display for Path<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for key in &self.0 {
            write!(f, "{}", key)?;
        }
        Ok(())
    }
}

impl<'a> Path<'a> {
    fn add(&self, key: Key<'a>) -> Path<'a> {
        let mut path_vec = self.0.clone();
        path_vec.push(key);
        Path(path_vec)
    }
}

struct Diff<'a> {
    left: Option<&'a Value>,
    right: Option<&'a Value>,
    path: Path<'a>,
}

impl<'a> Diff<'a> {
    fn new(left: Option<&'a Value>, right: Option<&'a Value>, path: Path<'a>) -> Diff<'a> {
        Diff { left, right, path }
    }
}

impl<'a> fmt::Display for Diff<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let json_to_string = |json: &Value| serde_json::to_string_pretty(json).unwrap();
        match (self.left, self.right) {
            (None, None) => todo!(),
            (None, Some(right)) => {
                writeln!(f, "  missing from left at {}", self.path)?;
                writeln!(f, "    right:")?;
                writeln!(f, "{}", json_to_string(right))
            }
            (Some(left), None) => {
                writeln!(f, "  missing from right at {}", self.path)?;
                writeln!(f, "    left:")?;
                writeln!(f, "{}", json_to_string(left))
            }
            (Some(left), Some(right)) => {
                writeln!(f, "  diff at {}", self.path)?;
                writeln!(f, "    left:")?;
                writeln!(f, "{}", json_to_string(left))?;
                writeln!(f, "    right:")?;
                writeln!(f, "{}", json_to_string(right))
            }
        }
    }
}

pub fn assert_json_matches<L, R>(left: &L, right: &R)
where
    L: Serialize,
    R: Serialize,
{
    if let Err(msg) = match_json(left, right) {
        panic!("\n{}", msg);
    }
}

pub fn match_json<L, R>(left: &L, right: &R) -> Result<(), String>
where
    L: Serialize,
    R: Serialize,
{
    let left = serde_json::to_value(left)
        .unwrap_or_else(|err| panic!("serde_json::to_value(left) failed: {}", err));
    let right = serde_json::to_value(right)
        .unwrap_or_else(|err| panic!("serde_json::to_value(right) failed: {}", err));
    if let Some(diff) = diff_value(&left, &right, Path(Vec::new())) {
        if diff.path.0.contains(&Key::Field("events")) {
            let mut path = diff.path.clone();
            while let [a, _] = path.0[path.0.len() - 2..] {
                if a == Key::Field("events") {
                    break;
                }
                path.0.pop();
            }
            let mut msg = diff.to_string();
            if let Some(left_event) = follow_path(&left, path.clone()) {
                msg.push_str(format!("  left event:\n{}\n", left_event).as_str());
            }
            if let Some(right_event) = follow_path(&right, path) {
                msg.push_str(format!("  right event:\n{}\n", right_event).as_str());
            }
            Err(msg)
        } else {
            Err(diff.to_string())
        }
    } else {
        Ok(())
    }
}

fn follow_path<'a>(top: &'a Value, path: Path) -> Option<&'a Value> {
    let mut ret = top;
    for k in path.0 {
        match k {
            Key::Idx(idx) => ret = ret.as_array()?.get(idx)?,
            Key::Field(key) => ret = ret.as_object()?.get(key)?,
        }
    }
    Some(ret)
}

fn diff_value<'a>(left: &'a Value, right: &'a Value, path: Path<'a>) -> Option<Diff<'a>> {
    match left {
        Value::Null | Value::Bool(_) => {
            if left != right {
                Some(Diff::new(Some(left), Some(right), path))
            } else {
                None
            }
        }
        Value::String(lefts) => {
            if let Value::String(rights) = right {
                if !approx_eq_string(lefts, rights, path.clone()) {
                    Some(Diff::new(Some(left), Some(right), path))
                } else {
                    None
                }
            } else {
                None
            }
        }
        Value::Number(leftn) => {
            if let Value::Number(rightn) = right {
                if !approx_eq_number(leftn, rightn, path.clone()) {
                    Some(Diff::new(Some(left), Some(right), path))
                } else {
                    None
                }
            } else {
                None
            }
        }
        Value::Array(_) => diff_array(left, right, path),
        Value::Object(_) => diff_object(left, right, path),
    }
}

fn diff_object<'a>(left: &'a Value, right: &'a Value, path: Path<'a>) -> Option<Diff<'a>> {
    if let Some(right) = right.as_object() {
        let left = left.as_object().unwrap();
        for key in sorted_keys(left, right) {
            let path = path.add(Key::Field(key));
            match (left.get(key), right.get(key)) {
                (None, None) => {}
                (None, Some(right)) => return Some(Diff::new(None, Some(right), path)),
                (Some(left), None) => return Some(Diff::new(Some(left), None, path)),
                (Some(left), Some(right)) => {
                    if let Some(diff) = diff_value(left, right, path) {
                        return Some(diff);
                    }
                }
            }
        }
        None
    } else {
        Some(Diff::new(Some(left), Some(right), path))
    }
}

fn diff_array<'a>(left: &'a Value, right: &'a Value, path: Path<'a>) -> Option<Diff<'a>> {
    if let Some(right) = right.as_array() {
        let left = left.as_array().unwrap();
        for idx in 0..left.len().max(right.len()) {
            let path = path.add(Key::Idx(idx));
            match (left.get(idx), right.get(idx)) {
                (None, None) => {}
                (None, Some(right)) => return Some(Diff::new(None, Some(right), path)),
                (Some(left), None) => return Some(Diff::new(Some(left), None, path)),
                (Some(left), Some(right)) => {
                    if let Some(diff) = diff_value(left, right, path) {
                        return Some(diff);
                    }
                }
            }
        }
        None
    } else {
        return Some(Diff::new(Some(left), Some(right), path));
    }
}

fn approx_eq_number(left: &serde_json::Number, right: &serde_json::Number, path: Path) -> bool {
    match path.0.as_slice() {
        // Sometimes events are emitted with userid -1 if a player disconnects.
        // demoinfogo replaces userid -1 with a random player's xuid.
        [.., Key::Field("userid")] => {
            (left == right) || (left.as_i64() == Some(-1)) || (right.as_i64() == Some(-1))
        }
        // demoinfogo omits the fractional part of vector components, which are actually f32.
        [.., Key::Field("attacker_pos"), Key::Idx(_)]
        | [.., Key::Field("victim_pos"), Key::Idx(_)]
        | [.., Key::Field("smoke"), Key::Idx(_), Key::Idx(_)] => {
            let left = left.as_i64().or(left.as_f64().map(|f| f as i64));
            let right = right.as_i64().or(right.as_f64().map(|f| f as i64));
            left == right
        }
        _ => {
            if let (Some(left), Some(right)) = (left.as_f64(), right.as_f64()) {
                (left - right).abs() < 1e-6
            } else {
                left == right
            }
        }
    }
}

fn approx_eq_string(left: &str, right: &str, path: Path) -> bool {
    match path.0.as_slice() {
        // demoinfogo has buggy string decoding for player names
        [.., Key::Field("player_names"), Key::Field(_)]
        | [.., Key::Field("name")]
        | [.., Key::Field("reason")] => true,
        _ => left == right,
    }
}

fn sorted_keys<'a>(
    left: &'a serde_json::Map<String, Value>,
    right: &'a serde_json::Map<String, Value>,
) -> Vec<&'a String> {
    let mut all_keys = left
        .keys()
        .chain(right.keys())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    all_keys.sort();
    all_keys
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn no_diff() {
        let actual = json!({"events": [{ "tick": 132 }]});
        let expected = json!({"events": [{ "tick": 132 }]});
        assert_json_matches(&actual, &expected);
    }

    #[test]
    fn event_pos_fractional_ignored() {
        let actual = json!({"events": [{
            "attacker_pos": [1, 2, 3],
            "victim_pos": [1, 2, 3]
        }]});
        let expected = json!({"events": [{
            "attacker_pos": [1.1, 2.2, 3.3],
            "victim_pos": [1.3, 2.4, 3.5]
        }]});
        assert_json_matches(&actual, &expected);
    }

    #[test]
    #[should_panic(expected = "diff at .events[0]")]
    fn diff_event() {
        let actual = json!({"events": [{ "tick": 1 }]});
        let expected = json!({"events": [{ "tick": 2 }]});
        assert_json_matches(&actual, &expected);
    }

    #[test]
    #[should_panic(expected = "missing from right at .events[1]")]
    fn diff_missing_right() {
        let actual = json!({"events": [{ "tick": 1 }, { "tick": 2 }]});
        let expected = json!({"events": [{ "tick": 1 }]});
        assert_json_matches(&actual, &expected);
    }

    #[test]
    #[should_panic(expected = "missing from left at .events[1]")]
    fn diff_missing_left() {
        let actual = json!({"events": [{ "tick": 1 }]});
        let expected = json!({"events": [{ "tick": 1 }, { "tick": 2 }]});
        assert_json_matches(&actual, &expected);
    }
}
