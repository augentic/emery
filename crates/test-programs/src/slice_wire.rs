//! Wire forms of build slices for the scripted target call log.

#![expect(missing_docs, reason = "serde wire shapes for the call log only")]

use emery_adapter::target::{DesignType, Requirement, Scenario, Slice};
use emery_adapter::{Route, Trigger};
use serde::{Deserialize, Serialize};

/// A build slice as recorded on the scripted target's call log.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SliceWire {
    /// Stable slice id.
    pub id: String,
    /// Stable part id.
    pub part: String,
    /// Part name.
    pub name: String,
    /// Slice index within its part.
    pub index: u32,
    /// Slice count within its part.
    pub count: u32,
    /// Head the slice builds over.
    pub base: String,
    /// Requirements in the slice.
    pub requirements: Vec<RequirementWire>,
    /// Owned design types.
    pub types: Vec<DesignTypeWire>,
    /// Part brief paragraphs.
    pub brief: Vec<String>,
    /// Cut specification.
    pub spec: String,
    /// Whole design.
    pub design: String,
    /// Part plan entry.
    pub plan: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RequirementWire {
    pub id: String,
    pub subject: String,
    pub statement: Vec<String>,
    pub scenarios: Vec<ScenarioWire>,
    pub trigger: Option<TriggerWire>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScenarioWire {
    pub name: String,
    pub given: Vec<String>,
    pub when: String,
    pub then: String,
    pub and: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case", tag = "kind")]
pub enum TriggerWire {
    Http { method: String, path: String },
    Message(String),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DesignTypeWire {
    pub key: String,
    pub signature: String,
}

impl SliceWire {
    /// Drops the large documents the guest should not serialise on its stack.
    #[must_use]
    pub fn compact(slice: &Slice) -> Self {
        let mut wire = Self::from(slice);
        wire.spec.clear();
        wire.design.clear();
        wire.plan.clear();
        wire
    }
}

impl From<&Slice> for SliceWire {
    fn from(slice: &Slice) -> Self {
        Self {
            id: slice.id.clone(),
            part: slice.part.clone(),
            name: slice.name.clone(),
            index: slice.index,
            count: slice.count,
            base: slice.base.clone(),
            requirements: slice.requirements.iter().map(RequirementWire::from).collect(),
            types: slice.types.iter().map(DesignTypeWire::from).collect(),
            brief: slice.brief.clone(),
            spec: slice.spec.clone(),
            design: slice.design.clone(),
            plan: slice.plan.clone(),
        }
    }
}

impl From<&Requirement> for RequirementWire {
    fn from(requirement: &Requirement) -> Self {
        Self {
            id: requirement.id.clone(),
            subject: requirement.subject.clone(),
            statement: requirement.statement.clone(),
            scenarios: requirement.scenarios.iter().map(ScenarioWire::from).collect(),
            trigger: requirement.trigger.as_ref().map(TriggerWire::from),
        }
    }
}

impl From<&Scenario> for ScenarioWire {
    fn from(scenario: &Scenario) -> Self {
        Self {
            name: scenario.name.clone(),
            given: scenario.given.clone(),
            when: scenario.when.clone(),
            then: scenario.then.clone(),
            and: scenario.and.clone(),
        }
    }
}

impl From<&Trigger> for TriggerWire {
    fn from(trigger: &Trigger) -> Self {
        match trigger {
            Trigger::Http(route) => Self::Http {
                method: route.method.clone(),
                path: route.path.clone(),
            },
            Trigger::Message(topic) => Self::Message(topic.clone()),
        }
    }
}

impl From<&DesignType> for DesignTypeWire {
    fn from(kind: &DesignType) -> Self {
        Self {
            key: kind.key.clone(),
            signature: kind.signature.clone(),
        }
    }
}

impl From<SliceWire> for Slice {
    fn from(wire: SliceWire) -> Self {
        Self {
            id: wire.id,
            part: wire.part,
            name: wire.name,
            index: wire.index,
            count: wire.count,
            base: wire.base,
            requirements: wire.requirements.into_iter().map(Requirement::from).collect(),
            types: wire.types.into_iter().map(DesignType::from).collect(),
            brief: wire.brief,
            spec: wire.spec,
            design: wire.design,
            plan: wire.plan,
        }
    }
}

impl From<RequirementWire> for Requirement {
    fn from(wire: RequirementWire) -> Self {
        Self {
            id: wire.id,
            subject: wire.subject,
            statement: wire.statement,
            scenarios: wire.scenarios.into_iter().map(Scenario::from).collect(),
            trigger: wire.trigger.map(Trigger::from),
        }
    }
}

impl From<ScenarioWire> for Scenario {
    fn from(wire: ScenarioWire) -> Self {
        Self {
            name: wire.name,
            given: wire.given,
            when: wire.when,
            then: wire.then,
            and: wire.and,
        }
    }
}

impl From<TriggerWire> for Trigger {
    fn from(wire: TriggerWire) -> Self {
        match wire {
            TriggerWire::Http { method, path } => Self::Http(Route { method, path }),
            TriggerWire::Message(topic) => Self::Message(topic),
        }
    }
}

impl From<DesignTypeWire> for DesignType {
    fn from(wire: DesignTypeWire) -> Self {
        Self {
            key: wire.key,
            signature: wire.signature,
        }
    }
}
