use color_eyre::eyre::Result;
use inet_core::{AgentTypeId, Machine};
use uuid::Uuid;
use whiteread::Reader;

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
    let n1 = lhs_agent.ports[0];
    let n2 = rhs_agent.ports[0];
    let n3 = lhs_agent.ports[1];
    machine.agents.remove(lhs_id);
    machine.agents.remove(rhs_id);
    let n = machine.new_name().unwrap();
    machine.new_custom(ADD_TYPE_ID, n2, vec![n, n3]).unwrap();
    machine.new_custom(S_TYPE_ID, n1, vec![n]).unwrap();
}

fn add_z(machine: &mut Machine, lhs_id: &Uuid, rhs_id: &Uuid) {
    let lhs_agent = machine.agents.get(lhs_id).unwrap();
    let n1 = lhs_agent.ports[0];
    let n2 = lhs_agent.ports[1];
    machine.agents.remove(lhs_id);
    machine.agents.remove(rhs_id);
    machine.eqs.push_back((n1, n2));
}

fn insert_number(machine: &mut Machine, port: Uuid, mut n: u32) -> Result<()> {
    let mut last_port = port;
    while n > 0 {
        let aux_port = machine.new_name()?;
        machine.new_custom(S_TYPE_ID, last_port, vec![aux_port])?;
        last_port = aux_port;
        n -= 1;
    }
    machine.new_custom(Z_TYPE_ID, last_port, vec![])?;
    Ok(())
}

fn main() -> Result<()> {
    let mut machine = Machine::new();
    machine.rules.insert((ADD_TYPE_ID, S_TYPE_ID), add_s);
    machine.rules.insert((ADD_TYPE_ID, Z_TYPE_ID), add_z);

    let mut input = Reader::from_stdin_naive();

    let (number1, number2): (u32, u32) = input.parse()?;

    let x = machine.new_name()?;
    let y = machine.new_name()?;
    let output = machine.new_name()?;

    insert_number(&mut machine, x, number1)?;
    insert_number(&mut machine, y, number2)?;

    machine.new_custom(ADD_TYPE_ID, x, vec![output, y])?;

    let (interactions, name_op, ind_op) = machine.eval()?;

    println!("({} interactions, {} name operations, {} indirection operations)", interactions, name_op, ind_op);

    // dbg!(ADD_TYPE_ID);
    // dbg!(S_TYPE_ID);
    // dbg!(Z_TYPE_ID);

    // dbg!(&machine.agents);

    let mut cur = output;
    let mut result = 0;

    loop {
        let agent = machine.agents.get(&cur).unwrap();
        match agent.type_id {
            AgentTypeId::Indirection => {}
            AgentTypeId::Custom(S_TYPE_ID) => {
                result += 1;
            }
            AgentTypeId::Custom(Z_TYPE_ID) => {
                break;
            }
            _ => unreachable!(),
        }
        cur = agent.ports[0];
    }

    println!("{}", result);

    Ok(())
}
