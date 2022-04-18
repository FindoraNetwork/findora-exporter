use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

pub fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

/// Returns a difference of 18 - decimal.
/// If the decimal is bigger or equal to 18 will return 0.
pub fn diff_of_decimal_18(decimal: &usize) -> u32 {
    if *decimal >= 18 {
        return 0;
    }
    (18 - *decimal) as u32
}

/// The balance number is like below:
/// 9989580120000000000
/// and it is using the 18th number as it's decimal point
/// 9989580120000000000 = 9.989580120000000000
/// and the max i64 is 9989580120000000000 a 19th number
/// so for filling this huge number into i64 we div by 12.
/// the real balances needs to div by 6 again
pub fn toi64_div_10pow12(balance: u128, diff_decimal: u32) -> i64 {
    balance
        .wrapping_mul(10u128.pow(diff_decimal))
        .wrapping_div(10u128.pow(12)) as i64
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

    #[test]
    fn test_diff_of_decimal_18() {
        assert_eq!(15, diff_of_decimal_18(&3));
        assert_eq!(8, diff_of_decimal_18(&10));
        assert_eq!(7, diff_of_decimal_18(&11));
        assert_eq!(0, diff_of_decimal_18(&18));
        assert_eq!(0, diff_of_decimal_18(&20));
    }

    #[test]
    fn test_toi64_div_10pow12() {
        assert_eq!(9989580, toi64_div_10pow12(9989580120000000000, 0));
        assert_eq!(538800000, toi64_div_10pow12(538800000, 9));
    }
}
