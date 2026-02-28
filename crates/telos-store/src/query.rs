//! Query functions for filtering Telos objects

use telos_core::hash::ObjectId;
use telos_core::object::decision_record::DecisionRecord;
use telos_core::object::intent::Intent;
use telos_core::object::TelosObject;

use crate::error::StoreError;
use crate::odb::ObjectDatabase;

/// Query intents with optional filters.
pub fn query_intents(
    odb: &ObjectDatabase,
    impact: Option<&str>,
    constraint_contains: Option<&str>,
) -> Result<Vec<(ObjectId, Intent)>, StoreError> {
    let all = odb.iter_all()?;
    let mut results = Vec::new();
    for (id, obj) in all {
        if let TelosObject::Intent(intent) = obj {
            let mut matches = true;
            if let Some(impact_filter) = impact {
                if !intent.impacts.iter().any(|i| i == impact_filter) {
                    matches = false;
                }
            }
            if let Some(constraint_filter) = constraint_contains {
                if !intent
                    .constraints
                    .iter()
                    .any(|c| c.to_lowercase().contains(&constraint_filter.to_lowercase()))
                {
                    matches = false;
                }
            }
            if matches {
                results.push((id, intent));
            }
        }
    }
    // Sort by timestamp descending (most recent first)
    results.sort_by(|a, b| b.1.timestamp.cmp(&a.1.timestamp));
    Ok(results)
}

/// Query decision records with optional filters.
pub fn query_decisions(
    odb: &ObjectDatabase,
    intent_id: Option<&ObjectId>,
    tag: Option<&str>,
) -> Result<Vec<(ObjectId, DecisionRecord)>, StoreError> {
    let all = odb.iter_all()?;
    let mut results = Vec::new();
    for (id, obj) in all {
        if let TelosObject::DecisionRecord(record) = obj {
            let mut matches = true;
            if let Some(filter_id) = intent_id {
                if &record.intent_id != filter_id {
                    matches = false;
                }
            }
            if let Some(tag_filter) = tag {
                if !record.tags.iter().any(|t| t == tag_filter) {
                    matches = false;
                }
            }
            if matches {
                results.push((id, record));
            }
        }
    }
    // Sort by timestamp descending
    results.sort_by(|a, b| b.1.timestamp.cmp(&a.1.timestamp));
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::collections::HashMap;
    use telos_core::object::intent::Author;

    fn make_odb() -> (tempfile::TempDir, ObjectDatabase) {
        let dir = tempfile::TempDir::new().unwrap();
        let odb = ObjectDatabase::new(dir.path().join("objects"));
        (dir, odb)
    }

    fn make_intent(statement: &str, impacts: Vec<&str>, constraints: Vec<&str>) -> Intent {
        Intent {
            author: Author {
                name: "Test".into(),
                email: "test@test.com".into(),
            },
            timestamp: Utc::now(),
            statement: statement.into(),
            constraints: constraints.into_iter().map(String::from).collect(),
            behavior_spec: vec![],
            parents: vec![],
            impacts: impacts.into_iter().map(String::from).collect(),
            behavior_diff: None,
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn query_intents_by_impact() {
        let (_dir, odb) = make_odb();
        let i1 = make_intent("Auth setup", vec!["auth"], vec![]);
        let i2 = make_intent("Task CRUD", vec!["tasks"], vec![]);
        let i3 = make_intent("Auth tokens", vec!["auth", "security"], vec![]);

        odb.write(&TelosObject::Intent(i1)).unwrap();
        odb.write(&TelosObject::Intent(i2)).unwrap();
        odb.write(&TelosObject::Intent(i3)).unwrap();

        let results = query_intents(&odb, Some("auth"), None).unwrap();
        assert_eq!(results.len(), 2);
        assert!(results
            .iter()
            .all(|(_, i)| i.impacts.contains(&"auth".to_string())));
    }

    #[test]
    fn query_intents_by_constraint() {
        let (_dir, odb) = make_odb();
        let i1 = make_intent("Auth setup", vec!["auth"], vec!["Token expiry <= 1 hour"]);
        let i2 = make_intent("Task CRUD", vec!["tasks"], vec!["Must validate input"]);

        odb.write(&TelosObject::Intent(i1)).unwrap();
        odb.write(&TelosObject::Intent(i2)).unwrap();

        let results = query_intents(&odb, None, Some("token")).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].1.constraints[0].contains("Token"));
    }

    #[test]
    fn query_decisions_by_intent() {
        let (_dir, odb) = make_odb();
        let intent = make_intent("Auth setup", vec!["auth"], vec![]);
        let intent_id = odb.write(&TelosObject::Intent(intent)).unwrap();

        let record = DecisionRecord {
            intent_id: intent_id.clone(),
            author: Author {
                name: "Test".into(),
                email: "test@test.com".into(),
            },
            timestamp: Utc::now(),
            question: "Which token format?".into(),
            decision: "JWT".into(),
            rationale: Some("Industry standard".into()),
            alternatives: vec![],
            tags: vec!["auth".into()],
        };
        odb.write(&TelosObject::DecisionRecord(record)).unwrap();

        let results = query_decisions(&odb, Some(&intent_id), None).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].1.decision, "JWT");
    }

    #[test]
    fn query_decisions_by_tag() {
        let (_dir, odb) = make_odb();
        let intent = make_intent("Auth setup", vec!["auth"], vec![]);
        let intent_id = odb.write(&TelosObject::Intent(intent)).unwrap();

        let r1 = DecisionRecord {
            intent_id: intent_id.clone(),
            author: Author {
                name: "Test".into(),
                email: "test@test.com".into(),
            },
            timestamp: Utc::now(),
            question: "Token format?".into(),
            decision: "JWT".into(),
            rationale: None,
            alternatives: vec![],
            tags: vec!["auth".into(), "security".into()],
        };
        let r2 = DecisionRecord {
            intent_id: intent_id.clone(),
            author: Author {
                name: "Test".into(),
                email: "test@test.com".into(),
            },
            timestamp: Utc::now(),
            question: "DB choice?".into(),
            decision: "Postgres".into(),
            rationale: None,
            alternatives: vec![],
            tags: vec!["infra".into()],
        };
        odb.write(&TelosObject::DecisionRecord(r1)).unwrap();
        odb.write(&TelosObject::DecisionRecord(r2)).unwrap();

        let results = query_decisions(&odb, None, Some("auth")).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].1.decision, "JWT");
    }
}
