module eureka::eureka {
    use std::string::{ String };
    use sui::{ event, table::{ Self, Table }, balance::{ Self, Balance }, coin::{ Self, Coin }, sui::SUI };

    // === Errors ===
    const EPrinterBusy: u64 = 1;
    const ENotAuthorized: u64 = 1;
    const EPriceNotMet: u64 = 3;

    // === Constants ===
    const PRINTER_STATUS_ONLINE: vector<u8> = b"online";
    const PRINTER_STATUS_BUSY: vector<u8> = b"busy";
    const PRINTER_STATUS_OFFLINE: vector<u8> = b"offline";

    const PRINT_STATUS_PENDING: vector<u8> = b"pending";
    const PRINT_STATUS_PRINTING: vector<u8> = b"printing";
    const PRINT_STATUS_COMPLETED: vector<u8> = b"completed";
    // const PRINT_STATUS_FAILED: vector<u8> = b"failed";

    // === Structs ===
    public struct PrinterRegisty has key {
        id: UID,
        printers: Table<ID, address>,
    }

    public struct Printer has key {
        id: UID,
        owner: address,
        status: String,
        price: u64,
        earnings: Balance<SUI>,
    }

    public struct PrinterCap has key, store {
        id: UID,
        printer_id: ID,
    }

    public struct PrintJob has key {
        id: UID,
        printer_id: ID,
        customer: address,
        status: String,
        paid_amount: u64,
        start_time: Option<u64>,
        end_time: Option<u64>,
    }

    // === One Time Witness ===

    public struct EUREKA has drop {}

    // === Events ===

    public struct PrinterRegistered has copy, drop {
        printer_id: ID,
        owner: address,
        status: String,
        price: u64,
    }

    public struct PrintJobCreated has copy, drop {
        job_id: ID,
        printer_id: ID,
        customer: address,
        paid_amount: u64,
    }

    public struct PrintJobStatusUpdated has copy, drop {
        job_id: ID,
        new_status: String,
    }

    public struct PrinterStatusUpdated has copy, drop {
        printer_id: ID,
        new_status: String,
    }

    /// == Initializer ==
    
    fun init(otw: EUREKA, ctx: &mut TxContext) {
        let registry = PrinterRegisty {
            id: object::new(ctx),
            printers: sui::table::new(ctx),
        };

        transfer::share_object(registry);
        let EUREKA {} = otw;
    }

    // === Public Functions ===
    #[allow(lint(self_transfer))]
    public entry fun register_printer(
        state: &mut PrinterRegisty,
        price: u64,
        ctx: &mut TxContext,
    ) {
        let sender = tx_context::sender(ctx);
        
        let printer = Printer {
            id: object::new(ctx),
            owner: sender,
            status: std::string::utf8(PRINTER_STATUS_OFFLINE),
            price,
            earnings: balance::zero(),
        };

        let printer_id = object::uid_to_inner(&printer.id);

        let cap = PrinterCap {
            id: object::new(ctx),
            printer_id,
        };

        transfer::share_object(printer);
        transfer::transfer(cap, sender);
        table::add(&mut state.printers, printer_id, sender);

        event::emit(PrinterRegistered {
            printer_id,
            owner: sender,
            status: std::string::utf8(PRINTER_STATUS_OFFLINE),
            price,
        });
    }

    public entry fun create_print_job(
        printer: &mut Printer,
        payment: Coin<SUI>,
        ctx: &mut TxContext,
    ): ID {
        let sender = tx_context::sender(ctx);
        assert!(std::string::utf8(PRINTER_STATUS_ONLINE) == printer.status, EPrinterBusy);
        
        let paid_amount = coin::value(&payment);
        assert!(paid_amount >= printer.price, EPriceNotMet);

        // Transfer payment to printer's earnings
        balance::join(&mut printer.earnings, coin::into_balance(payment));

        // Create print job
        let job = PrintJob {
            id: object::new(ctx),
            printer_id: object::uid_to_inner(&printer.id),
            customer: sender,
            status: std::string::utf8(PRINT_STATUS_PENDING),
            paid_amount,
            start_time: option::none(),
            end_time: option::none(),
        };

        let job_id = object::uid_to_inner(&job.id);

        // Update printer status
        printer.status = std::string::utf8(PRINTER_STATUS_BUSY);

        event::emit(PrintJobCreated {
            job_id,
            printer_id: object::uid_to_inner(&printer.id),
            customer: sender,
            paid_amount,
        });

        event::emit(PrinterStatusUpdated {
            printer_id: object::uid_to_inner(&printer.id),
            new_status: std::string::utf8(PRINTER_STATUS_BUSY),
        });

        transfer::share_object(job);
        job_id
    }

    public entry fun update_print_job(
        _cap: &PrinterCap,
        printer: &mut Printer,
        job: &mut PrintJob,
        new_status: vector<u8>,
        ctx: &mut TxContext,
    ) {
        assert!(job.printer_id == object::uid_to_inner(&printer.id), ENotAuthorized);
        
        let current_epoch = tx_context::epoch(ctx);
        
        if (new_status == PRINT_STATUS_PRINTING) {
            option::fill(&mut job.start_time, current_epoch);
        } else if (new_status == PRINT_STATUS_COMPLETED) {
            option::fill(&mut job.end_time, current_epoch);
            printer.status = std::string::utf8(PRINTER_STATUS_ONLINE);
            
            event::emit(PrinterStatusUpdated {
                printer_id: object::uid_to_inner(&printer.id),
                new_status: std::string::utf8(PRINTER_STATUS_ONLINE),
            });
        };

        job.status = std::string::utf8(new_status);
        
        event::emit(PrintJobStatusUpdated {
            job_id: object::uid_to_inner(&job.id),
            new_status: std::string::utf8(new_status),
        });
    }

    public entry fun withdraw_earnings(
        cap: &PrinterCap,
        printer: &mut Printer,
        ctx: &mut TxContext,
    ) {
        assert!(cap.printer_id == object::uid_to_inner(&printer.id), ENotAuthorized);
        let amount = balance::value(&printer.earnings);
        let coin = coin::from_balance(balance::split(&mut printer.earnings, amount), ctx);
        transfer::public_transfer(coin, tx_context::sender(ctx));
    }

    public entry fun update_printer_status(
        cap: &PrinterCap,
        printer: &mut Printer,
        new_status: vector<u8>,
    ) {
        assert!(cap.printer_id == object::uid_to_inner(&printer.id), ENotAuthorized);
        printer.status = std::string::utf8(new_status);
        
        event::emit(PrinterStatusUpdated {
            printer_id: object::uid_to_inner(&printer.id),
            new_status: std::string::utf8(new_status),
        });
    }

    // === Public View Functions ===
    public fun get_printer_id(printer: &Printer): ID {
        object::uid_to_inner(&printer.id)
    }

    public fun get_printer_status(printer: &Printer): String {
        printer.status
    }

    public fun get_printer_price(printer: &Printer): u64 {
        printer.price
    }

    public fun get_printer_owner(printer: &Printer): address {
        printer.owner
    }

    // === Public Package Functions ===
    public(package) fun add_earnings(printer: &mut Printer, payment: Balance<SUI>) {
        balance::join(&mut printer.earnings, payment);
    }
}


