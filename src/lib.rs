mod helpers;

use crate::helpers::{RSheet, construct_cell, get_cell_value_or_error, build_vector, build_vector_by_row, build_matrix};
use rsheet_lib::cell_expr::{CellArgument, CellExpr};
use rsheet_lib::cell_value::CellValue;
use rsheet_lib::cells::{column_name_to_number};
use rsheet_lib::command::Command;
use rsheet_lib::connect::{
    Connection, Manager, ReadMessageResult, Reader, WriteMessageResult, Writer,
};
use rsheet_lib::replies::Reply;
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

fn handle_connection<R, W>(
    mut recv: R,
    mut send: W,
    sheet: Arc<Mutex<RSheet>>,
)
where
    R: Reader,
    W: Writer,
{
    loop {
        match recv.read_message() {
            ReadMessageResult::Message(msg) => {
            // rsheet_lib already contains a FromStr<Command> (i.e. parse::<Command>)
            // implementation for parsing the get and set commands. This is just a
            // demonstration of how to use msg.parse::<Command>, you may want/have to
            // change this code.
                // let message = msg.trim_end().to_string();
                match msg.parse::<Command>() {
                    Ok(command) => match command {
                        Command::Get { cell_identifier } => {
                            let cell = construct_cell(cell_identifier);
                            let value = {
                                let sheet_guard = sheet.lock().unwrap();
                                get_cell_value_or_error(&*sheet_guard, &cell)
                            };

                            let reply = if let CellValue::Error(err_msg) = &value {
                                if err_msg.contains("Cell relies on another cell with an error") {
                                    Reply::Error(err_msg.clone())
                                } else {
                                    Reply::Value(cell.clone(), value.clone())
                                }
                            } else {
                                Reply::Value(cell.clone(), value.clone())
                            };

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
                                    eprintln!("Write error: {:?}", e);
                                    break;
                                }
                            };
                        }
                        Command::Set { cell_identifier, cell_expr } => {
                            let cell = construct_cell(cell_identifier);
                            let sheet_guard = sheet.lock().unwrap();
                            let cell_expression = CellExpr::new(&cell_expr);
                            let variable_name_vec = cell_expression.find_variable_names();
                            
                            for var_name in variable_name_vec {
                                if var_name.contains("_") {
                                    let new_vec_name = var_name
                                        .split("_")
                                        .map(|s| s.to_string())
                                        .collect::<Vec<String>>();
                                    // Retieve the column of the 2 cells
                                    let first_cell_column = new_vec_name[0].chars().take(1).collect::<String>();
                                    let second_cell_column = new_vec_name[1].chars().take(1).collect::<String>();

                                    // Retrieve the column index of the 2 cells using the provided function
                                    let first_column_index = column_name_to_number(&first_cell_column);
                                    let second_column_index = column_name_to_number(&second_cell_column);

                                    // Retrieve the row number of the 2 cells
                                    let first_cell_row = &new_vec_name[0][1..].parse::<u32>().unwrap();
                                    let second_cell_row = &new_vec_name[1][1..].parse::<u32>().unwrap();


                                    // Check if the 2 cells are in the same row then we construst a vector, otherwise a matrix
                                    if first_cell_row == second_cell_row {
                                        let vec = build_vector(&sheet_guard, first_column_index, second_column_index, *first_cell_row);
                                        sheet_guard.set(var_name, CellArgument::Vector(vec));
                                    } else if first_column_index == second_column_index {
                                        let vec = build_vector_by_row(&sheet_guard, first_column_index, *first_cell_row, *second_cell_row);
                                        sheet_guard.set(var_name, CellArgument::Vector(vec));
                                    } else {
                                        let matrix = build_matrix(&sheet_guard, first_column_index, second_column_index, *first_cell_row, *second_cell_row);
                                        sheet_guard.set(var_name, CellArgument::Matrix(matrix));
                                    }
                                } else if !sheet_guard.contains_key(&var_name) {
                                    sheet_guard.set(var_name.clone(), CellArgument::Value(CellValue::None));
                                }
                            }

                            // short-lived lock of the inner HashMap for evaluation
                            let value = {
                                let cells_guard = sheet_guard.lock_cells(); // MutexGuard<HashMap<...>>
                                match cell_expression.evaluate(&*cells_guard) {
                                    Ok(v) => v,
                                    Err(_) => {
                                        let error_msg = get_cell_value_or_error(&*sheet_guard, &cell_expr);
                                        CellValue::Error(format!(
                                            "Error: A dependent cell contained an error: Cell relies on another cell with an error: {:?}",
                                            error_msg
                                        ))
                                    }
                                }
                            }; 

                            // println!("Evaluated value: {:?}", value);
                            sheet_guard.set(cell.clone(), CellArgument::Value(value.clone()));
                        }
                    },
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
                                eprintln!("Write error: {:?}", e);
                                break;
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
                eprintln!("Write error: {:?}", e);
                break;
            }
        }
    }
}