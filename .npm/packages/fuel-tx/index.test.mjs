import {expect} from 'chai'
import * as path from 'node:path'
import * as fs from 'node:fs'
import * as tx from './dist/web/index.mjs'

describe('fuel-tx [mjs]', () => {

    it('should ensure URL/fetch patching was succesful', async () => {
        console.log('import.meta', import.meta);

        const mjsContents = fs.readFileSync('./dist/web/index.mjs', 'utf-8')
        const cjsContents = fs.readFileSync('./dist/node/index.cjs', 'utf-8')

        const reg = /(new URL|fetch)\(.+\)/
        expect(mjsContents).to.not.match(reg);
        expect(cjsContents).to.not.match(reg);
    })

    it('should export all types', () => {
        expect(tx.UtxoId).to.be.ok
        expect(tx.TxPointer).to.be.ok
        expect(tx.PredicateParameters).to.be.ok
        expect(tx.Input).to.be.ok
        expect(tx.Output).to.be.ok
        expect(tx.Script).to.be.ok
        expect(tx.Create).to.be.ok
        expect(tx.Mint).to.be.ok
        expect(tx.Transaction).to.be.ok
        expect(tx.Policies).to.be.ok
    })

    it('should serialize and deserialize UtxoId correctly', () => {
        let utxo_id = new tx.UtxoId("0x0c0000000000000000000000000000000000000000000000000000000000000b001a");
        let bytes = utxo_id.to_bytes();
        let utxo_id2 = tx.UtxoId.from_bytes(bytes);
        expect(utxo_id.toString()).to.equal(utxo_id2.toString())
    })

    it('should serialize and deserialize TxPointer correctly', () => {
        let utxo_id = new tx.TxPointer("0123456789ab");
        let bytes = utxo_id.to_bytes();
        let utxo_id2 = tx.TxPointer.from_bytes(bytes);
        expect(utxo_id.toString()).to.equal(utxo_id2.toString())
    })


    it('should serialize and deserialize all input variants correctly', () => {
        [
            tx.Input.coin_predicate(
                new tx.UtxoId("0x0c0000000000000000000000000000000000000000000000000000000000000b001a"),
                tx.Address.zeroed(),
                BigInt(1234),
                tx.AssetId.zeroed(),
                new tx.TxPointer("0123456789ab"),
                BigInt(9012),
                [1, 2, 3, 4],
                [5, 6, 7, 8],
            ),
            tx.Input.coin_signed(
                new tx.UtxoId("0x0c0000000000000000000000000000000000000000000000000000000000000b001a"),
                tx.Address.zeroed(),
                BigInt(1234),
                tx.AssetId.zeroed(),
                new tx.TxPointer("0123456789ab"),
                2,
            ),
            tx.Input.contract(
                new tx.UtxoId("0x0c0000000000000000000000000000000000000000000000000000000000000b001a"),
                tx.Bytes32.zeroed(),
                tx.Bytes32.zeroed(),
                new tx.TxPointer("0123456789ab"),
                tx.ContractId.zeroed(),
            ),
            tx.Input.message_coin_signed(
                tx.Address.zeroed(),
                tx.Address.zeroed(),
                BigInt(1234),
                tx.Nonce.zeroed(),
                2,
            ),
            tx.Input.message_coin_predicate(
                tx.Address.zeroed(),
                tx.Address.zeroed(),
                BigInt(1234),
                tx.Nonce.zeroed(),
                BigInt(1234),
                [1, 2, 3, 4],
                [5, 6, 7, 8],
            ),
            tx.Input.message_data_signed(
                tx.Address.zeroed(),
                tx.Address.zeroed(),
                BigInt(1234),
                tx.Nonce.zeroed(),
                2,
                [1, 2, 3, 4],
            ),
            tx.Input.message_data_predicate(
                tx.Address.zeroed(),
                tx.Address.zeroed(),
                BigInt(1234),
                tx.Nonce.zeroed(),
                BigInt(1234),
                [0, 1, 2, 3],
                [1, 2, 3, 4],
                [5, 6, 7, 8],
            ),
        ].forEach(input => {
            let bytes = input.to_bytes();
            let input2 = tx.Input.from_bytes(bytes);
            expect(input.toString()).to.equal(input2.toString())
        })
    })


    it('should serialize and deserialize all output variants correctly', () => {
        [
            tx.Output.coin(
                tx.Address.zeroed(),
                BigInt(1234),
                tx.AssetId.zeroed(),
            ),
            tx.Output.contract(
                2,
                tx.Bytes32.zeroed(),
                tx.Bytes32.zeroed(),
            ),
            tx.Output.change(
                tx.Address.zeroed(),
                BigInt(1234),
                tx.AssetId.zeroed(),
            ),
            tx.Output.variable(
                tx.Address.zeroed(),
                BigInt(1234),
                tx.AssetId.zeroed(),
            ),
            tx.Output.contract_created(
                tx.ContractId.zeroed(),
                tx.Bytes32.zeroed(),
            ),
        ].forEach(output => {
            let bytes = output.to_bytes();
            let output2 = tx.Output.from_bytes(bytes);
            expect(output.toString()).to.equal(output2.toString())
        })
    })


    it('should serialize and deserialize all transaction variants correctly', () => {
        [
            [tx.Script, tx.Transaction.script(
                1234n,
                [1, 2, 3, 4],
                [5, 6, 7, 8],
                new tx.Policies(),
                [],
                [],
                [],
            )],
            [tx.Create, tx.Transaction.create(
                1,
                new tx.Policies(),
                tx.Salt.zeroed(),
                [],
                [],
                [],
                [],
            )],
            [tx.Mint, tx.Transaction.mint(
                new tx.TxPointer("0123456789ab"),
                new tx.InputContract(
                    new tx.UtxoId("0xc49d65de61cf04588a764b557d25cc6c6b4bc0d7429227e2a21e61c213b3a3e2:18ab"),
                    tx.Bytes32.zeroed(),
                    tx.Bytes32.zeroed(),
                    new tx.TxPointer("0123456789ab"),
                    tx.ContractId.zeroed(),
                ),
                new tx.OutputContract(
                    3,
                    tx.Bytes32.zeroed(),
                    tx.Bytes32.zeroed(),
                ),
                1234n,
                tx.AssetId.zeroed(),
                1234n,
            )],
        ].forEach(([tx_variant_type, tx_variant]) => {
            let bytes = tx_variant.to_bytes();
            let tx_variant2 = tx_variant_type.from_bytes(bytes);
            expect(tx_variant.toString()).to.equal(tx_variant2.toString())

            let wrapped_tx = tx_variant.as_tx();
            let tx_bytes = wrapped_tx.to_bytes();
            let wrapped_tx2 = tx.Transaction.from_bytes(tx_bytes);
            expect(wrapped_tx.toString()).to.equal(wrapped_tx2.toString())
        })
    })

    // Hex string to byte string conversion.
    const hexToBytes = hex => {
        if (hex.length % 2 != 0) {
            throw new Error("Needs full bytes");
        }
        const lookup = "0123456789abcdef";
        let result = new Uint8Array(hex.length / 2);
        for (let i = 0; i < result.length; i += 1) {
            let high = lookup.indexOf(hex[i * 2]);
            let low = lookup.indexOf(hex[i * 2 + 1]);
            if (high === -1 || low === -1) {
                throw new Error("Invalid hex char");
            }
            result[i] = (high << 4) | low;
        }
        return result;
    }

    it('should validate input correctly', () => {
        let input = tx.Input.coin_signed(
            new tx.UtxoId("0xc49d65de61cf04588a764b557d25cc6c6b4bc0d7429227e2a21e61c213b3a3e2:18ab"),
            tx.Address.from_bytes(hexToBytes("f1e92c42b90934aa6372e30bc568a326f6e66a1a0288595e6e3fbd392a4f3e6e")),
            10599410012256088338n,
            tx.AssetId.from_bytes(hexToBytes("2cafad611543e0265d89f1c2b60d9ebf5d56ad7e23d9827d6b522fd4d6e44bc3")),
            new tx.TxPointer("000000000000"),
            0,
            new tx.BlockHeight(0),
        );

        tx.check_input(input, 0, tx.Bytes32.from_bytes(hexToBytes("108eae4147d2c1c86ef4c2ab7c9fe94126645c8d8737495a0574ef1518ae74d8")), [], [{data: hexToBytes("7ce4de2225f041b7f9fec727343a501d99e5b7b58d33f3d4a2cf218d3489959bdec24d13770b5d3bb084b4dac3474f95153e6ecc98f6f0f8ca37a2897b9562ee")}], new tx.PredicateParameters(10000n, 10000n, 10000n, 10000n));
    })

    it('should validate output correctly', () => {
        let output = tx.Output.change(
            tx.Address.zeroed(),
            1234n,
            tx.AssetId.zeroed(),
        );

        tx.check_output(output, 0, []);
    })

    it('should be able to deserialize snapshots', () => {
        const snapshots = '../../../fuel-tx/src/transaction/types/input/snapshots';
        fs.readdirSync(snapshots).filter(fn => fn.endsWith('_canonical.snap')).forEach(file => {
            fs.readFile(path.join(snapshots, file), 'utf8', (err, data) => {
                expect(err).to.be.null;
                let dataBytes = hexToBytes(data.split('---\n').at(-1).trim());
                let inTx = tx.Transaction.from_bytes(dataBytes);
                let serialized = inTx.to_bytes();
                expect(serialized.toString()).to.eq(dataBytes.toString());
            })
        })
    })
})
