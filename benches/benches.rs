#[macro_use]
extern crate bencher;

use msg_store::store::{
    generate_store,
    insert,
    Msg
};

use bencher::Bencher;

fn single_group_inserts(bench: &mut Bencher) {
    let mut store = generate_store();
    bench.iter(|| {
        insert(&mut store, &Msg { priority: 0, byte_size: 10 }).unwrap();
    })
}

fn multi_group_inserts(bench: &mut Bencher) {
    let mut store = generate_store();
    bench.iter(|| {
        insert(&mut store, &Msg { priority: 0, byte_size: 10 }).unwrap();
    })
}

benchmark_group!(benches, single_group_inserts, multi_group_inserts);
benchmark_main!(benches);

