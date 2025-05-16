module eureka::eureka {

    use std::string::{ String };
    use sui::{ 
        event,
        vec_set::{ Self, VecSet },
        balance::{ Self, Balance },
        coin::{ Self, Coin },
        sui::SUI,
        clock::{ Self },
        dynamic_object_field as dof,
    };
    use eureka::print_job::{
        archive_print_job,
        mutate_print_job_status,
        mutate_print_job_start_time,
        get_print_job_status,
        get_print_job_fees,
        mutate_print_job_end_time,
        get_print_job_id,
        create_print_job,
        extract_print_job_fees
    };
    use archimeters::sculpt::{
        Sculpt,
        print_sculpt,
        get_sculpt_info
    };

    /// === Errors ===
    const EPrintJobCompleted: u64 = 1;
    const EPrintJobExists: u64 = 2;
    const ENotAuthorized: u64 = 3;

    /// === One Time Witness ===
    public struct EUREKA has drop {}


    /// === Structs ===
    
    /// Registry for all printers in the system
    public struct PrinterRegistry has key {
        id: UID,
        printers: VecSet<ID>,
    }

    // Represents a 3D printer with associated properties and balance
    public struct Printer has key, store {
        id: UID,
        owner: address,
        alias: String,
        online: bool,
        pool: Balance<SUI>,
    }

    public struct PrinterCap has key, store {
        id: UID,
        printer_id: ID,
    }

    /// === Events ===
     
    /// Emitted when a new printer is registered in the system
    public struct PrinterRegistered has copy, drop {
        printer_id: ID,
        owner: address,
        status: bool,
    }

    public struct PrintJobCreated has copy, drop {
        job_id: ID,
        printer_id: ID,
        customer: address,
        alias: String,
        paid_amount: u64,
    }

    public struct PrintJobCompleted has copy, drop {
        job_id: ID,
        printer_id: ID,
        customer: address,
        paid_amount: u64,
    }

    /// Emitted when a printer's status is updated
    public struct PrinterStatusUpdated has copy, drop {
        printer_id: ID,
        new_status: bool,
    }

    /// === Initialization ===
    
    /// Initializes the module with a registry
    fun init(otw: EUREKA, ctx: &mut TxContext) {
        let registry = PrinterRegistry {
            id: object::new(ctx),
            printers: vec_set::empty(),
        };

        transfer::share_object(registry);
        let EUREKA {} = otw;
    }

    /// === Public Entry Functions ===
    
    /// Registers a new printer in the registry
    public entry fun register_printer(
        state: &mut PrinterRegistry,
        alias: String,
        ctx: &mut TxContext,
    ) {
        let sender = tx_context::sender(ctx);
        
        let printer = Printer {
            id: object::new(ctx),
            owner: sender,
            alias,  
            online: false,
            pool: balance::zero(),
        };


        let printer_id = object::uid_to_inner(&printer.id);
        let printer_cap = PrinterCap {
            id: object::new(ctx),
            printer_id,
        };

        transfer::public_transfer(printer_cap, sender);
        transfer::share_object(printer);
        vec_set::insert(&mut state.printers, printer_id);

        event::emit(PrinterRegistered {
            printer_id,
            owner: sender,
            status: false,
        });
    }

    // Creates a new print job and assigns it to a printer (with payment)
    public entry fun create_and_assign_print_job(
        printer: &mut Printer,
        sculpt: &mut Sculpt,
        payment: Coin<SUI>,
        ctx: &mut TxContext,
    ) {
        let payment_value = coin::value(&payment);
        let payment_balance = coin::into_balance(payment);
        
        // use internal function to create print job
        create_and_assign_print_job_internal(
            printer,
            sculpt,
            payment_balance,
            payment_value,
            ctx
        );
    }
    
    // Creates a new print job without payment
    public entry fun create_and_assign_print_job_free(
        printer: &mut Printer,
        sculpt: &mut Sculpt,
        ctx: &mut TxContext,
    ) {

        // use internal function to create print job
        create_and_assign_print_job_internal(
            printer,
            sculpt,
            balance::zero<SUI>(),
            0,
            ctx
        );
    }
    
    // Internal function to create print job (used by both entry functions)
    fun create_and_assign_print_job_internal(
        printer: &mut Printer,
        sculpt: &Sculpt,
        payment_balance: Balance<SUI>,
        payment_value: u64,
        ctx: &mut TxContext,
    ) {
        // check if the printer has a print job
        assert!(!dof::exists_(&printer.id, b"print_job"), EPrintJobExists);

        let printer_id = object::uid_to_inner(&printer.id);
        let (sculpt_id, sculpt_alias, sculpt_structure) = get_sculpt_info(sculpt);
        let job = create_print_job(
            ctx.sender(),
            sculpt_alias,
            sculpt_id,
            sculpt_structure,
            payment_balance,
            printer_id,
            ctx,
        );
        
        // Add job to printer as dynamic field
        add_print_job_field(printer, b"print_job", job);

        event::emit(PrintJobCreated {
            job_id: get_print_job_id_via_printer(printer),
            printer_id: object::uid_to_inner(&printer.id),
            customer: ctx.sender(),
            alias: sculpt_alias,
            paid_amount: payment_value,
        });
    }

    // Starts a print job and updates the printer status
    public entry fun start_print_job(
        printer_cap: &PrinterCap,
        printer: &mut Printer,
        sculpt: &mut Sculpt,
        clock: &clock::Clock,
    ) {
        assert!(printer_cap.printer_id == object::uid_to_inner(&printer.id), ENotAuthorized);

        mutate_print_job_start_time_via_printer(printer, clock);
        print_sculpt(sculpt, clock);
    }

    // Completes a print job and updates the printer status
    public entry fun complete_print_job(
        printer_cap: &PrinterCap,
        printer: &mut Printer,
        sculpt: &mut Sculpt,
        clock: &clock::Clock,
        ctx: &mut TxContext,
    ) {
        // check if the printer cap is valid
        assert!(printer_cap.printer_id == object::uid_to_inner(&printer.id), ENotAuthorized);
        // check if the print job is active (not completed)
        assert!(!get_print_job_status_via_printer(printer), EPrintJobCompleted);
        
        // Update job status
        mutate_print_job_status_via_printer(printer);
        
        // Transfer fees to printer pool
        withdraw_fees_via_printer(printer);
        
        // Record end time
        mutate_print_job_end_time_via_printer(printer, clock);
        
        // Emit completion event
        emit_print_job_completed_event(printer, ctx);

        // Archive print job to sculpt
        archive_print_job(dof::borrow_mut(&mut printer.id, b"print_job"), sculpt, ctx);
    }

    // Withdraws accumulated fees from a printer
    entry fun withdraw_fees(
        printer_cap: &PrinterCap,
        printer: &mut Printer,
        ctx: &mut TxContext,
    ) {
        // check if the printer cap is valid    
        assert!(printer_cap.printer_id == object::uid_to_inner(&printer.id), ENotAuthorized);

        let amount = balance::value(&printer.pool);
        let coin = coin::from_balance(balance::split(&mut printer.pool, amount), ctx);
        transfer::public_transfer(coin, tx_context::sender(ctx));
    }

    /// === Status Management Functions ===
    
    /// Updates a printer's online status
    entry fun update_printer_status(
        printer_cap: &PrinterCap,
        printer: &mut Printer,
    ) {
        // check if the printer cap is valid
        assert!(printer_cap.printer_id == object::uid_to_inner(&printer.id), ENotAuthorized);

        printer.online = !printer.online;
        
        event::emit(PrinterStatusUpdated {
            printer_id: object::uid_to_inner(&printer.id),
            new_status: printer.online,
        });
    }

    /// ==== PrintJob Access Functions ===
    
    /// Helper function to mutate the status of a print job
    fun mutate_print_job_status_via_printer(printer: &mut Printer) {
        mutate_print_job_status(dof::borrow_mut(&mut printer.id, b"print_job"));
    }

    // Helper function to mutate the start time of a print job
    fun mutate_print_job_start_time_via_printer(printer: &mut Printer, clock: &clock::Clock) {
        mutate_print_job_start_time(dof::borrow_mut(&mut printer.id, b"print_job"), clock);
    }

    // Helper function to mutate the end time of a print job
    fun mutate_print_job_end_time_via_printer(printer: &mut Printer, clock: &clock::Clock) {
        mutate_print_job_end_time(dof::borrow_mut(&mut printer.id, b"print_job"), clock);
    }

    // Gets print job status via printer
    fun get_print_job_status_via_printer(printer: &Printer): bool {
        get_print_job_status(dof::borrow(&printer.id, b"print_job"))
    }

    // Transfers print job fees to printer pool
    fun withdraw_fees_via_printer(printer: &mut Printer) {
        let fee = extract_print_job_fees(dof::borrow_mut(&mut printer.id, b"print_job"));
        add_fees(printer, fee);
    }

    /// === Event Functions ===
    
    /// Emits print job completion event
    fun emit_print_job_completed_event(printer: &Printer, ctx: &TxContext) {
        event::emit(PrintJobCompleted {
            job_id: get_print_job_id_via_printer(printer),
            printer_id: object::uid_to_inner(&printer.id),
            customer: ctx.sender(),
            paid_amount: get_print_job_fees_via_printer(printer),
        });
    }

    /// === Helper Functions ===
    
    /// Adds fees to the printer's pool
    public(package) fun add_fees(printer: &mut Printer, payment: Balance<SUI>) {
        balance::join(&mut printer.pool, payment);
    }
    
    // Helper function to add a print job as a dynamic field
    public(package) fun add_print_job_field<K: copy + drop + store, V: key + store>(
        printer: &mut Printer,
        k: K,
        v: V
    ) {
        sui::dynamic_object_field::add(&mut printer.id, k, v);
    }

    /// === Print Job Getter Functions ===
    
    /// Gets fee amount from a print job via printer
    fun get_print_job_fees_via_printer(printer: &Printer): u64 {
        get_print_job_fees(dof::borrow(&printer.id, b"print_job"))
    }

    // Gets print job ID via printer
    fun get_print_job_id_via_printer(printer: &Printer): ID {
        get_print_job_id(dof::borrow(&printer.id, b"print_job"))
    }

    /// === Printer Getter Functions ===
    
    /// Gets the owner address of a printer
    public fun get_printer_owner(printer: &Printer): address {
        printer.owner
    }

    /// Gets the online status of a printer
    public fun get_printer_status(printer: &Printer): bool {
        printer.online
    }


    /// === Test Only Functions ===
    #[test_only]
    public fun test_init_for_testing(ctx: &mut TxContext) {
        let registry = PrinterRegistry {
            id: object::new(ctx),
            printers: vec_set::empty(),
        };
        transfer::share_object(registry);
    }
}


