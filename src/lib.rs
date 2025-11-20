mod helpers;

use crate::helpers::{RSheet, construct_cell};
use rsheet_lib::cell_expr::{CellArgument, CellExpr};
use rsheet_lib::cell_value::CellValue;
use rsheet_lib::cells::{column_name_to_number, column_number_to_name};
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
                                    let first_cell_row = &new_vec_name[0][1..].parse::<i32>().unwrap();
                                    let second_cell_row = &new_vec_name[1][1..].parse::<i32>().unwrap();


                                    // Check if the 2 cells are in the same row then we construst a vector, otherwise a matrix
                                    if first_cell_row == second_cell_row {
                                        let mut vec = Vec::new();

                                        for i in first_column_index..=second_column_index {
                                            let cell_name = format!("{}{}", column_number_to_name(i), first_cell_row);
                                            let cell_value = if let CellArgument::Value(e) = new_sheet.get(&cell_name) {
                                                e
                                            } else {
                                                CellValue::Error("Cell does not contain a value".to_string())
                                            };
                                            vec.push(cell_value);
                                        }

                                        new_sheet.set(var_name, CellArgument::Vector(vec));
                                    } else if first_column_index == second_column_index {
                                        let mut vec = Vec::new();

                                        for i in *first_cell_row..=*second_cell_row {
                                            let cell_name = format!("{}{}", column_number_to_name(first_column_index), i);
                                            let cell_value = if let CellArgument::Value(e) = new_sheet.get(&cell_name) {
                                                e
                                            } else {
                                                CellValue::Error("Cell does not contain a value".to_string())
                                            };
                                            vec.push(cell_value);
                                        }
                                        new_sheet.set(var_name, CellArgument::Vector(vec));
                                    } else {
                                        let mut matrix: Vec<Vec<CellValue>> = Vec::new();
                                        for i in *first_cell_row..=*second_cell_row {
                                            let mut vec = Vec::new();
                                            for j in first_column_index..=second_column_index {
                                                let cell_name = format!("{}{}", column_number_to_name(j), i);
                                                let cell_value = if let CellArgument::Value(e) = new_sheet.get(&cell_name) {
                                                    e
                                                } else {
                                                    CellValue::Error("Cell does not contain a value".to_string())
                                                };
                                                vec.push(cell_value);
                                            }
                                            matrix.push(vec);
                                        }
                                        new_sheet.set(var_name, CellArgument::Matrix(matrix));
                                    }
                                }
                            }
                            let value = cell_expression.evaluate(&new_sheet.cells).unwrap();
                            new_sheet.set(cell.clone(), CellArgument::Value(value.clone()));
                            continue; // No reply needed for Set command
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
