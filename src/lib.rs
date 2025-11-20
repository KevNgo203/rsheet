mod helpers;

use crate::helpers::{RSheet, construct_cell};
use rsheet_lib::cell_expr::{CellArgument, CellExpr};
use rsheet_lib::cell_value::CellValue;
use rsheet_lib::command::Command;
use rsheet_lib::connect::{
    Connection, Manager, ReadMessageResult, Reader, WriteMessageResult, Writer,
};
use rsheet_lib::replies::Reply;
use std::error::Error;
use log::info;

pub fn start_server<M>(mut manager: M) -> Result<(), Box<dyn Error>>
where
    M: Manager,
{
    // This initiates a single client connection, and reads and writes messages
    // indefinitely.
    let (mut recv, mut send) = match manager.accept_new_connection() {
        Connection::NewConnection { reader, writer } => (reader, writer),
        Connection::NoMoreConnections => {
            // There are no more new connections to accept.
            return Ok(());
        }
    };

    // Initialise a new hashmap to store cell values
    let mut new_sheet = RSheet::new();
    loop {
        info!("Just got message");
        match recv.read_message() {
            ReadMessageResult::Message(msg) => {
                // rsheet_lib already contains a FromStr<Command> (i.e. parse::<Command>)
                // implementation for parsing the get and set commands. This is just a
                // demonstration of how to use msg.parse::<Command>, you may want/have to
                // change this code.

                match msg.parse::<Command>() {
                    Ok(command) => match command {
                        Command::Get { cell_identifier } => {
                            let cell = construct_cell(cell_identifier);
                            let value = if let CellArgument::Value(e) = new_sheet.get(&cell) {
                                e
                            } else {
                                CellValue::Error("Cell does not contain a value".to_string())
                            };
                            let reply = Reply::Value(cell, value);

                            match send.write_message(reply) {
                                WriteMessageResult::Ok => {
                                    // Message successfully sent, continue.
                                }
                                WriteMessageResult::ConnectionClosed => {
                                    // The connection was closed. This is not an error, but
                                    // should terminate this connection.
                                    break;
                                }
                                WriteMessageResult::Err(e) => {
                                    // An unexpected error was encountered.
                                    return Err(Box::new(e));
                                }
                            };
                        }
                        Command::Set {
                            cell_identifier,
                            cell_expr,
                        } => {
                            let cell = construct_cell(cell_identifier);
                            let value = CellExpr::new(&cell_expr).evaluate(&new_sheet.cells).unwrap();
                            new_sheet.set(cell.clone(), CellArgument::Value(value.clone()));
                            continue; 
                        }
                    }
                    Err(_) => {
                        let reply = Reply::Error("Invalid command or Invalid Key Provided".to_string());
                        match send.write_message(reply) {
                            WriteMessageResult::Ok => {
                                // Message successfully sent, continue.
                            }
                            WriteMessageResult::ConnectionClosed => {
                                // The connection was closed. This is not an error, but
                                // should terminate this connection.
                                break;
                            }
                            WriteMessageResult::Err(e) => {
                                // An unexpected error was encountered.
                                return Err(Box::new(e));
                            }
                        };
                    }
                }
            }
            ReadMessageResult::ConnectionClosed => {
                // The connection was closed. This is not an error, but
                // should terminate this connection.
                break;
            }
            ReadMessageResult::Err(e) => {
                // An unexpected error was encountered.
                return Err(Box::new(e));
            }
        }
    }
    Ok(())
}
