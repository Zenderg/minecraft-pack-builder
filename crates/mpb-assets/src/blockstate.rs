use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockstateModelCondition {
    pub any_of: Vec<BTreeMap<String, Vec<String>>>,
}

#[derive(Debug, Clone)]
pub struct BlockstateModelReference {
    pub condition: Option<BlockstateModelCondition>,
    pub model: String,
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub uv_lock: bool,
}

#[derive(Debug, Clone, Default)]
pub struct BlockstateModelReferences {
    pub variants_are_multipart: bool,
    pub models: Vec<BlockstateModelReference>,
    pub state_definitions: BTreeMap<String, BTreeSet<String>>,
}

pub fn collect_blockstate_models(value: &serde_json::Value) -> BlockstateModelReferences {
    if let Some(multipart) = value.get("multipart").and_then(serde_json::Value::as_array) {
        let models = collect_multipart_models(multipart);
        return BlockstateModelReferences {
            variants_are_multipart: true,
            state_definitions: state_definitions_from_models(&models),
            models,
        };
    }

    let models = value
        .get("variants")
        .and_then(serde_json::Value::as_object)
        .map(|variants| {
            variants
                .iter()
                .flat_map(|(condition, model)| {
                    collect_model_references(parse_variant_condition(condition), model)
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let state_definitions = state_definitions_from_models(&models);

    BlockstateModelReferences {
        variants_are_multipart: false,
        state_definitions,
        models,
    }
}

fn collect_multipart_models(multipart: &[serde_json::Value]) -> Vec<BlockstateModelReference> {
    multipart
        .iter()
        .flat_map(|entry| {
            let condition = entry.get("when").and_then(parse_when_condition);
            entry
                .get("apply")
                .map(|apply| collect_model_references(condition, apply))
                .unwrap_or_default()
        })
        .collect()
}

fn collect_model_references(
    condition: Option<BlockstateModelCondition>,
    value: &serde_json::Value,
) -> Vec<BlockstateModelReference> {
    if let Some(array) = value.as_array() {
        return array
            .first()
            .map(|first| collect_model_references(condition, first))
            .unwrap_or_default();
    }

    let Some(model) = value.get("model").and_then(serde_json::Value::as_str) else {
        return Vec::new();
    };

    vec![BlockstateModelReference {
        condition,
        model: model.to_string(),
        x: value
            .get("x")
            .and_then(serde_json::Value::as_f64)
            .map(|value| value as f32),
        y: value
            .get("y")
            .and_then(serde_json::Value::as_f64)
            .map(|value| value as f32),
        uv_lock: value
            .get("uvlock")
            .or_else(|| value.get("uvLock"))
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
    }]
}

fn parse_variant_condition(value: &str) -> Option<BlockstateModelCondition> {
    if value.trim().is_empty() {
        return None;
    }
    let states = value
        .split(',')
        .filter_map(|part| {
            let (key, values) = part.split_once('=')?;
            Some((
                key.trim().to_string(),
                values
                    .split('|')
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty())
                    .collect::<Vec<_>>(),
            ))
        })
        .filter(|(key, values)| !key.is_empty() && !values.is_empty())
        .collect::<BTreeMap<_, _>>();
    (!states.is_empty()).then_some(BlockstateModelCondition {
        any_of: vec![states],
    })
}

fn parse_when_condition(value: &serde_json::Value) -> Option<BlockstateModelCondition> {
    let object = value.as_object()?;
    if let Some(or_values) = object.get("OR").and_then(serde_json::Value::as_array) {
        let any_of = or_values
            .iter()
            .filter_map(parse_simple_state_map)
            .collect::<Vec<_>>();
        return (!any_of.is_empty()).then_some(BlockstateModelCondition { any_of });
    }
    parse_simple_state_map(value).map(|states| BlockstateModelCondition {
        any_of: vec![states],
    })
}

fn parse_simple_state_map(value: &serde_json::Value) -> Option<BTreeMap<String, Vec<String>>> {
    let object = value.as_object()?;
    let states = object
        .iter()
        .filter_map(|(key, value)| {
            let text = value.as_str()?;
            Some((
                key.clone(),
                text.split('|')
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty())
                    .collect::<Vec<_>>(),
            ))
        })
        .filter(|(_, values)| !values.is_empty())
        .collect::<BTreeMap<_, _>>();
    (!states.is_empty()).then_some(states)
}

fn state_definitions_from_models(
    models: &[BlockstateModelReference],
) -> BTreeMap<String, BTreeSet<String>> {
    let mut definitions = BTreeMap::new();
    for condition in models.iter().filter_map(|model| model.condition.as_ref()) {
        for states in &condition.any_of {
            for (name, values) in states {
                definitions
                    .entry(name.clone())
                    .or_insert_with(BTreeSet::new)
                    .extend(values.iter().cloned());
            }
        }
    }
    for values in definitions.values_mut() {
        if values
            .iter()
            .all(|value| value == "true" || value == "false")
        {
            values.insert("false".to_string());
            values.insert("true".to_string());
        }
    }
    definitions
}
