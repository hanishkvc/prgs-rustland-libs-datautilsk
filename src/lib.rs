//!
//! DataUtils - a assorted set of helper entities and logics wrt basic data types.
//!
//! HanishKVC, 2022
//!

pub mod variant;
pub mod integer;
pub mod hex;
pub mod sigpro;
pub mod testlib;


#[cfg(test)]
mod tests {
    use crate::testlib;


    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }

    #[test]
    fn test_variant() {
        testlib::test_variant();
    }

    #[test]
    fn test_bufhex() {
        testlib::test_bufhex();
    }

    #[test]
    fn test_vecavg() {
        testlib::test_vecavg();
    }

}
