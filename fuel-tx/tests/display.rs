use fuel_tx::*;
use fuel_tx_test_helpers::TransactionFactory;

#[test]
fn to_from_str() {
    TransactionFactory::<_, Script>::from_seed(1295)
        .take(20)
        .for_each(|(tx, _)| {
            let tx: Transaction = tx.into();
            let tx_p = tx.to_json();
            let tx_p = Transaction::from_json(&tx_p).expect("failed to restore tx");

            assert_eq!(tx, tx_p);
        });
    TransactionFactory::<_, Create>::from_seed(1295)
        .take(20)
        .for_each(|(tx, _)| {
            let tx: Transaction = tx.into();
            let tx_p = tx.to_json();
            let tx_p = Transaction::from_json(&tx_p).expect("failed to restore tx");

            assert_eq!(tx, tx_p);
        });
    TransactionFactory::<_, Mint>::from_seed(1295)
        .take(20)
        .for_each(|tx| {
            let tx: Transaction = tx.into();
            let tx_p = tx.to_json();
            let tx_p = Transaction::from_json(&tx_p).expect("failed to restore tx");

            assert_eq!(tx, tx_p);
        });
}
