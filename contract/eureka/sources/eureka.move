module eureka::eureka {

    use std::string::{ String };
    use sui::{ 
        event,
        table::{ Self, Table },
        balance::{ Self, Balance },
        coin::{ Self },
        sui::SUI 
    };

    /// === Errors ===
    const ENotAuthorized: u64 = 1;

    /// === One Time Witness ===
    public struct EUREKA has drop {}

    /// === Structs ===
    
    /// Registry for all printers in the system
    public struct PrinterRegisty has key {
        id: UID,
        printers: Table<ID, address>,
    }

    /// Represents a 3D printer with associated properties and balance
    public struct Printer has key {
        id: UID,
        owner: address,
        alias: String,
        online: bool,
        pool: Balance<SUI>,
    }

    /// Capability object that authorizes printer operations
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

    /// Emitted when a printer's status is updated
    public struct PrinterStatusUpdated has copy, drop {
        printer_id: ID,
        new_status: bool,
    }

    /// === Initialization ===
    
    /// Initializes the module with a registry
    fun init(otw: EUREKA, ctx: &mut TxContext) {
        let registry = PrinterRegisty {
            id: object::new(ctx),
            printers: sui::table::new(ctx),
        };

        transfer::share_object(registry);
        let EUREKA {} = otw;
    }

    /// === Public Entry Functions ===
    
    /// Registers a new printer in the registry
    public entry fun register_printer(
        state: &mut PrinterRegisty,
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
            status: false,
        });
    }

    /// Withdraws accumulated fees from a printer
    public entry fun withdraw_fees(
        cap: &PrinterCap,
        printer: &mut Printer,
        ctx: &mut TxContext,
    ) {
        assert!(cap.printer_id == object::uid_to_inner(&printer.id), ENotAuthorized);
        let amount = balance::value(&printer.pool);
        let coin = coin::from_balance(balance::split(&mut printer.pool, amount), ctx);
        transfer::public_transfer(coin, tx_context::sender(ctx));
    }

    /// === Status Management Functions ===
    
    /// Updates a printer's online status
    public fun update_printer_status(
        cap: &PrinterCap,
        printer: &mut Printer,
        new_status: bool,
    ) {
        assert!(cap.printer_id == object::uid_to_inner(&printer.id), ENotAuthorized);
        printer.online = new_status;
        
        event::emit(PrinterStatusUpdated {
            printer_id: object::uid_to_inner(&printer.id),
            new_status,
        });
    }

    /// === Getter Functions ===
    
    /// Gets the ID of a printer
    public fun get_printer_id(printer: &Printer): ID {
        object::uid_to_inner(&printer.id)
    }

    /// Gets the online status of a printer
    public fun get_printer_status(printer: &Printer): bool {
        printer.online
    }

    /// Gets the owner address of a printer
    public fun get_printer_owner(printer: &Printer): address {
        printer.owner
    }

    /// === Package-only Functions ===
    
    /// Adds fees to the printer's pool
    public(package) fun add_fees(printer: &mut Printer, payment: Balance<SUI>) {
        balance::join(&mut printer.pool, payment);
    }
    
    /// Helper function to add a print job as a dynamic field
    public(package) fun add_print_job_field<K: copy + drop + store, V: key + store>(
        printer: &mut Printer,
        k: K,
        v: V
    ) {
        sui::dynamic_object_field::add(&mut printer.id, k, v);
    }
}


