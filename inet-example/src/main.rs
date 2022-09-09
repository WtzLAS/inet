use inet_core::{Agent, Context, Machine};

const Z_ID: usize = 0;
const S_ID: usize = 1;
const ADD_ID: usize = 2;

fn new_number(machine: &Machine, mut n: u32) -> *mut Agent {
    let name = machine.new_name();
    let mut last_port = name;
    while n > 0 {
        let aux_port = machine.new_name();
        machine.new_agent_and_eq(S_ID, last_port, &[aux_port]);
        last_port = aux_port;
        n -= 1;
    }
    machine.new_agent_and_eq(Z_ID, last_port, &[]);
    name
}

fn main() {
    let mut machine = Machine::new();

    let rule_add_s = |ctx: Context| {
        let add_agent = ctx.lhs();
        let s_agent = ctx.rhs();

        let n = ctx.machine.new_name();

        ctx.machine
            .new_agent_and_eq(ADD_ID, s_agent.ports()[1], &[add_agent.ports()[1], n]);
        ctx.machine
            .new_agent_and_eq(S_ID, add_agent.ports()[2], &[n]);
    };

    let rule_add_z = |ctx: Context| {
        let add_agent = ctx.lhs();
        let _ = ctx.rhs();

        ctx.machine.new_eq(add_agent.ports()[1], add_agent.ports()[2]);
    };

    machine.new_rule(ADD_ID, S_ID, Box::new(rule_add_s));
    machine.new_rule(ADD_ID, Z_ID, Box::new(rule_add_z));

    let x = new_number(&machine, 1000);
    let y = new_number(&machine, 1000);
    let z = new_number(&machine, 1000);
    let xpy = machine.new_name();
    let o = machine.new_name();

    machine.new_agent_and_eq(ADD_ID, x, &[y, xpy]);
    machine.new_agent_and_eq(ADD_ID, z, &[xpy, o]);

    dbg!(machine.run());

    let o = unsafe { Box::from_raw(o) };
    dbg!(&o);
    o.drop_recursive();
}
