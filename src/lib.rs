mod helpers;
mod handle_connection;

use crate::helpers::{RSheet};
use crate::handle_connection::handle_connection;
use rsheet_lib::connect::{Connection, Manager};
use std::error::Error;
use std::sync::{Arc, Mutex};

pub fn start_server<M>(mut manager: M) -> Result<(), Box<dyn Error>>
where
    M: Manager + Send + 'static,
{
    // Shared sheet across all connections
    let sheet = Arc::new(Mutex::new(RSheet::new()));
    
    loop {
        match manager.accept_new_connection() {
            Connection::NewConnection { reader, writer } => {
                let sheet_clone = Arc::clone(&sheet);
                let thread_per_connection = std::thread::spawn(move || {
                    // Each connection runs its own loop
                    handle_connection(reader, writer, sheet_clone);
                });

                thread_per_connection.join().unwrap();
            }
            Connection::NoMoreConnections => {
                break;
            }
        }
    }
    
    Ok(())
}

