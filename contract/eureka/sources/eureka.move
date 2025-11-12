module eureka::eureka {

    use std::string::{ String };
    use sui::{ 
        package,
        display,
        event,
        vec_set::{ Self, VecSet },
        balance::{ Self, Balance },
        coin::{ Self, Coin },
        sui::SUI,
        clock::{ Self },
        dynamic_object_field as dof,
    };
    use eureka::print_job::{
        PrintJob,
        mutate_print_job_status,
        mutate_print_job_start_time,
        get_print_job_status,
        get_print_job_fees,
        get_print_job_start_time,
        mutate_print_job_end_time,
        get_print_job_id,
        get_print_job_printer_id,
        create_print_job,
        extract_print_job_fees,
    };
    use archimeters::sculpt::{
        Sculpt,
        print_sculpt,
        get_sculpt_info,
        get_seal_resource_id,
        add_print_record
    };
    use archimeters::atelier::ATELIER;
    use sui::kiosk::{ Self, Kiosk, KioskOwnerCap };

    /// === Errors ===
    const EPrintJobCompleted: u64 = 1;
    const EPrintJobExists: u64 = 2;
    const ENotAuthorized: u64 = 3;
    const EPrintJobNotStarted: u64 = 4;
    const ENotPrinterOwner: u64 = 5;
    const EInvalidPrinterCap: u64 = 6;
    const EPrintJobNotFound: u64 = 7;
    const EPrinterIdMismatch: u64 = 8;

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

    /// === One Time Witness ===
    public struct EUREKA has drop {}

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

    /// === Initializer ===
    
    /// Initializes the module with a registry
    fun init(otw: EUREKA, ctx: &mut TxContext) {
        let publisher = package::claim(otw, ctx);
        let mut display = display::new<Printer>(&publisher, ctx);
        let registry = PrinterRegistry {
            id: object::new(ctx),
            printers: vec_set::empty(),
        };

        display.add(
            b"name".to_string(),
            b"{alias}".to_string()
        );

        display.add(
            b"description".to_string(),
            b"Eureka!".to_string()
        );

        display.add(
            b"image_url".to_string(),
            b"https://aggregator.walrus-testnet.walrus.space/v1/blobs/fKj0Fgc3I4ty1VxS3NpW4rasQ1FXIXIi9dPhq2uwwHo"
            .to_string()
        );
        
        display.update_version();
        
        transfer::share_object(registry);
        transfer::public_transfer(publisher, ctx.sender());
        transfer::public_transfer(display, ctx.sender());
    }

    /// === Public Entry Functions ===
    
    /// Entry function wrapper that registers a printer and transfers the cap
    entry fun register_printer_and_transfer(
        state: &mut PrinterRegistry,
        alias: String,
        ctx: &mut TxContext,
    ) {
        let printer_cap = register_printer(state, alias, ctx);
        transfer::public_transfer(printer_cap, tx_context::sender(ctx));
    }

    /// Registers a new printer in the registry
    public fun register_printer(
        state: &mut PrinterRegistry,
        alias: String,
        ctx: &mut TxContext,
    ): PrinterCap {
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

        transfer::share_object(printer);
        vec_set::insert(&mut state.printers, printer_id);

        event::emit(PrinterRegistered {
            printer_id,
            owner: sender,
            status: false,
        });

        printer_cap
    }

    // Creates a new print job and assigns it to a printer (with payment)
    public fun create_and_assign_print_job(
        printer: &mut Printer,
        sculpt: &mut Sculpt<ATELIER>,
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
    public fun create_and_assign_print_job_free(
        printer: &mut Printer,
        sculpt: &mut Sculpt<ATELIER>,
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
    
    // Creates a print job from a sculpt in a kiosk (with payment)
    public fun create_print_job_from_kiosk(
        printer: &mut Printer,
        kiosk: &mut Kiosk,
        kiosk_cap: &KioskOwnerCap,
        sculpt_id: ID,
        payment: Coin<SUI>,
        ctx: &mut TxContext,
    ) {
        let payment_value = coin::value(&payment);
        let payment_balance = coin::into_balance(payment);
        
        // Borrow the sculpt from kiosk (returns only the mutable reference)
        let sculpt = kiosk::borrow_mut<Sculpt<ATELIER>>(kiosk, kiosk_cap, sculpt_id);
        
        // Create print job using the borrowed sculpt
        create_and_assign_print_job_internal(
            printer,
            sculpt,
            payment_balance,
            payment_value,
            ctx
        );
        
        // Sculpt is automatically returned when the reference goes out of scope
    }
    
    // Creates a print job from a sculpt in a kiosk (free)
    public fun create_print_job_from_kiosk_free(
        printer: &mut Printer,
        kiosk: &mut Kiosk,
        kiosk_cap: &KioskOwnerCap,
        sculpt_id: ID,
        ctx: &mut TxContext,
    ) {
        // Borrow the sculpt from kiosk (returns only the mutable reference)
        let sculpt = kiosk::borrow_mut<Sculpt<ATELIER>>(kiosk, kiosk_cap, sculpt_id);
        
        // Create print job using the borrowed sculpt
        create_and_assign_print_job_internal(
            printer,
            sculpt,
            balance::zero<SUI>(),
            0,
            ctx
        );
        
        // Sculpt is automatically returned when the reference goes out of scope
    }
    
    // Internal function to create print job (used by both entry functions)
    fun create_and_assign_print_job_internal(
        printer: &mut Printer,
        sculpt: &mut Sculpt<ATELIER>,
        payment_balance: Balance<SUI>,
        payment_value: u64,
        ctx: &mut TxContext,
    ) {
        // check if the printer has a print job
        assert!(!dof::exists_(&printer.id, b"print_job"), EPrintJobExists);

        let printer_id = object::uid_to_inner(&printer.id);
        let (sculpt_id, sculpt_alias, _sculpt_glb, sculpt_structure) = get_sculpt_info(sculpt);
        let seal_resource_id = get_seal_resource_id(sculpt);
        
        // Add printer to sculpt whitelist for decryption access
        archimeters::sculpt::add_printer_to_whitelist(sculpt, printer_id, ctx);
        
        let job = create_print_job(
            ctx.sender(),
            sculpt_alias,
            sculpt_id,
            sculpt_structure,
            seal_resource_id,
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
    public fun start_print_job(
        printer_cap: &PrinterCap,
        printer: &mut Printer,
        sculpt: &mut Sculpt<ATELIER>,
        clock: &clock::Clock,
    ) {
        assert!(printer_cap.printer_id == object::uid_to_inner(&printer.id), ENotAuthorized);
        assert!(!get_print_job_status_via_printer(printer), EPrintJobCompleted);

        mutate_print_job_start_time_via_printer(printer, clock);
        print_sculpt(sculpt, clock);
    }
    
    // Starts a print job from a sculpt in a kiosk
    public fun start_print_job_from_kiosk(
        printer_cap: &PrinterCap,
        printer: &mut Printer,
        kiosk: &mut Kiosk,
        kiosk_cap: &KioskOwnerCap,
        sculpt_id: ID,
        clock: &clock::Clock,
    ) {
        assert!(printer_cap.printer_id == object::uid_to_inner(&printer.id), ENotAuthorized);
        assert!(!get_print_job_status_via_printer(printer), EPrintJobCompleted);
        
        // Borrow the sculpt from kiosk
        let sculpt = kiosk::borrow_mut<Sculpt<ATELIER>>(kiosk, kiosk_cap, sculpt_id);
        
        // Start print job
        mutate_print_job_start_time_via_printer(printer, clock);
        print_sculpt(sculpt, clock);
        
        // Sculpt is automatically returned when the reference goes out of scope
    }

    // Completes a print job and updates the printer status
    public fun complete_print_job(
        printer_cap: &PrinterCap,
        printer: &mut Printer,
        sculpt: &mut Sculpt<ATELIER>,
        clock: &clock::Clock,
        ctx: &mut TxContext,
    ) { 
        // check if the printer cap is valid
        assert!(printer_cap.printer_id == object::uid_to_inner(&printer.id), ENotAuthorized);

        // check if the print job is active (not completed)
        assert!(!get_print_job_status_via_printer(printer), EPrintJobCompleted);

        // check if the print job has been started
        assert!(*get_print_job_start_time_via_printer(printer) > 0, EPrintJobNotStarted);
        
        // Update job status
        mutate_print_job_status_via_printer(printer);
        
        // Transfer fees to printer pool
        withdraw_fees_via_printer(printer);
        
        // Record end time
        mutate_print_job_end_time_via_printer(printer, clock);
        
        // Emit completion event
        emit_print_job_completed_event(printer, ctx);

        // Archive print job to sculpt
        let print_job = dof::remove<vector<u8>, PrintJob>(&mut printer.id, b"print_job");
        add_print_record(sculpt, print_job, clock);
    }
    
    // Completes a print job from a sculpt in a kiosk and removes printer from whitelist
    public fun complete_print_job_from_kiosk(
        printer_cap: &PrinterCap,
        printer: &mut Printer,
        kiosk: &mut Kiosk,
        kiosk_cap: &KioskOwnerCap,
        sculpt_id: ID,
        clock: &clock::Clock,
        ctx: &mut TxContext,
    ) {
        // check if the printer cap is valid
        assert!(printer_cap.printer_id == object::uid_to_inner(&printer.id), ENotAuthorized);

        // check if the print job is active (not completed)
        assert!(!get_print_job_status_via_printer(printer), EPrintJobCompleted);

        // check if the print job has been started
        assert!(*get_print_job_start_time_via_printer(printer) > 0, EPrintJobNotStarted);
        
        // Borrow the sculpt from kiosk
        let sculpt = kiosk::borrow_mut<Sculpt<ATELIER>>(kiosk, kiosk_cap, sculpt_id);
        
        // Update job status
        mutate_print_job_status_via_printer(printer);
        
        // Transfer fees to printer pool
        withdraw_fees_via_printer(printer);
        
        // Record end time
        mutate_print_job_end_time_via_printer(printer, clock);
        
        // Emit completion event
        emit_print_job_completed_event(printer, ctx);

        // Archive print job to sculpt
        let print_job = dof::remove<vector<u8>, PrintJob>(&mut printer.id, b"print_job");
        add_print_record(sculpt, print_job, clock);
        
        // Remove printer from whitelist to revoke decryption access
        let printer_id = object::uid_to_inner(&printer.id);
        archimeters::sculpt::remove_printer_from_whitelist(sculpt, printer_id, ctx);
        
        // Sculpt is automatically returned when the reference goes out of scope
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

    // Gets print job start time via printer
    fun get_print_job_start_time_via_printer(printer: &Printer): &u64 {
        get_print_job_start_time(dof::borrow(&printer.id, b"print_job"))
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
    
    /// Get printer_id from PrinterCap
    public fun get_printer_cap_id(printer_cap: &PrinterCap): ID {
        printer_cap.printer_id
    }
    
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

    /// === Seal Authorization ===
    
    /// Seal approval entry point for Seal SDK integration
    /// Authorization is verified through PrintJob existence:
    /// - Only sculpt owner can create PrintJob
    /// - PrintJob creation adds printer to sculpt whitelist
    /// - Therefore, PrintJob existence = authorized decryption access
    entry fun seal_approve(
        _id: vector<u8>,
        printer: &Printer,
        printer_cap: &PrinterCap,
        ctx: &TxContext,
    ) {
        let printer_id = object::uid_to_inner(&printer.id);
        
        // Verify caller is the printer owner
        assert!(ctx.sender() == printer.owner, ENotPrinterOwner);
        
        // Verify printer_cap matches this printer
        assert!(printer_cap.printer_id == printer_id, EInvalidPrinterCap);
        
        // Verify PrintJob exists (proves authorization from sculpt owner)
        assert!(dof::exists_(&printer.id, b"print_job"), EPrintJobNotFound);
        
        let print_job = dof::borrow<vector<u8>, PrintJob>(&printer.id, b"print_job");
        
        // Verify PrintJob's printer_id matches this printer
        let job_printer_id = get_print_job_printer_id(print_job);
        assert!(job_printer_id == printer_id, EPrinterIdMismatch);
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
    
    #[test_only]
    /// Create a test PrinterCap for unit testing
    public fun create_test_printer_cap(printer_id: ID, ctx: &mut TxContext): PrinterCap {
        PrinterCap {
            id: object::new(ctx),
            printer_id,
        }
    }
}


