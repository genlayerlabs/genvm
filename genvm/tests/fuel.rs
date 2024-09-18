struct FakeFuel(u64);

impl FakeFuel {
    fn consume_fuel(&mut self, amount: u64) {
        self.0 -= amount.min(self.0);
    }

    fn get_fuel(&self) -> u64 {
        self.0
    }
}

fn fuel_fuzz_impl(consume_baseline: u64) {
    for i in 0u64..1000 {
        let initial_fuel = (1u64 << (30 + i % 5)) + i * 127;
        let fuel = wasmtime::FuelDescriptor::new(initial_fuel);
        let mut fake_fuel = FakeFuel(initial_fuel);

        while fake_fuel.get_fuel() > 0 {
            assert_eq!(fake_fuel.get_fuel(), fuel.get_fuel());
            let to_consume = (1u64 << (consume_baseline + i % 3)) + i * 53;
            fuel.consume_fuel(to_consume);
            fake_fuel.consume_fuel(to_consume);
        }
        assert_eq!(fake_fuel.get_fuel(), fuel.get_fuel());
    }
}

#[test]
fn fuel_fuzz_31() {
    fuel_fuzz_impl(31)
}

#[test]
fn fuel_fuzz_30() {
    fuel_fuzz_impl(30)
}
