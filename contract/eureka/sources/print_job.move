module eureka::print_job {

    use sui::sui::SUI;
    use std::string::{ String };
    use sui::{ balance::{ Self, Balance }, clock::{ Self } };
    use archimeters::sculpt::{ Sculpt };

    /// === Structs ===
 
    /// Represents a print job with associated properties and balance
    public struct PrintJob has key, store {
        id: UID,
        sculpt_alias: String,
        sculpt_id: ID,
        sculpt_structure: String,
        customer: address,
        printer_id: ID,
        is_completed: bool,
        paid_amount: Balance<SUI>,
        start_time: Option<u64>,
        end_time: Option<u64>,
    }    

    /// === Mutator Functions ===
    
    /// Creates a new print job
    public(package) fun create_print_job(
        customer: address,
        sculpt_alias: String,
        sculpt_id: ID,
        sculpt_structure: String,
        paid_amount: Balance<SUI>,
        printer_id: ID,
        ctx: &mut TxContext,
    ): PrintJob {
        let job = PrintJob {
            id: object::new(ctx),
            sculpt_alias,
            sculpt_id,
            sculpt_structure,
            customer,
            printer_id,
            is_completed: false,
            paid_amount,
            start_time: option::none(),
            end_time: option::none(),
        };
        job
    }
    
    // Mutates the status of a print job
    public(package) fun mutate_print_job_status(print_job: &mut PrintJob) {
        print_job.is_completed = !print_job.is_completed;
    }

    // Mutates the start time of a print job
    public(package) fun mutate_print_job_start_time(print_job: &mut PrintJob, clock: &clock::Clock) {
        print_job.start_time = option::some(clock::timestamp_ms(clock));
    }

    // Mutates the end time of a print job
    public(package) fun mutate_print_job_end_time(print_job: &mut PrintJob, clock: &clock::Clock) {
        print_job.end_time = option::some(clock::timestamp_ms(clock));
    }   

    // Mutates the fees of a print job
    public(package) fun mutate_print_job_fees(print_job: &mut PrintJob, fee: Balance<SUI>) {
        balance::join(&mut print_job.paid_amount, fee);
    }

    // Extracts the fees from a print job
    public(package) fun extract_print_job_fees(print_job: &mut PrintJob): Balance<SUI> {
        let amount = balance::value(&print_job.paid_amount);
        balance::split(&mut print_job.paid_amount, amount)
    }

    // Helper function to attach a print job to the original sculpt
    public(package) fun archive_print_job(
        print_job: &mut PrintJob,
        sculpt: Sculpt
    ) {
        sui::dynamic_object_field::add(&mut print_job.id, b"copy", sculpt);
    }

    /// === Getter Functions ===
    
    /// Gets the ID of a print job
    public(package) fun get_print_job_id(print_job: &PrintJob): ID {
        object::uid_to_inner(&print_job.id)
    }

    public(package) fun get_print_job_status(print_job: &PrintJob): bool {
        print_job.is_completed
    }

    public(package) fun get_print_job_fees(print_job: &PrintJob): u64 {
        balance::value(&print_job.paid_amount)
    }

    public(package) fun get_print_job_start_time(print_job: &PrintJob): &u64 {
        option::borrow(&print_job.start_time)
    }   

    public(package) fun get_print_job_end_time(print_job: &PrintJob): &u64 {
        option::borrow(&print_job.end_time)
    }

    public(package) fun get_print_job_sculpt_alias(print_job: &PrintJob): String {
        print_job.sculpt_alias
    }

    public(package) fun get_print_job_customer(print_job: &PrintJob): address {
        print_job.customer
    }
} 