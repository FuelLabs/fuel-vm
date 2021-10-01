pub struct ProofSet {
    storage: Vec<[u8; 32]>,
}

impl ProofSet {
    pub fn new() -> Self {
        Self {
            storage: Vec::new(),
        }
    }

    pub fn push(&mut self, data: &[u8; 32]) {
        self.storage.push(data.clone());
    }

    pub fn get(&self, index: usize) -> Option<[u8; 32]> {
        self.storage.get(index).cloned()
    }

    pub fn len(&self) -> usize {
        self.storage.len()
    }
}

#[cfg(test)]
mod proof_set_test {
    use super::ProofSet;

    #[test]
    fn get_returns_the_byte_array_at_the_given_index() {
        let mut set = ProofSet::new();

        {
            let data = [255u8; 32];
            set.push(&data);
        }

        let data = set.get(0).unwrap();
        let expected_data = [255u8; 32];
        assert_eq!(data, expected_data);
    }

    #[test]
    fn get_returns_none_if_no_data_exists_at_the_given_index() {
        let set = ProofSet::new();

        let data = set.get(0);
        assert!(data.is_none());
    }

    #[test]
    fn len_returns_the_number_of_items_pushed() {
        let mut set = ProofSet::new();

        {
            let data = [255u8; 32];
            set.push(&data);
            set.push(&data);
            set.push(&data);
        }

        let len = set.len();
        let expected_len = 3;
        assert_eq!(len, expected_len);
    }
}
