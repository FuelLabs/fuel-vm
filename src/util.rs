#[macro_export]
macro_rules! script_with_data_offset {
    ($offset:ident, $script:expr) => {{
        use fuel_types::bytes;
        use fuel_vm::consts::VM_TX_MEMORY;
        let $offset = 0;
        let script_bytes: Vec<u8> = { $script }.into_iter().collect();
        let data_offset = VM_TX_MEMORY + Transaction::script_offset() + bytes::padded_len(script_bytes.as_slice());
        let $offset = data_offset;
        {
            $script
        }
    }};
}
