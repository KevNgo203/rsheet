use std::collections::HashMap;
use rsheet_lib::command::CellIdentifier;
use rsheet_lib::{cell_expr::CellArgument, cell_value::CellValue};
use rsheet_lib::cells::column_number_to_name;
use std::sync::{Arc, Mutex, MutexGuard};

pub struct RSheet {
    pub cells: Arc<Mutex<HashMap<String, CellArgument>>>,
    pub dependencies: HashMap<String, Vec<String>>, // maps cell -> dependents
    pub expressions: HashMap<String, String>, // maps cell -> its expression
}

impl RSheet {
    pub fn new() -> Self {
        RSheet {
            cells: Arc::new(Mutex::new(HashMap::new())),
            dependencies: HashMap::new(),
            expressions: HashMap::new(),
        }
    }

    pub fn get(&self, id: &String) -> CellArgument {
        let cells = self.cells.lock().unwrap();
        cells.get(id).cloned().unwrap_or(CellArgument::Value(CellValue::None))
    }

    pub fn set(&self, id: String , value: CellArgument) {
        let mut cells = self.cells.lock().unwrap();
        cells.insert(id, value);
    }

    pub fn contains_key(&self, id: &str) -> bool {
        let cells = self.cells.lock().unwrap();
        cells.contains_key(id)
    }

    /// Lock and return a guard to the inner HashMap so callers can borrow `&HashMap<...>`
    /// (e.g. for `CellExpr::evaluate(&cells)`). Caller must drop the guard before calling
    /// methods that re-lock the same inner mutex (like `set`).
    pub fn lock_cells(&self) -> MutexGuard<'_, HashMap<String, CellArgument>> {
        self.cells.lock().unwrap()
    }
}

pub fn construct_cell(cell_identifier: CellIdentifier) -> String {
    let cell_character = column_number_to_name(cell_identifier.col);
    let cell_number = (cell_identifier.row + 1).to_string();
    format!("{}{}", cell_character, cell_number)
}

pub fn get_cell_value_or_error(sheet: &RSheet, cell_name: &str) -> CellValue {
    match sheet.get(&cell_name.to_string()) {
        CellArgument::Value(e) => e,
        _ => CellValue::Error("Cell does not contain a value".to_string()),
    }
}

pub fn build_vector(sheet: &RSheet, col_start: u32, col_end: u32, row: u32) -> Vec<CellValue> {
    (col_start..=col_end)
        .map(|i| {
            let cell_name = format!("{}{}", column_number_to_name(i), row);
            get_cell_value_or_error(sheet, &cell_name)
        })
        .collect()
}

pub fn build_vector_by_row(sheet: &RSheet, col: u32, row_start: u32, row_end: u32) -> Vec<CellValue> {
    (row_start..=row_end)
        .map(|i| {
            let cell_name = format!("{}{}", column_number_to_name(col), i);
            get_cell_value_or_error(sheet, &cell_name)
        })
        .collect()
}

pub fn build_matrix(sheet: &RSheet, col_start: u32, col_end: u32, row_start: u32, row_end: u32) -> Vec<Vec<CellValue>> {
    (row_start..=row_end)
        .map(|i| build_vector(sheet, col_start, col_end, i))
        .collect()
}

