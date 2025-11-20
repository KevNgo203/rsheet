use std::collections::HashMap;
use rsheet_lib::command::CellIdentifier;
use rsheet_lib::{cell_expr::CellArgument, cell_value::CellValue};
use rsheet_lib::cells::column_number_to_name;

pub struct RSheet {
    pub cells: HashMap<String, CellArgument>,
}

impl RSheet {
    pub fn new() -> Self {
        RSheet {
            cells: HashMap::new(),
        }
    }

    pub fn get(&self, id: &String) -> CellArgument {
        self.cells.get(id).cloned().unwrap_or(CellArgument::Value(CellValue::None))
    }

    pub fn set(&mut self, id: String , value: CellArgument) {
        self.cells.insert(id, value);
    }
}

pub fn construct_cell(cell_identifier: CellIdentifier) -> String {
    let cell_character = column_number_to_name(cell_identifier.col);
    let cell_number = (cell_identifier.row + 1).to_string();
    format!("{}{}", cell_character, cell_number)
}