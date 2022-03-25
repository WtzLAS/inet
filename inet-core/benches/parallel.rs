use criterion::{criterion_group, criterion_main, Criterion};

use inet_core::{Context, Machine, MachineBuilder};

fn insert_number(
    machine: &mut MachineBuilder,
    s_type_id: usize,
    z_type_id: usize,
    port: usize,
    mut n: u32,
) {
    let mut last_port = port;
    while n > 0 {
        let aux_port = machine.new_tag();
        machine.new_agent(s_type_id, last_port, &[aux_port]);
        last_port = aux_port;
        n -= 1;
    }
    machine.new_agent(z_type_id, last_port, &[]);
}
fn prepare() -> Machine {
    let mut builder = MachineBuilder::default();

    let add_type_id = builder.new_type();
    let s_type_id = builder.new_type();
    let z_type_id = builder.new_type();

    let rule_add_s = move |context: Context| {
        let n = context.machine.new_tag();
        context.machine.new_agent(
            add_type_id,
            context.rhs_ports[0],
            &[n, context.lhs_ports[1]],
        );
        context
            .machine
            .new_agent(s_type_id, context.lhs_ports[0], &[n]);
        context.remove_old_agents();
    };

    let rule_add_z = |context: Context| {
        context
            .machine
            .new_eq(context.lhs_ports[0], context.lhs_ports[1]);
        context.remove_old_agents();
    };

    builder.new_rule(add_type_id, s_type_id, Box::new(rule_add_s));
    builder.new_rule(add_type_id, z_type_id, Box::new(rule_add_z));

    let x = builder.new_tag();
    let y = builder.new_tag();
    let output = builder.new_tag();

    insert_number(&mut builder, s_type_id, z_type_id, x, 10000);
    insert_number(&mut builder, s_type_id, z_type_id, y, 10000);

    builder.new_agent(add_type_id, x, &[output, y]);

    builder.into_machine()
}

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("heavy_add", |b| {
        b.iter_batched(
            prepare,
            |machine| machine.eval(),
            criterion::BatchSize::SmallInput,
        );
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
