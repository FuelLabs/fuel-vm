use super::*;
use crate::error::PanicOrBug;
use test_case::test_case;

struct GasChargeInput {
    cgas: u64,
    ggas: u64,
    dependent_factor: u64,
}
#[derive(Debug, PartialEq, Eq)]
struct GasChargeOutput {
    cgas: u64,
    ggas: u64,
}
#[test_case(GasChargeInput{cgas: 0, ggas: 0, dependent_factor: 0} => Ok(GasChargeOutput{ cgas: 0, ggas: 0}); "zero")]
#[test_case(GasChargeInput{cgas: 0, ggas: 0, dependent_factor: 1} => Err(PanicOrBug::Panic(PanicReason::OutOfGas)); "no gas")]
#[test_case(GasChargeInput{cgas: 2, ggas: 0, dependent_factor: 1} => matches Err(PanicOrBug::Bug(_)); "global gas less than context")]
#[test_case(GasChargeInput{cgas: 0, ggas: 2, dependent_factor: 1} => Err(PanicOrBug::Panic(PanicReason::OutOfGas)); "no call gas")]
#[test_case(GasChargeInput{cgas: 1, ggas: 1, dependent_factor: 1} => Ok(GasChargeOutput{ cgas: 0, ggas: 0}); "just enough")]
#[test_case(GasChargeInput{cgas: 10, ggas: 15, dependent_factor: 1} => Ok(GasChargeOutput{ cgas: 9, ggas: 14}); "heaps")]

fn test_gas_charge(input: GasChargeInput) -> SimpleResult<GasChargeOutput> {
    let GasChargeInput {
        mut cgas,
        mut ggas,
        dependent_factor,
    } = input;
    let mut cgas = RegMut::new(&mut cgas);
    let mut ggas = RegMut::new(&mut ggas);
    gas_charge_inner(cgas.as_mut(), ggas.as_mut(), dependent_factor).map(|_| {
        GasChargeOutput {
            cgas: *cgas,
            ggas: *ggas,
        }
    })
}

#[test]
fn test_gas_charges_ggas_on_out_of_gas() {
    let mut cgas = 10;
    let mut ggas = 15;
    let gas = 20;
    let mut cgas = RegMut::new(&mut cgas);
    let mut ggas = RegMut::new(&mut ggas);
    let _ = gas_charge_inner(cgas.as_mut(), ggas.as_mut(), gas)
        .expect_err("Gas charge should fail");
    assert_eq!(*ggas, 5);
    assert_eq!(*cgas, 0);
}

struct DepGasChargeInput {
    input: GasChargeInput,
    gas_cost: DependentCost,
}

#[test_case(
    DepGasChargeInput{
        input: GasChargeInput{cgas: 0, ggas: 0, dependent_factor: 0},
        gas_cost: DependentCost::from_units_per_gas(0, 1)
    } => Ok(GasChargeOutput{ cgas: 0, ggas: 0}); "zero"
)]
#[test_case(
    DepGasChargeInput{
        input: GasChargeInput{cgas: 1, ggas: 1, dependent_factor: 0},
        gas_cost: DependentCost::from_units_per_gas(1, 1)
    } => Ok(GasChargeOutput{ cgas: 0, ggas: 0}); "just base"
)]
#[test_case(
    DepGasChargeInput{
        input: GasChargeInput{cgas: 1, ggas: 1, dependent_factor: 1},
        gas_cost: DependentCost::from_units_per_gas(1, 2)
    } => Ok(GasChargeOutput{ cgas: 0, ggas: 0}); "just base with gas"
)]
#[test_case(
    DepGasChargeInput{
        input: GasChargeInput{cgas: 3, ggas: 3, dependent_factor: 8},
        gas_cost: DependentCost::from_units_per_gas(1, 4)
    } => Ok(GasChargeOutput{ cgas: 0, ggas: 0}); "base with gas and a unit"
)]
#[test_case(
    DepGasChargeInput{
        input: GasChargeInput{cgas: 3, ggas: 3, dependent_factor: 5},
        gas_cost: DependentCost::from_units_per_gas(0, 4)
    } => Ok(GasChargeOutput{ cgas: 2, ggas: 2}); "base with gas and a unit and left over"
)]
#[test_case(
    DepGasChargeInput{
        input: GasChargeInput{cgas: 0, ggas: 1, dependent_factor: 0},
        gas_cost: DependentCost::from_units_per_gas(1, 1)
    } => Err(PanicOrBug::Panic(PanicReason::OutOfGas)); "just base with no cgas"
)]
#[test_case(
    DepGasChargeInput{
        input: GasChargeInput{cgas: 5, ggas: 10, dependent_factor: 25},
        gas_cost: DependentCost::from_units_per_gas(1, 5)
    } => Err(PanicOrBug::Panic(PanicReason::OutOfGas)); "unit with not enough cgas"
)]
fn test_dependent_gas_charge(input: DepGasChargeInput) -> SimpleResult<GasChargeOutput> {
    let DepGasChargeInput { input, gas_cost } = input;
    let GasChargeInput {
        mut cgas,
        mut ggas,
        dependent_factor,
    } = input;
    let mut cgas = RegMut::new(&mut cgas);
    let mut ggas = RegMut::new(&mut ggas);
    let pc = 0;
    let is = 0;
    let profiler = ProfileGas {
        pc: Reg::new(&pc),
        is: Reg::new(&is),
        current_contract: None,
        profiler: &mut Profiler::default(),
    };

    dependent_gas_charge(
        cgas.as_mut(),
        ggas.as_mut(),
        profiler,
        gas_cost,
        dependent_factor,
    )
    .map(|_| GasChargeOutput {
        cgas: *cgas,
        ggas: *ggas,
    })
}

#[test_case(
    DepGasChargeInput{
        input: GasChargeInput{cgas: 0, ggas: 0, dependent_factor: 0},
        gas_cost: DependentCost::from_units_per_gas(0, 1)
    } => Ok(GasChargeOutput{ cgas: 0, ggas: 0}); "zero"
)]
#[test_case(
    DepGasChargeInput{
        input: GasChargeInput{cgas: 1, ggas: 1, dependent_factor: 0},
        gas_cost: DependentCost::from_units_per_gas(1, 1)
    } => Ok(GasChargeOutput{ cgas: 1, ggas: 1}); "even without base"
)]
#[test_case(
    DepGasChargeInput{
        input: GasChargeInput{cgas: 1, ggas: 1, dependent_factor: 1},
        gas_cost: DependentCost::from_units_per_gas(1, 2)
    } => Ok(GasChargeOutput{ cgas: 1, ggas: 1}); "just base with gas"
)]
#[test_case(
    DepGasChargeInput{
        input: GasChargeInput{cgas: 3, ggas: 3, dependent_factor: 8},
        gas_cost: DependentCost::from_units_per_gas(1, 4)
    } => Ok(GasChargeOutput{ cgas: 1, ggas: 1}); "base with gas and a unit"
)]
#[test_case(
    DepGasChargeInput{
        input: GasChargeInput{cgas: 3, ggas: 3, dependent_factor: 5},
        gas_cost: DependentCost::from_units_per_gas(0, 4)
    } => Ok(GasChargeOutput{ cgas: 2, ggas: 2}); "base with gas and a unit and left over"
)]
#[test_case(
    DepGasChargeInput{
        input: GasChargeInput{cgas: 0, ggas: 1, dependent_factor: 0},
        gas_cost: DependentCost::from_units_per_gas(1, 1)
    } => Ok(GasChargeOutput{ cgas: 0, ggas: 1}); "just base with no cgas"
)]
#[test_case(
    DepGasChargeInput{
        input: GasChargeInput{cgas: 5, ggas: 10, dependent_factor: 25},
        gas_cost: DependentCost::from_units_per_gas(1, 4)
    } => Err(PanicOrBug::Panic(PanicReason::OutOfGas)); "unit with not enough cgas"
)]
fn test_dependent_gas_charge_wihtout_base(
    input: DepGasChargeInput,
) -> SimpleResult<GasChargeOutput> {
    let DepGasChargeInput { input, gas_cost } = input;
    let GasChargeInput {
        mut cgas,
        mut ggas,
        dependent_factor,
    } = input;
    let mut cgas = RegMut::new(&mut cgas);
    let mut ggas = RegMut::new(&mut ggas);
    dependent_gas_charge_without_base(
        cgas.as_mut(),
        ggas.as_mut(),
        gas_cost,
        dependent_factor,
    )
    .map(|_| GasChargeOutput {
        cgas: *cgas,
        ggas: *ggas,
    })
}
