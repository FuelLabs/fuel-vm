use super::*;
use test_case::test_case;

struct GasChargeInput {
    cgas: u64,
    ggas: u64,
    gas: u64,
}
#[derive(Debug, PartialEq, Eq)]
struct GasChargeOutput {
    cgas: u64,
    ggas: u64,
}
#[test_case(GasChargeInput{cgas: 0, ggas: 0, gas: 0} => Ok(GasChargeOutput{ cgas: 0, ggas: 0}); "zero")]
#[test_case(GasChargeInput{cgas: 0, ggas: 0, gas: 1} => Err(RuntimeError::Recoverable(PanicReason::OutOfGas)); "no gas")]
// Currently panics
// #[test_case(GasChargeInput{cgas: 2, ggas: 0, gas: 1} => Err(RuntimeError::Recoverable(PanicReason::OutOfGas)); "no global gas")]
#[test_case(GasChargeInput{cgas: 0, ggas: 2, gas: 1} => Err(RuntimeError::Recoverable(PanicReason::OutOfGas)); "no call gas")]
#[test_case(GasChargeInput{cgas: 1, ggas: 1, gas: 1} => Ok(GasChargeOutput{ cgas: 0, ggas: 0}); "just enough")]
#[test_case(GasChargeInput{cgas: 10, ggas: 5, gas: 1} => Ok(GasChargeOutput{ cgas: 9, ggas: 4}); "heaps")]

fn test_gas_charge(input: GasChargeInput) -> Result<GasChargeOutput, RuntimeError> {
    let GasChargeInput {
        mut cgas,
        mut ggas,
        gas,
    } = input;
    let mut cgas = RegMut::new(&mut cgas);
    let mut ggas = RegMut::new(&mut ggas);
    gas_charge_inner(cgas.as_mut(), ggas.as_mut(), gas).map(|_| GasChargeOutput {
        cgas: *cgas,
        ggas: *ggas,
    })
}

#[test]
fn test_gas_charges_ggas_on_out_of_gas() {
    let mut cgas = 10;
    let mut ggas = 15;
    let gas = 20;
    let mut cgas = RegMut::new(&mut cgas);
    let mut ggas = RegMut::new(&mut ggas);
    gas_charge_inner(cgas.as_mut(), ggas.as_mut(), gas).expect_err("Gas charge should fail");
    assert_eq!(*ggas, 5);
    assert_eq!(*cgas, 0);
}

struct DepGasChargeInput {
    input: GasChargeInput,
    gas_cost: DependentCost,
}

#[test_case(
    DepGasChargeInput{
        input: GasChargeInput{cgas: 0, ggas: 0, gas: 0},
        gas_cost: DependentCost{base: 0, dep_per_unit: 1}
    } => Ok(GasChargeOutput{ cgas: 0, ggas: 0}); "zero"
)]
#[test_case(
    DepGasChargeInput{
        input: GasChargeInput{cgas: 1, ggas: 1, gas: 0},
        gas_cost: DependentCost{base: 1, dep_per_unit: 1}
    } => Ok(GasChargeOutput{ cgas: 0, ggas: 0}); "just base"
)]
#[test_case(
    DepGasChargeInput{
        input: GasChargeInput{cgas: 1, ggas: 1, gas: 1},
        gas_cost: DependentCost{base: 1, dep_per_unit: 2}
    } => Ok(GasChargeOutput{ cgas: 0, ggas: 0}); "just base with gas"
)]
#[test_case(
    DepGasChargeInput{
        input: GasChargeInput{cgas: 3, ggas: 3, gas: 8},
        gas_cost: DependentCost{base: 1, dep_per_unit: 4}
    } => Ok(GasChargeOutput{ cgas: 0, ggas: 0}); "base with gas and a unit"
)]
#[test_case(
    DepGasChargeInput{
        input: GasChargeInput{cgas: 3, ggas: 3, gas: 5},
        gas_cost: DependentCost{base: 0, dep_per_unit: 4}
    } => Ok(GasChargeOutput{ cgas: 2, ggas: 2}); "base with gas and a unit and left over"
)]
#[test_case(
    DepGasChargeInput{
        input: GasChargeInput{cgas: 0, ggas: 1, gas: 0},
        gas_cost: DependentCost{base: 1, dep_per_unit: 1}
    } => Err(RuntimeError::Recoverable(PanicReason::OutOfGas)); "just base with no cgas"
)]
#[test_case(
    DepGasChargeInput{
        input: GasChargeInput{cgas: 5, ggas: 10, gas: 25},
        gas_cost: DependentCost{base: 1, dep_per_unit: 5}
    } => Err(RuntimeError::Recoverable(PanicReason::OutOfGas)); "unit with not enough cgas"
)]
fn test_dependent_gas_charge(input: DepGasChargeInput) -> Result<GasChargeOutput, RuntimeError> {
    let DepGasChargeInput { input, gas_cost } = input;
    let GasChargeInput {
        mut cgas,
        mut ggas,
        gas,
    } = input;
    let mut cgas = RegMut::new(&mut cgas);
    let mut ggas = RegMut::new(&mut ggas);
    dependent_gas_charge_inner(cgas.as_mut(), ggas.as_mut(), gas_cost, gas).map(|_| GasChargeOutput {
        cgas: *cgas,
        ggas: *ggas,
    })
}
