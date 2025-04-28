use crate::{
    checked_transaction::{
        Checked,
        IntoChecked,
    },
    error::InterpreterError,
    interpreter::{
        Interpreter,
        InterpreterParams,
    },
    prelude::*,
    storage::MemoryStorage,
};
use fuel_asm::{
    PanicReason,
    op,
};
use fuel_tx::{
    ConsensusParameters,
    GasCosts,
    Input,
    Output,
    Transaction,
    Upgrade,
    field::Outputs,
    policies::Policies,
};
use fuel_types::AssetId;

#[cfg(feature = "alloc")]
use alloc::{
    vec,
    vec::Vec,
};

mod state_transition {
    use super::*;
    use crate::storage::UploadedBytecode;
    use fuel_tx::UpgradePurpose;
    use fuel_types::Bytes32;

    const CURRENT_STATE_TRANSITION_VERSION: u32 = 123;

    fn valid_storage(hash: Bytes32, bytecode: Vec<u8>) -> MemoryStorage {
        let mut storage = MemoryStorage::default();
        storage.set_state_transition_version(CURRENT_STATE_TRANSITION_VERSION);
        storage
            .state_transition_bytecodes_mut()
            .insert(hash, UploadedBytecode::Completed(bytecode));

        storage
    }

    const AMOUNT: u64 = 1000;

    fn valid_transaction(hash: Bytes32) -> Checked<Upgrade> {
        let predicate = vec![op::ret(1)].into_iter().collect::<Vec<u8>>();
        let owner = Input::predicate_owner(&predicate);
        let inputs = vec![Input::coin_predicate(
            Default::default(),
            owner,
            AMOUNT,
            AssetId::BASE,
            Default::default(),
            Default::default(),
            predicate,
            vec![],
        )];
        let outputs = vec![Output::change(owner, 0, AssetId::BASE)];

        let upgrade = Transaction::upgrade(
            UpgradePurpose::StateTransition { root: hash },
            Policies::new().with_max_fee(AMOUNT),
            inputs,
            outputs,
            vec![],
        );

        let mut consensus_params = ConsensusParameters::standard();
        consensus_params.set_privileged_address(owner);

        upgrade
            .into_checked_basic(0.into(), &consensus_params)
            .expect("failed to generate checked tx")
    }

    #[test]
    fn transact_updates_state_transition_version() {
        let state_transition_hash = [1; 32].into();
        let mut client = Interpreter::<_, _, Upgrade>::with_storage(
            MemoryInstance::new(),
            valid_storage(state_transition_hash, vec![]),
            InterpreterParams::default(),
        );

        // Given
        let expected_version = CURRENT_STATE_TRANSITION_VERSION + 1;
        let tx = valid_transaction(state_transition_hash).test_into_ready();
        assert!(
            !client
                .as_mut()
                .state_transition_bytecodes_versions_mut()
                .contains_key(&expected_version)
        );

        // When
        let _ = client.transact(tx).expect("failed to transact");

        // Then
        assert!(
            client
                .as_mut()
                .state_transition_bytecodes_versions_mut()
                .contains_key(&expected_version)
        );
    }

    #[test]
    fn transact_with_zero_gas_price_doesnt_affect_change_output() {
        let state_transition_hash = [1; 32].into();
        let mut client = Interpreter::<_, _, Upgrade>::with_storage(
            MemoryInstance::new(),
            valid_storage(state_transition_hash, vec![]),
            InterpreterParams::default(),
        );

        // Given
        let gas_price = 0;
        client.set_gas_price(gas_price);
        let tx = valid_transaction(state_transition_hash)
            .into_ready(gas_price, &GasCosts::default(), &Default::default(), None)
            .expect("failed to generate ready tx");

        // When
        let state = client.transact(tx).expect("failed to transact");

        // Then
        let Output::Change {
            amount, asset_id, ..
        } = state.tx().outputs()[0]
        else {
            panic!("expected change output");
        };
        assert_eq!(amount, AMOUNT);
        assert_eq!(asset_id, AssetId::BASE);
    }

    #[test]
    fn transact_with_non_zero_gas_price_affects_change_output() {
        let state_transition_hash = [1; 32].into();
        let mut client = Interpreter::<_, _, Upgrade>::with_storage(
            MemoryInstance::new(),
            valid_storage(state_transition_hash, vec![]),
            InterpreterParams::default(),
        );

        // Given
        let gas_price = 1;
        client.set_gas_price(gas_price);
        let tx = valid_transaction(state_transition_hash)
            .into_ready(gas_price, &GasCosts::default(), &Default::default(), None)
            .expect("failed to generate ready tx");

        // When
        let state = client.transact(tx).expect("failed to transact");

        // Then
        let Output::Change {
            amount, asset_id, ..
        } = state.tx().outputs()[0]
        else {
            panic!("expected change output");
        };
        assert_eq!(amount, AMOUNT - 1);
        assert_eq!(asset_id, AssetId::BASE);
    }

    #[test]
    fn transact_fails_for_unknown_root() {
        let known_state_transition_hash = [1; 32].into();
        let mut client = Interpreter::<_, _, Upgrade>::with_storage(
            MemoryInstance::new(),
            valid_storage(known_state_transition_hash, vec![]),
            InterpreterParams::default(),
        );

        // Given
        let unknown_state_transition_hash = [2; 32].into();
        let tx = valid_transaction(unknown_state_transition_hash).test_into_ready();

        // When
        let result = client.transact(tx).map(|_| ());

        // Then
        assert_eq!(
            Err(InterpreterError::Panic(
                PanicReason::UnknownStateTransactionBytecodeRoot
            )),
            result
        );
    }

    #[test]
    fn transact_fails_for_known_uncomplete_root() {
        let known_state_transition_hash = [1; 32].into();
        let tx = valid_transaction(known_state_transition_hash).test_into_ready();

        // Given
        let mut storage = valid_storage(known_state_transition_hash, vec![]);
        storage.state_transition_bytecodes_mut().insert(
            known_state_transition_hash,
            UploadedBytecode::Uncompleted {
                bytecode: vec![],
                uploaded_subsections_number: 0,
            },
        );
        let mut client = Interpreter::<_, _, Upgrade>::with_storage(
            MemoryInstance::new(),
            storage,
            InterpreterParams::default(),
        );

        // When
        let result = client.transact(tx).map(|_| ());

        // Then
        assert_eq!(
            Err(InterpreterError::Panic(
                PanicReason::UnknownStateTransactionBytecodeRoot
            )),
            result
        );
    }

    #[test]
    fn transact_fails_when_try_to_override_state_bytecode() {
        let state_transition_hash = [1; 32].into();
        let mut client = Interpreter::<_, _, Upgrade>::with_storage(
            MemoryInstance::new(),
            valid_storage(state_transition_hash, vec![]),
            InterpreterParams::default(),
        );
        let first_tx = valid_transaction(state_transition_hash).test_into_ready();
        let _ = client
            .transact(first_tx)
            .expect("failed to do first transact");

        // Given
        let second_tx = valid_transaction(state_transition_hash).test_into_ready();

        // When
        let result = client.transact(second_tx).map(|_| ());

        // Then
        assert_eq!(
            Err(InterpreterError::Panic(
                PanicReason::OverridingStateTransactionBytecode
            )),
            result
        );
    }
}

mod consensus_parameters {
    use super::*;

    const CURRENT_CONSENSUS_PARAMETERS_VERSION: u32 = 123;

    fn valid_storage() -> MemoryStorage {
        let mut storage = MemoryStorage::default();
        storage.set_consensus_parameters_version(CURRENT_CONSENSUS_PARAMETERS_VERSION);

        storage
    }

    const AMOUNT: u64 = 1000;

    fn valid_transaction() -> Checked<Upgrade> {
        let predicate = vec![op::ret(1)].into_iter().collect::<Vec<u8>>();
        let owner = Input::predicate_owner(&predicate);
        let inputs = vec![Input::coin_predicate(
            Default::default(),
            owner,
            AMOUNT,
            AssetId::BASE,
            Default::default(),
            Default::default(),
            predicate,
            vec![],
        )];
        let outputs = vec![Output::change(owner, 0, AssetId::BASE)];

        let upgrade = Transaction::upgrade_consensus_parameters(
            &ConsensusParameters::standard(),
            Policies::new().with_max_fee(AMOUNT),
            inputs,
            outputs,
            vec![],
        )
        .expect("failed to generate upgrade tx");

        let mut consensus_params = ConsensusParameters::standard();
        consensus_params.set_privileged_address(owner);

        upgrade
            .into_checked_basic(0.into(), &consensus_params)
            .expect("failed to generate checked tx")
    }

    #[test]
    fn transact_updates_consensus_parameters_version() {
        let mut client = Interpreter::<_, _, Upgrade>::with_storage(
            MemoryInstance::new(),
            valid_storage(),
            InterpreterParams::default(),
        );

        // Given
        let expected_version = CURRENT_CONSENSUS_PARAMETERS_VERSION + 1;
        let tx = valid_transaction().test_into_ready();
        assert!(
            !client
                .as_mut()
                .consensus_parameters_versions_mut()
                .contains_key(&expected_version)
        );

        // When
        let _ = client.transact(tx).expect("failed to transact");

        // Then
        assert!(
            client
                .as_mut()
                .consensus_parameters_versions_mut()
                .contains_key(&expected_version)
        );
    }

    #[test]
    fn transact_with_zero_gas_price_doesnt_affect_change_output() {
        let mut client = Interpreter::<_, _, Upgrade>::with_storage(
            MemoryInstance::new(),
            valid_storage(),
            InterpreterParams::default(),
        );

        // Given
        let gas_price = 0;
        client.set_gas_price(gas_price);
        let tx = valid_transaction()
            .into_ready(gas_price, &GasCosts::default(), &Default::default(), None)
            .expect("failed to generate ready tx");

        // When
        let state = client.transact(tx).expect("failed to transact");

        // Then
        let Output::Change {
            amount, asset_id, ..
        } = state.tx().outputs()[0]
        else {
            panic!("expected change output");
        };
        assert_eq!(amount, AMOUNT);
        assert_eq!(asset_id, AssetId::BASE);
    }

    #[test]
    fn transact_with_non_zero_gas_price_affects_change_output() {
        let mut client = Interpreter::<_, _, Upgrade>::with_storage(
            MemoryInstance::new(),
            valid_storage(),
            InterpreterParams::default(),
        );

        // Given
        let gas_price = 1;
        client.set_gas_price(gas_price);
        let tx = valid_transaction()
            .into_ready(gas_price, &GasCosts::default(), &Default::default(), None)
            .expect("failed to generate ready tx");

        // When
        let state = client.transact(tx).expect("failed to transact");

        // Then
        let Output::Change {
            amount, asset_id, ..
        } = state.tx().outputs()[0]
        else {
            panic!("expected change output");
        };
        assert_eq!(amount, AMOUNT - 1);
        assert_eq!(asset_id, AssetId::BASE);
    }

    #[test]
    fn transact_fails_when_try_to_override_consensus_parameters() {
        let mut client = Interpreter::<_, _, Upgrade>::with_storage(
            MemoryInstance::new(),
            valid_storage(),
            InterpreterParams::default(),
        );
        let first_tx = valid_transaction().test_into_ready();
        let _ = client
            .transact(first_tx)
            .expect("failed to do first transact");

        // Given
        let second_tx = valid_transaction().test_into_ready();

        // When
        let result = client.transact(second_tx).map(|_| ());

        // Then
        assert_eq!(
            Err(InterpreterError::Panic(
                PanicReason::OverridingConsensusParameters
            )),
            result
        );
    }
}
