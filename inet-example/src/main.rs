use std::{sync::atomic::Ordering, time::Instant};

use color_eyre::eyre::Result;
use inet_core::{Agent, Machine, MachineBuilder};
use mimalloc::MiMalloc;
use whiteread::Reader;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn insert_number(
    machine: &mut MachineBuilder,
    s_type_id: usize,
    z_type_id: usize,
    port: usize,
    mut n: u32,
) -> Result<()> {
    let mut last_port = port;
    while n > 0 {
        let aux_port = machine.new_tag()?;
        machine.new_agent(s_type_id, last_port, &[aux_port])?;
        last_port = aux_port;
        n -= 1;
    }
    machine.new_agent(z_type_id, last_port, &[])?;
    Ok(())
}

fn main() -> Result<()> {
    let mut machine = MachineBuilder::default();

    let add_type_id = machine.new_type();
    let s_type_id = machine.new_type();
    let z_type_id = machine.new_type();

    let rule_add_s = move |machine: &Machine, lhs_ports: &[usize], rhs_ports: &[usize]| {
        let n = machine.new_tag().unwrap();
        machine
            .new_agent(add_type_id, rhs_ports[0], &[n, lhs_ports[1]])
            .unwrap();
        machine.new_agent(s_type_id, lhs_ports[0], &[n]).unwrap();
    };

    let rule_add_z = |machine: &Machine, lhs_ports: &[usize], _rhs_ports: &[usize]| {
        machine.new_eq(lhs_ports[0], lhs_ports[1]);
    };

    machine.new_rule(add_type_id, s_type_id, Box::new(rule_add_s));
    machine.new_rule(add_type_id, z_type_id, Box::new(rule_add_z));

    let mut input = Reader::from_stdin_naive();

    let (number1, number2): (u32, u32) = input.parse()?;

    let x = machine.new_tag()?;
    let y = machine.new_tag()?;
    let output = machine.new_tag()?;

    insert_number(&mut machine, s_type_id, z_type_id, x, number1).unwrap();
    insert_number(&mut machine, s_type_id, z_type_id, y, number2).unwrap();

    machine.new_agent(add_type_id, x, &[output, y]).unwrap();

    let machine = machine.into_machine();

    let start = Instant::now();
    let (interactions, name_op) = machine.eval()?;
    let end = Instant::now();

    println!("| {} seconds", (end - start).as_secs_f64());
    println!(
        "| {} interactions, {} name operations",
        interactions, name_op
    );

    let mut cur = output;
    let mut result = 0;

    loop {
        let agent = machine.get_agent(cur).unwrap();
        match &*agent {
            Agent::Tag(is_ind, target) if is_ind.load(Ordering::Relaxed) => {
                cur = target.load(Ordering::Relaxed);
            }
            Agent::Custom(type_id, ports) if *type_id == s_type_id => {
                result += 1;
                cur = ports[0];
            }
            Agent::Custom(type_id, _) if *type_id == z_type_id => {
                break;
            }
            _ => unreachable!(),
        }
    }

    println!("{}", result);

    Ok(())
}
