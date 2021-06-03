use bytes::Bytes;

pub struct ProofSet {
    storage: Vec<Bytes>,
}

impl ProofSet {
    pub fn new() -> Self {
        Self {
            storage: Vec::new(),
        }
    }

    pub fn push(&mut self, data: &[u8]) {
        self.storage.push(Bytes::copy_from_slice(data))
    }

    pub fn get(&self, index: usize) -> Option<&[u8]> {
        let d = self.storage.get(index);
        d.map(|element| &element[..])
    }

    pub fn len(&self) -> usize {
        self.storage.len()
    }
}

#[cfg(test)]
mod proof_set_test {
    use super::*;

    #[test]
    fn get_returns_the_byte_array_at_the_given_index() {
        let mut set = ProofSet::new();

        {
            let data = "Hello World";
            set.push(data.as_bytes());
        }

        let data = set.get(0).unwrap();
        let expected_data = "Hello World".as_bytes();
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
            let data = "Hello World";
            set.push(data.as_bytes());
            set.push(data.as_bytes());
            set.push(data.as_bytes());
        }

        let len = set.len();
        let expected_len = 3;
        assert_eq!(len, expected_len);
    }
}
