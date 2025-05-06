module eureka::print_job {

    use sui::coin::{ Self, Coin };
    use sui::sui::SUI;
    use sui::{ 
        event, 
        clock, 
        balance::{Self, Balance},
    };
    use eureka::eureka::{ 
        Self,
        Printer,
        PrinterCap,
        update_printer_status,
        add_print_job_field,
        add_fees,
    };

    /// === Errors ===
    const EPrinterBusy: u64 = 1;
    const ENotAuthorized: u64 = 2;

    /// === Structs ===
    public struct PrintJob has key, store {
        id: UID,
        printer_id: ID,
        customer: address,
        is_completed: bool,
        paid_amount: Balance<SUI>,
        start_time: Option<u64>,
        end_time: Option<u64>,
    }

    /// === Events ===
    public struct PrintJobCreated has copy, drop {
        job_id: ID,
        printer_id: ID,
        customer: address,
        paid_amount: u64,
    }

    public struct PrintJobStatusUpdated has copy, drop {
        job_id: ID,
        new_status: bool,
    }

    /// === Job Lifecycle Functions ===

    /// Creates a new print job and assigns it to a printer
    public entry fun create_and_assign_print_job(
        printer: &mut Printer,
        payment: Coin<SUI>,
        ctx: &mut TxContext,
    ) {
        let payment_value = coin::value(&payment);
        let payment_balance = coin::into_balance(payment);
        let (job, job_id) = create_job_object(printer, payment_balance, ctx);

        event::emit(PrintJobCreated {
            job_id,
            printer_id: eureka::get_printer_id(printer),
            customer: ctx.sender(),
            paid_amount: payment_value,
        });
        
        add_print_job_field(printer, job_id, job);
    }

    /// Marks a print job as started and sets printer status to busy
    public entry fun start_print_job(
        cap: &PrinterCap,
        printer: &mut Printer,
        job: &mut PrintJob,
        clock: &clock::Clock,
    ) {
        record_timestamp(&mut job.start_time, clock);
        update_printer_status(cap, printer, false);
    }

    /// Marks a print job as completed and sets printer status to available
    public entry fun complete_print_job(
        cap: &PrinterCap,
        printer: &mut Printer,
        job: &mut PrintJob,
        clock: &clock::Clock,
    ) {
        verify_job_authorization(printer, job);
        assert!(!job.is_completed, EPrinterBusy);

        let amount = balance::value(&job.paid_amount);
        let payment = balance::split(&mut job.paid_amount, amount);
        add_fees(printer, payment);
        
        job.is_completed = true;
        emit_job_status_event(job, job.is_completed);
        update_printer_status(cap, printer, job.is_completed);
        record_timestamp(&mut job.end_time, clock);
    }
    
    /// === Getter Functions ===

    /// Checks if a print job is completed
    public fun is_completed(job: &PrintJob): bool {
        job.is_completed
    }
    
    /// Gets the printer ID associated with a job
    public fun get_printer_id(job: &PrintJob): ID {
        job.printer_id
    }
    
    /// Gets the customer address for a job
    public fun get_customer(job: &PrintJob): address {
        job.customer
    }
    
    /// Gets the amount paid for a job
    public fun get_paid_amount(job: &PrintJob): u64 {
        balance::value(&job.paid_amount)
    }
    
    /// Gets the start time of a job
    public fun get_start_time(job: &PrintJob): Option<u64> {
        *&job.start_time
    }
    
    /// Gets the end time of a job
    public fun get_end_time(job: &PrintJob): Option<u64> {
        *&job.end_time
    }
    
    /// === Internal Helper Functions ===

    /// Creates a new PrintJob object with the specified parameters
    fun create_job_object(
        printer: &Printer, 
        fee: Balance<SUI>, 
        ctx: &mut TxContext
    ): (PrintJob, ID) {
        let sender = tx_context::sender(ctx);
        
        let job = PrintJob {
            id: object::new(ctx),
            printer_id: eureka::get_printer_id(printer),
            customer: sender,
            is_completed: false,
            paid_amount: fee,
            start_time: option::none(),
            end_time: option::none(),
        };
        
        let job_id = object::uid_to_inner(&job.id);
        (job, job_id)
    }
    
    /// Verifies that a job is authorized to be processed by a printer
    fun verify_job_authorization(printer: &Printer, job: &PrintJob) {
        assert!(job.printer_id == eureka::get_printer_id(printer), ENotAuthorized);
    }
    
    /// Emits a job status update event
    fun emit_job_status_event(job: &PrintJob, new_status: bool) {
        event::emit(PrintJobStatusUpdated {
            job_id: object::uid_to_inner(&job.id),
            new_status,
        });
    }

    /// Records a timestamp for a time field if not already set
    fun record_timestamp(time_field: &mut Option<u64>, clock: &clock::Clock) {
        if (option::is_none(time_field)) {
            let now = clock::timestamp_ms(clock);
            option::fill(time_field, now);
        };
    }
} 