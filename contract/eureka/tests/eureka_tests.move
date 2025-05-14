#[test_only]
module eureka::eureka_tests{

    use sui::test_scenario as ts;
    use std::string;
    use eureka::eureka::{Self, PrinterRegistry, Printer, test_init_for_testing, register_printer};
    
    const ADMIN: address = @0xAD;
    const PRINTER_OWNER: address = @0xB0B;
    const ENotImplemented: u64 = 0;

    #[test]
    fun test_printer_registration_and_status_update() {
        // setup test scenario
        let mut scenario = ts::begin(ADMIN);

        // initialize
        {
            let ctx = ts::ctx(&mut scenario);
            test_init_for_testing(ctx);
        };

        // verify PrinterRegistry is created and shared
        ts::next_tx(&mut scenario, ADMIN);
        {
            assert!(ts::has_most_recent_shared<PrinterRegistry>(), 0);
        };
        
        // 1. Register printer
        let printer_alias = string::utf8(b"Test Printer");
        ts::next_tx(&mut scenario, PRINTER_OWNER);
        {
            let mut registry = ts::take_shared<PrinterRegistry>(&scenario);
            let ctx = ts::ctx(&mut scenario);
            
            register_printer(&mut registry, printer_alias, ctx);
            
            ts::return_shared(registry);
        };
        
        // 2. Verify registration and initial state
        ts::next_tx(&mut scenario, PRINTER_OWNER);
        {
            // Check if printer was transferred to owner
            assert!(ts::has_most_recent_for_address<Printer>(PRINTER_OWNER), 0);
            
            let mut printer = ts::take_from_address<Printer>(&scenario, PRINTER_OWNER);
            
            // Validate printer properties
            assert!(eureka::get_printer_owner(&printer) == PRINTER_OWNER, 1);
            assert!(!eureka::get_printer_status(&printer), 2); // Default is offline
            
            // 3. First status update: offline -> online
            eureka::update_printer_status(&mut printer);
            
            // Verify status changed to online
            assert!(eureka::get_printer_status(&printer), 3);
            
            ts::return_to_address(PRINTER_OWNER, printer);
        };

        // 4. Second status update: online -> offline
        ts::next_tx(&mut scenario, PRINTER_OWNER);
        {
            let mut printer = ts::take_from_address<Printer>(&scenario, PRINTER_OWNER);
            
            // Verify current status is online
            assert!(eureka::get_printer_status(&printer), 4);
            
            // Update status again
            eureka::update_printer_status(&mut printer);
            
            // Verify status switched back to offline
            assert!(!eureka::get_printer_status(&printer), 5);
            
            ts::return_to_address(PRINTER_OWNER, printer);
        };
        
        ts::end(scenario);
    }

    /* Note: Testing create_and_assign_print_job requires a Sculpt object
     * 
     * Implementation requires either:
     * 1. Test helper functions in archimeters::sculpt module
     * 2. Mock Sculpt structure in test code
     */
    #[test]
    #[expected_failure(abort_code = ::eureka::eureka_tests::ENotImplemented)]
    fun test_create_and_assign_print_job() {
        // Directly abort due to missing Sculpt object implementation
        abort ENotImplemented
    }

    #[test]
    #[expected_failure(abort_code = ::eureka::eureka_tests::ENotImplemented)]
    fun test_eureka_fail() {
        abort ENotImplemented
    }
}