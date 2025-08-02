use std::{cmp::Ordering, fmt};


#[derive(Debug)]
pub struct Toy {
    pub amount: u32, 
    pub name: String,
}

impl fmt::Display for Toy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Write whatever custom format you want
        write!(f, "{}x {}\n", self.amount, self.name.trim())
    }
}

impl PartialEq for Toy {
    fn eq(&self, other: &Self) -> bool {
        self.amount == other.amount
    }
}

impl PartialOrd for Toy {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.amount.partial_cmp(&other.amount)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_formatting() {
        let toy = Toy {
            amount: 3,
            name: "Teddy Bear  ".into(),
        };
        assert_eq!(format!("{}", toy), "3x Teddy Bear\n");
    }

    #[test]
    fn test_partial_eq_same_amount() {
        let toy1 = Toy {
            amount: 5,
            name: "Car".into(),
        };
        let toy2 = Toy {
            amount: 5,
            name: "Train".into(),
        };
        assert_eq!(toy1, toy2);
    }

    #[test]
    fn test_partial_eq_different_amount() {
        let toy1 = Toy {
            amount: 4,
            name: "Car".into(),
        };
        let toy2 = Toy {
            amount: 6,
            name: "Car".into(),
        };
        assert_ne!(toy1, toy2);
    }

    #[test]
    fn test_partial_ord_less_than() {
        let toy1 = Toy {
            amount: 1,
            name: "Car".into(),
        };
        let toy2 = Toy {
            amount: 2,
            name: "Truck".into(),
        };
        assert!(toy1 < toy2);
    }

    #[test]
    fn test_partial_ord_greater_than() {
        let toy1 = Toy {
            amount: 10,
            name: "Spaceship".into(),
        };
        let toy2 = Toy {
            amount: 3,
            name: "Drone".into(),
        };
        assert!(toy1 > toy2);
    }

    #[test]
    fn test_partial_ord_equal() {
        let toy1 = Toy {
            amount: 7,
            name: "Ball".into(),
        };
        let toy2 = Toy {
            amount: 7,
            name: "Ball".into(),
        };
        assert!(toy1.partial_cmp(&toy2) == Some(Ordering::Equal));
    }
}
