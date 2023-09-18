use fuel_vm::profiler::{
    InstructionLocation,
    ProfilingData,
};

#[test]
fn test_profile_serde() {
    let mut data = ProfilingData::default();

    {
        let coverage = data.coverage_mut();
        coverage.set(InstructionLocation::new(None, 1));
        coverage.set(InstructionLocation::new(None, 1));
        coverage.set(InstructionLocation::new(None, 2));
    }

    {
        let gas = data.gas_mut();
        gas.add(InstructionLocation::new(None, 1), 4);
        gas.add(InstructionLocation::new(None, 1), 4);
        gas.add(InstructionLocation::new(None, 2), 2);
    }

    let json = serde_json::to_vec(&data).expect("Serialization failed");
    let _: ProfilingData = serde_json::from_slice(&json).expect("Deserialization failed");
}
