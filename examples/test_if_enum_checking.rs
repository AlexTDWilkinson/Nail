use tokio;
use Nail::std_lib;

#[tokio::main]
async fn main() {
    #[derive(Debug, PartialEq)]
    enum Status {
        Active,
        Inactive,
        Pending,
        Suspended,
    }
    let current_status: Status =     Status::Active;
    if current_status ==     Status::Active {
        println!("{}", "Active".string_from());
    }
    else if current_status ==     Status::Inactive {
        println!("{}", "Inactive".string_from());
    }
    else if current_status ==     Status::Pending {
        println!("{}", "Pending".string_from());
    }
    else if current_status ==     Status::Suspended {
        println!("{}", "Suspended".string_from());
    }
    if current_status ==     Status::Active {
        println!("{}", "Active".string_from());
    }
    else if current_status ==     Status::Inactive {
        println!("{}", "Inactive".string_from());
    }
    if current_status ==     Status::Active {
        println!("{}", "Active".string_from());
    }
    else if current_status ==     Status::Inactive {
        println!("{}", "Inactive".string_from());
    }
    else {
        println!("{}", "Other status".string_from());
    }
    println!("{}", "Enum checking test complete".string_from());
}
