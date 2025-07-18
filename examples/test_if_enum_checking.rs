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
        println!("{}", "Active".to_string());
    }
    else if current_status ==     Status::Inactive {
        println!("{}", "Inactive".to_string());
    }
    else if current_status ==     Status::Pending {
        println!("{}", "Pending".to_string());
    }
    else if current_status ==     Status::Suspended {
        println!("{}", "Suspended".to_string());
    }
    if current_status ==     Status::Active {
        println!("{}", "Active".to_string());
    }
    else if current_status ==     Status::Inactive {
        println!("{}", "Inactive".to_string());
    }
    if current_status ==     Status::Active {
        println!("{}", "Active".to_string());
    }
    else if current_status ==     Status::Inactive {
        println!("{}", "Inactive".to_string());
    }
    else {
        println!("{}", "Other status".to_string());
    }
    println!("{}", "Enum checking test complete".to_string());
}
