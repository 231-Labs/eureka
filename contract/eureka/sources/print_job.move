module eureka::print_job {
    use std::string::{ String };
    use sui::{ event, coin::{ Self, Coin }, sui::SUI };
    use eureka::eureka::{Self, Printer, PrinterCap};

    // === Errors ===
    const EPrinterBusy: u64 = 1;
    const ENotAuthorized: u64 = 2;
    const EPriceNotMet: u64 = 3;

    // === Constants ===
    const PRINTER_STATUS_ONLINE: vector<u8> = b"online";
    // const PRINTER_STATUS_OFFLINE: vector<u8> = b"offline";
    const PRINTER_STATUS_BUSY: vector<u8> = b"busy";
    const PRINT_STATUS_PENDING: vector<u8> = b"pending";
    const PRINT_STATUS_PRINTING: vector<u8> = b"printing";
    const PRINT_STATUS_COMPLETED: vector<u8> = b"completed";
    // const PRINT_STATUS_FAILED: vector<u8> = b"failed";

    // === Structs ===
    public struct PrintJob has key {
        id: UID,
        printer_id: ID,
        customer: address,
        status: String,
        paid_amount: u64,
        start_time: option::Option<u64>,
        end_time: option::Option<u64>,
    }

    // === Events ===
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

    // === Public Functions ===
    public fun create_print_job(
        cap: &PrinterCap,
        printer: &mut Printer,
        payment: Coin<SUI>,
        ctx: &mut TxContext,
    ): ID {
        let sender = tx_context::sender(ctx);
        assert!(std::string::utf8(PRINTER_STATUS_ONLINE) == eureka::get_printer_status(printer), EPrinterBusy);
        
        let paid_amount = coin::value(&payment);
        assert!(paid_amount >= eureka::get_printer_price(printer), EPriceNotMet);

        // Transfer payment to printer's earnings
        eureka::add_earnings(printer, coin::into_balance(payment));

        // Create print job
        let job = PrintJob {
            id: object::new(ctx),
            printer_id: eureka::get_printer_id(printer),
            customer: sender,
            status: std::string::utf8(PRINT_STATUS_PENDING),
            paid_amount,
            start_time: option::none(),
            end_time: option::none(),
        };

        let job_id = object::uid_to_inner(&job.id);

        // Update printer status
        eureka::update_printer_status(cap, printer, PRINTER_STATUS_BUSY);

        event::emit(PrintJobCreated {
            job_id,
            printer_id: eureka::get_printer_id(printer),
            customer: sender,
            paid_amount,
        });

        transfer::share_object(job);
        job_id
    }

    public fun update_print_job(
        cap: &PrinterCap,
        printer: &mut Printer,
        job: &mut PrintJob,
        new_status: vector<u8>,
        ctx: &mut TxContext,
    ) {
        assert!(job.printer_id == eureka::get_printer_id(printer), ENotAuthorized);
        
        let current_epoch = tx_context::epoch(ctx);
        
        if (new_status == PRINT_STATUS_PRINTING) {
            option::fill(&mut job.start_time, current_epoch);
        } else if (new_status == PRINT_STATUS_COMPLETED) {
            option::fill(&mut job.end_time, current_epoch);
            eureka::update_printer_status(cap, printer, PRINTER_STATUS_ONLINE);
        };

        job.status = std::string::utf8(new_status);
        
        event::emit(PrintJobStatusUpdated {
            job_id: object::uid_to_inner(&job.id),
            new_status: std::string::utf8(new_status),
        });
    }
} 