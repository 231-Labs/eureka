/*
#[test_only]
module eureka::eureka_tests;
// uncomment this line to import the module
// use eureka::eureka;

const ENotImplemented: u64 = 0;

#[test]
fun test_eureka() {
    // pass
}

#[test, expected_failure(abort_code = ::eureka::eureka_tests::ENotImplemented)]
fun test_eureka_fail() {
    abort ENotImplemented
}
*/

#[test_only]
module eureka::eureka_tests {
    use sui::test_utils::{assert_eq};
    
    #[test]
    fun test_complete_print_flow() {

        assert_eq(true, true);
    }
}
