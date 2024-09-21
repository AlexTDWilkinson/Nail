use std::any::Any;
use std::io::{Read, Write};
use std::net::TcpListener;

// Adjust start_server to accept Box<dyn Any> for arguments and return Result<Box<dyn Any>, String>
pub fn start_server(args: Box<dyn Any>) -> Result<Box<dyn Any>, String> {
    !!("start_server function has at least ran");
    println!("here are the args {:?}", args);
    if let Ok(address) = args.downcast::<String>() {
        let listener = TcpListener::bind(&**address).map_err(|e| e.to_string())?;
        println!("Server is running on {}", address);

        for stream in listener.incoming() {
            let mut stream = stream.map_err(|e| e.to_string())?;
            let mut buffer = [0; 512];
            stream.read(&mut buffer).map_err(|e| e.to_string())?;

            let response = "HTTP/1.1 200 OK\r\n\r\nHello, world!";
            stream
                .write(response.as_bytes())
                .map_err(|e| e.to_string())?;
            stream.flush().map_err(|e| e.to_string())?;
        }
        Ok(Box::new(())) // Return an empty tuple to indicate success
    } else {
        Err("Invalid argument for start_server".into())
    }
}
