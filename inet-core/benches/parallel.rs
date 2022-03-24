use criterion::{criterion_group, criterion_main, Criterion};

use color_eyre::eyre::Result;
use inet_core::{Machine, MachineBuilder};

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

fn prepare() -> Machine {
    let mut builder = MachineBuilder::default();

    let add_type_id = builder.new_type();
    let s_type_id = builder.new_type();
    let z_type_id = builder.new_type();

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

    builder.new_rule(add_type_id, s_type_id, Box::new(rule_add_s));
    builder.new_rule(add_type_id, z_type_id, Box::new(rule_add_z));

    let x = builder.new_tag().unwrap();
    let y = builder.new_tag().unwrap();
    let output = builder.new_tag().unwrap();

    insert_number(&mut builder, s_type_id, z_type_id, x, 10000).unwrap();
    insert_number(&mut builder, s_type_id, z_type_id, y, 10000).unwrap();

    builder.new_agent(add_type_id, x, &[output, y]).unwrap();

    builder.into_machine()
}

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("heavy_add", |b| {
        b.iter_batched(
            || prepare(),
            |machine| machine.eval().unwrap(),
            criterion::BatchSize::SmallInput,
        );
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
