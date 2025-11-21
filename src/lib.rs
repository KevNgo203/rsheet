mod helpers;
mod handle_connection;

use crate::helpers::{RSheet};
use crate::handle_connection::handle_connection;
use rsheet_lib::cell_expr::{CellArgument, CellExpr};
use rsheet_lib::connect::{Connection, Manager};
use std::error::Error;
use std::sync::{Arc, Mutex};

pub fn start_server<M>(mut manager: M) -> Result<(), Box<dyn Error>>
where
    M: Manager + Send + 'static,
{
    // Shared sheet across all connections
    let sheet = Arc::new(Mutex::new(RSheet::new()));

    // Channel for dependency updates
    let (tx, rx) = std::sync::mpsc::channel::<String>();

    // keep the worker handle so we can join later
    let mut worker_thread: Option<std::thread::JoinHandle<()>>;

    // Spawn a worker thread to handle dependency updates
    {
        let sheet_clone = Arc::clone(&sheet);
        worker_thread = Some(std::thread::spawn(move || {
            println!("Dependency worker thread started");
            while let Ok(changed_cell) = rx.recv() {
                let dependents: Vec<String> = {
                    let sheet_guard = sheet_clone.lock().unwrap();
                    sheet_guard.dependencies.get(&changed_cell).cloned().unwrap_or_default()
                };

                for dep in dependents {
                    let expr_arc = {
                        let sheet_guard = sheet_clone.lock().unwrap();
                        sheet_guard.expressions.get(&dep).cloned()
                    };
                    if let Some(expr_arc) = expr_arc {
                        let sheet_guard = sheet_clone.lock().unwrap();
                        let cells_guard = sheet_guard.lock_cells();
                        if let Ok(new_val) = CellExpr::new(&expr_arc).evaluate(&*cells_guard) {
                            sheet_guard.set(dep.clone(), CellArgument::Value(new_val));
                        }
                    }
                }
            }
        }));
    }

    loop {
        match manager.accept_new_connection() {
            Connection::NewConnection { reader, writer } => {
                let sheet_clone = Arc::clone(&sheet);
                let tx_clone = tx.clone();
                let thread_per_connection = std::thread::spawn(move || {
                    // Each connection runs its own loop
                    handle_connection(reader, writer, sheet_clone, tx_clone);
                });
                thread_per_connection.join().unwrap();
            }
            Connection::NoMoreConnections => {
                break;
            }
        }
    }

    // Signal worker to stop by dropping the sender, then join if thread exists
    drop(tx);
    if let Some(handle) = worker_thread.take() {
        // handle.join may return Err if the thread panicked
        handle.join().expect("worker thread panicked");
    }

    Ok(())
}

