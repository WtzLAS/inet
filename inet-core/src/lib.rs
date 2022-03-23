use std::collections::{HashMap, VecDeque};

use snafu::{OptionExt, ResultExt, Snafu};
use time::OffsetDateTime;
use uuid::{
    v1::{Context, Timestamp},
    Uuid,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AgentTypeId {
    Name,
    Indirection,
    Custom(Uuid),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Agent {
    pub type_id: AgentTypeId,
    pub ports: Vec<Uuid>,
}

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("the agent {id} is accidentally removed"))]
    MissingAgent { id: Uuid },
    #[snafu(display("no rule for {lhs_type_id} >< {rhs_type_id}"))]
    NoRule {
        lhs_type_id: Uuid,
        rhs_type_id: Uuid,
    },
    #[snafu(display("error when generating uuid"))]
    Uuid { source: uuid::Error },
}

type RuleFn = fn(&mut Machine, &Uuid, &Uuid);

pub struct Machine {
    uuid_context: Context,
    pub rules: HashMap<(Uuid, Uuid), RuleFn>,
    pub agents: HashMap<Uuid, Agent>,
    pub eqs: VecDeque<(Uuid, Uuid)>,
}

impl Machine {
    pub fn new() -> Machine {
        Machine {
            uuid_context: Context::new(0),
            rules: HashMap::new(),
            agents: HashMap::new(),
            eqs: VecDeque::new(),
        }
    }

    pub fn generate_id(&self) -> Result<Uuid, Error> {
        let time = OffsetDateTime::now_utc();
        let timestamp = Timestamp::from_unix(
            &self.uuid_context,
            time.unix_timestamp() as u64,
            time.nanosecond(),
        );
        Uuid::new_v1(timestamp, &[0; 6]).context(UuidSnafu)
    }

    pub fn new_name(&mut self) -> Result<Uuid, Error> {
        let id = self.generate_id()?;
        self.agents.insert(
            id,
            Agent {
                type_id: AgentTypeId::Name,
                ports: vec![],
            },
        );
        Ok(id)
    }

    pub fn new_custom(
        &mut self,
        type_id: Uuid,
        principal_port: Uuid,
        aux_ports: Vec<Uuid>,
    ) -> Result<Uuid, Error> {
        let id = self.generate_id()?;
        self.eqs.push_back((id, principal_port));
        self.agents.insert(
            id,
            Agent {
                type_id: AgentTypeId::Custom(type_id),
                ports: aux_ports,
            },
        );
        Ok(id)
    }

    pub fn eval(&mut self) -> Result<(usize, usize, usize), Error> {
        let mut interactions: usize = 0;
        let mut name_op: usize = 0;
        let mut ind_op: usize = 0;
        while let Some((lhs_id, rhs_id)) = self.eqs.pop_back() {
            let lhs_type_id = self
                .agents
                .get(&lhs_id)
                .context(MissingAgentSnafu { id: lhs_id })?
                .type_id;
            let rhs_type_id = self
                .agents
                .get(&rhs_id)
                .context(MissingAgentSnafu { id: rhs_id })?
                .type_id;
            match (lhs_type_id, rhs_type_id) {
                (_, AgentTypeId::Indirection) => {
                    ind_op += 1;
                    let rhs_target = self
                        .agents
                        .get(&rhs_id)
                        .context(MissingAgentSnafu { id: rhs_id })?
                        .ports[0];
                    self.agents.remove(&rhs_id);
                    self.eqs.push_back((lhs_id, rhs_target));
                }
                (_, AgentTypeId::Name) => {
                    name_op += 1;
                    let rhs_agent = self
                        .agents
                        .get_mut(&rhs_id)
                        .context(MissingAgentSnafu { id: rhs_id })?;
                    rhs_agent.type_id = AgentTypeId::Indirection;
                    rhs_agent.ports.resize(1, Uuid::default());
                    rhs_agent.ports[0] = lhs_id;
                }
                (AgentTypeId::Indirection, _) => {
                    ind_op += 1;
                    let lhs_target = self
                        .agents
                        .get(&lhs_id)
                        .context(MissingAgentSnafu { id: lhs_id })?
                        .ports[0];
                    self.agents.remove(&lhs_id);
                    self.eqs.push_back((lhs_target, rhs_id));
                }
                (AgentTypeId::Name, _) => {
                    name_op += 1;
                    let lhs_agent = self
                        .agents
                        .get_mut(&lhs_id)
                        .context(MissingAgentSnafu { id: lhs_id })?;
                    lhs_agent.type_id = AgentTypeId::Indirection;
                    lhs_agent.ports.resize(1, Uuid::default());
                    lhs_agent.ports[0] = rhs_id;
                }
                (AgentTypeId::Custom(lhs_type_id), AgentTypeId::Custom(rhs_type_id)) => {
                    interactions += 1;
                    let rule = self.rules.get(&(lhs_type_id, rhs_type_id));
                    match rule {
                        Some(rule) => {
                            rule(self, &lhs_id, &rhs_id);
                        }
                        None => {
                            let rule = self.rules.get(&(rhs_type_id, lhs_type_id)).context(
                                NoRuleSnafu {
                                    lhs_type_id,
                                    rhs_type_id,
                                },
                            )?;
                            rule(self, &rhs_id, &lhs_id);
                        }
                    }
                }
            }
        }
        Ok((interactions, name_op, ind_op))
    }
}
