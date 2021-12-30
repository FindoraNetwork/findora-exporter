use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_hash() {
        #[derive(Hash)]
        struct Person {
            id: u32,
            name: String,
            phone: u64,
        }

        let person1 = Person {
            id: 5,
            name: "Janet".to_string(),
            phone: 666,
        };
        let person2 = Person {
            id: 5,
            name: "Bob".to_string(),
            phone: 666,
        };

        assert!(calculate_hash(&person1) != calculate_hash(&person2));
    }
}
