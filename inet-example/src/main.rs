use color_eyre::eyre::Result;
use inet_core::{Agent, AgentTypeId, Machine};
use uuid::Uuid;

const ADD_TYPE_ID: Uuid = Uuid::from_bytes([
    250, 211, 204, 127, 115, 229, 73, 117, 147, 186, 121, 90, 43, 20, 175, 16,
]);
const S_TYPE_ID: Uuid = Uuid::from_bytes([
    97, 125, 248, 135, 132, 230, 70, 92, 140, 32, 66, 213, 194, 177, 70, 34,
]);
const Z_TYPE_ID: Uuid = Uuid::from_bytes([
    216, 242, 220, 207, 4, 15, 73, 7, 184, 188, 246, 152, 27, 96, 167, 122,
]);

fn add_s(machine: &mut Machine, lhs_id: &Uuid, rhs_id: &Uuid) {
    let lhs_agent = machine.agents.get(lhs_id).unwrap();
    let rhs_agent = machine.agents.get(rhs_id).unwrap();
    let n1 = lhs_agent.ports[1];
    let n2 = rhs_agent.ports[1];
    let n3 = lhs_agent.ports[2];
    machine.agents.remove(lhs_id);
    machine.agents.remove(rhs_id);
    let n = machine.new_name().unwrap();
    machine.new_custom(ADD_TYPE_ID, vec![n2, n, n3]).unwrap();
    machine.new_custom(S_TYPE_ID, vec![n1, n]).unwrap();
}

fn add_z(machine: &mut Machine, lhs_id: &Uuid, rhs_id: &Uuid) {
    let lhs_agent = machine.agents.get(lhs_id).unwrap();
    let n1 = lhs_agent.ports[1];
    let n2 = lhs_agent.ports[2];
    machine.agents.remove(lhs_id);
    machine.agents.remove(rhs_id);
    machine.eqs.push_back((n1, n2));
}

fn main() -> Result<()> {
    let mut machine = Machine::new();
    machine.rules.insert((ADD_TYPE_ID, S_TYPE_ID), add_s);
    machine.rules.insert((ADD_TYPE_ID, Z_TYPE_ID), add_z);
    let n1 = machine.new_name()?;
    let n2 = machine.new_name()?;
    let n3 = machine.new_name()?;
    let n4 = machine.new_name()?;
    let o = machine.new_name()?;

    machine.new_custom(S_TYPE_ID, vec![n2, n1])?;
    machine.new_custom(Z_TYPE_ID, vec![n1])?;
    machine.new_custom(S_TYPE_ID, vec![n3, n4])?;
    machine.new_custom(Z_TYPE_ID, vec![n4])?;
    machine.new_custom(ADD_TYPE_ID, vec![n2, o, n3])?;

    dbg!(ADD_TYPE_ID);
    dbg!(S_TYPE_ID);
    dbg!(Z_TYPE_ID);

    // dbg!(&machine.agents);
    machine.eval()?;
    dbg!(machine.agents);
    dbg!(o);
    Ok(())
}
