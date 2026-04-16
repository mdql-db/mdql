//! PyO3 bindings for MDQL — drop-in replacement for the Python mdql package.
//!
//! Exposes internal modules as functions so Python wrappers can provide
//! a compatible API surface.

use std::collections::HashMap;
use std::path::PathBuf;

use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use mdql_core::model::Value;
use mdql_core::query_parser as qp;

// ── Helpers ──────────────────────────────────────────────────────────────

fn value_to_py(py: Python<'_>, val: &Value) -> PyObject {
    match val {
        Value::Null => py.None(),
        Value::String(s) => s.into_pyobject(py).unwrap().into_any().unbind(),
        Value::Int(n) => n.into_pyobject(py).unwrap().into_any().unbind(),
        Value::Float(f) => f.into_pyobject(py).unwrap().into_any().unbind(),
        Value::Bool(b) => b.into_pyobject(py).unwrap().to_owned().into_any().unbind(),
        Value::Date(d) => {
            use chrono::Datelike;
            let date_mod = py.import("datetime").unwrap();
            let date_cls = date_mod.getattr("date").unwrap();
            date_cls.call1((d.year(), d.month(), d.day())).unwrap().into_pyobject(py).unwrap().into_any().unbind()
        }
        Value::DateTime(dt) => {
            use chrono::{Datelike, Timelike};
            let dt_mod = py.import("datetime").unwrap();
            let dt_cls = dt_mod.getattr("datetime").unwrap();
            dt_cls.call1((dt.year(), dt.month(), dt.day(), dt.hour(), dt.minute(), dt.second())).unwrap().into_pyobject(py).unwrap().into_any().unbind()
        }
        Value::List(items) => {
            let list = PyList::new(py, items).unwrap();
            list.into_pyobject(py).unwrap().into_any().unbind()
        }
    }
}

fn py_to_value(obj: &Bound<'_, pyo3::PyAny>) -> PyResult<Value> {
    if obj.is_none() {
        return Ok(Value::Null);
    }
    if let Ok(b) = obj.extract::<bool>() {
        return Ok(Value::Bool(b));
    }
    if let Ok(n) = obj.extract::<i64>() {
        return Ok(Value::Int(n));
    }
    if let Ok(f) = obj.extract::<f64>() {
        return Ok(Value::Float(f));
    }
    if let Ok(s) = obj.extract::<String>() {
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M:%S") {
            return Ok(Value::DateTime(dt));
        }
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M:%S%.f") {
            return Ok(Value::DateTime(dt));
        }
        if let Ok(d) = chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d") {
            return Ok(Value::Date(d));
        }
        return Ok(Value::String(s));
    }
    if let Ok(list) = obj.downcast::<PyList>() {
        let items: Vec<String> = list
            .iter()
            .filter_map(|item| item.extract::<String>().ok())
            .collect();
        return Ok(Value::List(items));
    }
    Ok(Value::String(obj.str()?.to_string()))
}

fn row_to_dict(py: Python<'_>, row: &HashMap<String, Value>) -> PyResult<PyObject> {
    let dict = PyDict::new(py);
    for (k, v) in row {
        dict.set_item(k, value_to_py(py, v))?;
    }
    Ok(dict.into_pyobject(py)?.into_any().unbind())
}

fn dict_to_map(dict: &Bound<'_, PyDict>) -> PyResult<HashMap<String, Value>> {
    let mut map = HashMap::new();
    for (k, v) in dict.iter() {
        let key: String = k.extract()?;
        let value = py_to_value(&v)?;
        map.insert(key, value);
    }
    Ok(map)
}

fn dict_to_row(dict: &Bound<'_, PyDict>) -> PyResult<HashMap<String, Value>> {
    dict_to_map(dict)
}

fn yaml_value_to_py(py: Python<'_>, val: &serde_yaml::Value) -> PyObject {
    match val {
        serde_yaml::Value::Null => py.None(),
        serde_yaml::Value::Bool(b) => b.into_pyobject(py).unwrap().to_owned().into_any().unbind(),
        serde_yaml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.into_pyobject(py).unwrap().into_any().unbind()
            } else if let Some(f) = n.as_f64() {
                f.into_pyobject(py).unwrap().into_any().unbind()
            } else {
                py.None()
            }
        }
        serde_yaml::Value::String(s) => s.into_pyobject(py).unwrap().into_any().unbind(),
        serde_yaml::Value::Sequence(seq) => {
            let items: Vec<PyObject> = seq.iter().map(|v| yaml_value_to_py(py, v)).collect();
            let list = PyList::new(py, &items).unwrap();
            list.into_pyobject(py).unwrap().into_any().unbind()
        }
        serde_yaml::Value::Mapping(m) => {
            let dict = PyDict::new(py);
            for (k, v) in m {
                if let Some(key) = k.as_str() {
                    dict.set_item(key, yaml_value_to_py(py, v)).unwrap();
                }
            }
            dict.into_pyobject(py).unwrap().into_any().unbind()
        }
        _ => py.None(),
    }
}

fn sqlvalue_to_py(py: Python<'_>, val: &qp::SqlValue) -> PyObject {
    match val {
        qp::SqlValue::Null => py.None(),
        qp::SqlValue::String(s) => s.into_pyobject(py).unwrap().into_any().unbind(),
        qp::SqlValue::Int(n) => n.into_pyobject(py).unwrap().into_any().unbind(),
        qp::SqlValue::Float(f) => f.into_pyobject(py).unwrap().into_any().unbind(),
        qp::SqlValue::List(items) => {
            let list = PyList::new(py, items.iter().map(|v| sqlvalue_to_py(py, v))).unwrap();
            list.into_pyobject(py).unwrap().into_any().unbind()
        }
    }
}

// ── Query AST PyO3 classes ───────────────────────────────────────────────

#[pyclass(name = "Comparison")]
#[derive(Clone)]
struct PyComparison {
    #[pyo3(get)]
    column: String,
    #[pyo3(get)]
    op: String,
    value_inner: Option<qp::SqlValue>,
}

#[pymethods]
impl PyComparison {
    #[getter]
    fn value(&self, py: Python<'_>) -> PyObject {
        match &self.value_inner {
            None => py.None(),
            Some(v) => sqlvalue_to_py(py, v),
        }
    }

    fn __repr__(&self) -> String {
        format!("Comparison({}, {}, {:?})", self.column, self.op, self.value_inner)
    }
}

#[pyclass(name = "BoolOp")]
#[derive(Clone)]
struct PyBoolOp {
    #[pyo3(get)]
    op: String,
    left_inner: qp::WhereClause,
    right_inner: qp::WhereClause,
}

#[pymethods]
impl PyBoolOp {
    #[getter]
    fn left(&self, py: Python<'_>) -> PyObject {
        where_clause_to_py(py, &self.left_inner)
    }

    #[getter]
    fn right(&self, py: Python<'_>) -> PyObject {
        where_clause_to_py(py, &self.right_inner)
    }

    fn __repr__(&self) -> String {
        format!("BoolOp({})", self.op)
    }
}

#[pyclass(name = "OrderSpec")]
#[derive(Clone)]
struct PyOrderSpec {
    #[pyo3(get)]
    column: String,
    #[pyo3(get)]
    descending: bool,
}

#[pymethods]
impl PyOrderSpec {
    #[new]
    #[pyo3(signature = (column, descending=false))]
    fn new(column: String, descending: bool) -> Self {
        PyOrderSpec { column, descending }
    }

    fn __eq__(&self, other: &PyOrderSpec) -> bool {
        self.column == other.column && self.descending == other.descending
    }

    fn __repr__(&self) -> String {
        format!("OrderSpec({}, {})", self.column, self.descending)
    }
}

#[pyclass(name = "JoinInfo")]
#[derive(Clone)]
struct PyJoinInfo {
    #[pyo3(get)]
    table: String,
    #[pyo3(get)]
    alias: Option<String>,
    #[pyo3(get)]
    left_col: String,
    #[pyo3(get)]
    right_col: String,
}

#[pyclass(name = "Query")]
#[derive(Clone)]
struct PyQuery {
    #[pyo3(get)]
    table: String,
    #[pyo3(get)]
    table_alias: Option<String>,
    #[pyo3(get)]
    order_by: Option<Vec<PyOrderSpec>>,
    #[pyo3(get)]
    limit: Option<i64>,
    #[pyo3(get)]
    joins: Vec<PyJoinInfo>,
    columns_inner: qp::ColumnList,
    where_inner: Option<qp::WhereClause>,
}

#[pymethods]
impl PyQuery {
    #[getter]
    fn columns(&self, py: Python<'_>) -> PyObject {
        match &self.columns_inner {
            qp::ColumnList::All => "*".into_pyobject(py).unwrap().into_any().unbind(),
            qp::ColumnList::Named(exprs) => {
                let names: Vec<String> = exprs.iter().map(|e| e.output_name()).collect();
                let list = PyList::new(py, &names).unwrap();
                list.into_pyobject(py).unwrap().into_any().unbind()
            }
        }
    }

    #[getter(where_clause)]
    fn where_clause(&self, py: Python<'_>) -> PyObject {
        match &self.where_inner {
            None => py.None(),
            Some(wc) => where_clause_to_py(py, wc),
        }
    }

    fn __repr__(&self) -> String {
        format!("Query(table={})", self.table)
    }
}

#[pyclass(name = "InsertQuery")]
#[derive(Clone)]
struct PyInsertQuery {
    #[pyo3(get)]
    table: String,
    #[pyo3(get)]
    columns: Vec<String>,
    values_inner: Vec<qp::SqlValue>,
}

#[pymethods]
impl PyInsertQuery {
    #[getter]
    fn values(&self, py: Python<'_>) -> PyObject {
        let list = PyList::new(py, self.values_inner.iter().map(|v| sqlvalue_to_py(py, v))).unwrap();
        list.into_pyobject(py).unwrap().into_any().unbind()
    }
}

#[pyclass(name = "UpdateQuery")]
#[derive(Clone)]
struct PyUpdateQuery {
    #[pyo3(get)]
    table: String,
    assignments_inner: Vec<(String, qp::SqlValue)>,
    where_inner: Option<qp::WhereClause>,
}

#[pymethods]
impl PyUpdateQuery {
    #[getter]
    fn assignments(&self, py: Python<'_>) -> PyObject {
        let list = PyList::new(py, self.assignments_inner.iter().map(|(k, v)| {
            let tuple = pyo3::types::PyTuple::new(py, [
                k.into_pyobject(py).unwrap().into_any().unbind(),
                sqlvalue_to_py(py, v),
            ]).unwrap();
            tuple.into_pyobject(py).unwrap().into_any().unbind()
        })).unwrap();
        list.into_pyobject(py).unwrap().into_any().unbind()
    }

    #[getter(where_clause)]
    fn where_clause(&self, py: Python<'_>) -> PyObject {
        match &self.where_inner {
            None => py.None(),
            Some(wc) => where_clause_to_py(py, wc),
        }
    }
}

#[pyclass(name = "DeleteQuery")]
#[derive(Clone)]
struct PyDeleteQuery {
    #[pyo3(get)]
    table: String,
    where_inner: Option<qp::WhereClause>,
}

#[pymethods]
impl PyDeleteQuery {
    #[getter(where_clause)]
    fn where_clause(&self, py: Python<'_>) -> PyObject {
        match &self.where_inner {
            None => py.None(),
            Some(wc) => where_clause_to_py(py, wc),
        }
    }
}

#[pyclass(name = "AlterRenameFieldQuery")]
#[derive(Clone)]
struct PyAlterRenameFieldQuery {
    #[pyo3(get)]
    table: String,
    #[pyo3(get)]
    old_name: String,
    #[pyo3(get)]
    new_name: String,
}

#[pyclass(name = "AlterDropFieldQuery")]
#[derive(Clone)]
struct PyAlterDropFieldQuery {
    #[pyo3(get)]
    table: String,
    #[pyo3(get)]
    field_name: String,
}

#[pyclass(name = "AlterMergeFieldsQuery")]
#[derive(Clone)]
struct PyAlterMergeFieldsQuery {
    #[pyo3(get)]
    table: String,
    #[pyo3(get)]
    sources: Vec<String>,
    #[pyo3(get)]
    into: String,
}

fn where_clause_to_py(py: Python<'_>, wc: &qp::WhereClause) -> PyObject {
    match wc {
        qp::WhereClause::Comparison(cmp) => {
            Py::new(py, PyComparison {
                column: cmp.column.clone(),
                op: cmp.op.clone(),
                value_inner: cmp.value.clone(),
            }).unwrap().into_pyobject(py).unwrap().into_any().unbind()
        }
        qp::WhereClause::BoolOp(bop) => {
            Py::new(py, PyBoolOp {
                op: bop.op.clone(),
                left_inner: *bop.left.clone(),
                right_inner: *bop.right.clone(),
            }).unwrap().into_pyobject(py).unwrap().into_any().unbind()
        }
    }
}

fn select_query_to_py(py: Python<'_>, q: &qp::SelectQuery) -> PyResult<PyObject> {
    let order_by = q.order_by.as_ref().map(|specs| {
        specs.iter().map(|s| PyOrderSpec {
            column: s.column.clone(),
            descending: s.descending,
        }).collect::<Vec<_>>()
    });

    let joins: Vec<PyJoinInfo> = q.joins.iter().map(|j| PyJoinInfo {
        table: j.table.clone(),
        alias: j.alias.clone(),
        left_col: j.left_col.clone(),
        right_col: j.right_col.clone(),
    }).collect();

    let py_q = Py::new(py, PyQuery {
        table: q.table.clone(),
        table_alias: q.table_alias.clone(),
        order_by,
        limit: q.limit,
        joins,
        columns_inner: q.columns.clone(),
        where_inner: q.where_clause.clone(),
    })?;
    Ok(py_q.into_pyobject(py)?.into_any().unbind())
}

// ── Table ─────────────────────────────────────────────────────────────────

#[pyclass(name = "RustTable")]
struct PyTable {
    inner: mdql_core::api::Table,
}

#[pymethods]
impl PyTable {
    #[new]
    fn new(path: &str) -> PyResult<Self> {
        let table = mdql_core::api::Table::new(PathBuf::from(path))
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        Ok(PyTable { inner: table })
    }

    #[getter]
    fn path(&self) -> String {
        self.inner.path.to_string_lossy().to_string()
    }

    #[getter]
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn schema_data(&self, py: Python<'_>) -> PyResult<PyObject> {
        let s = self.inner.schema();
        schema_to_py(py, s)
    }

    /// Load rows from the table. Optionally filter with where={"field": "value"}.
    #[pyo3(signature = (*, r#where=None))]
    fn load(
        &self,
        py: Python<'_>,
        r#where: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<(PyObject, PyObject)> {
        let (rows, errors) = self
            .inner
            .load()
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

        let filtered = if let Some(where_val) = r#where {
            if let Ok(where_dict) = where_val.downcast::<PyDict>() {
                // Dict → equality filter
                let filter = dict_to_map(where_dict)?;
                rows.into_iter()
                    .filter(|row| {
                        filter.iter().all(|(k, v)| {
                            row.get(k).map_or(false, |rv| rv.to_display_string() == v.to_display_string())
                        })
                    })
                    .collect()
            } else if let Ok(where_str) = where_val.extract::<String>() {
                // String → parse as SQL WHERE clause
                let fake_sql = format!("SELECT * FROM _t WHERE {}", where_str);
                let stmt = qp::parse_query(&fake_sql)
                    .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
                let clause = match stmt {
                    qp::Statement::Select(q) => q.where_clause.ok_or_else(|| {
                        PyValueError::new_err("Could not parse WHERE clause")
                    })?,
                    _ => return Err(PyValueError::new_err("Invalid WHERE clause")),
                };
                rows.into_iter()
                    .filter(|row| mdql_core::query_engine::evaluate(&clause, row))
                    .collect()
            } else {
                return Err(PyValueError::new_err(
                    "where must be a dict or a SQL WHERE string",
                ));
            }
        } else {
            rows
        };

        let py_rows = PyList::new(
            py,
            filtered.iter().map(|r| row_to_dict(py, r).unwrap()),
        )?;
        let py_errors = PyList::new(
            py,
            errors.iter().map(|e| e.to_string()),
        )?;
        Ok((py_rows.into_pyobject(py)?.into_any().unbind(), py_errors.into_pyobject(py)?.into_any().unbind()))
    }

    fn validate(&self, py: Python<'_>) -> PyResult<PyObject> {
        let errors = self
            .inner
            .validate()
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        let py_errors = PyList::new(py, errors.iter().map(|e| e.to_string()))?;
        Ok(py_errors.into_pyobject(py)?.into_any().unbind())
    }

    #[pyo3(signature = (fields, *, body=None, filename=None, replace=false))]
    fn insert(
        &self,
        fields: &Bound<'_, PyDict>,
        body: Option<&str>,
        filename: Option<&str>,
        replace: bool,
    ) -> PyResult<String> {
        let data = dict_to_map(fields)?;
        let path = self
            .inner
            .insert(&data, body, filename, replace)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        Ok(path.to_string_lossy().to_string())
    }

    #[pyo3(signature = (filename, fields, *, body=None))]
    fn update(
        &self,
        filename: &str,
        fields: &Bound<'_, PyDict>,
        body: Option<&str>,
    ) -> PyResult<String> {
        let data = dict_to_map(fields)?;
        let path = self.inner
            .update(filename, &data, body)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        Ok(path.to_string_lossy().to_string())
    }

    /// Update multiple files with the same field values.
    /// Returns list of updated file paths.
    fn update_many(
        &self,
        filenames: Vec<String>,
        fields: &Bound<'_, PyDict>,
    ) -> PyResult<Vec<String>> {
        let data = dict_to_map(fields)?;
        let mut updated = Vec::new();
        for filename in &filenames {
            let path = self.inner
                .update(filename, &data, None)
                .map_err(|e| PyRuntimeError::new_err(format!("{}: {}", filename, e)))?;
            updated.push(path.to_string_lossy().to_string());
        }
        Ok(updated)
    }

    fn delete(&self, filename: &str) -> PyResult<String> {
        let path = self.inner
            .delete(filename)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        Ok(path.to_string_lossy().to_string())
    }

    fn execute_sql(&mut self, sql: &str) -> PyResult<String> {
        self.inner
            .execute_sql(sql)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    /// Execute a SELECT query and return structured results.
    /// Returns (rows: list[dict], columns: list[str]).
    fn query(&mut self, py: Python<'_>, sql: &str) -> PyResult<(PyObject, PyObject)> {
        let (result_rows, columns) = self.inner
            .query_sql(sql)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

        let py_rows = PyList::new(
            py,
            result_rows.iter().map(|r| row_to_dict(py, r).unwrap()),
        )?;
        let py_cols = PyList::new(py, &columns)?;
        Ok((
            py_rows.into_pyobject(py)?.into_any().unbind(),
            py_cols.into_pyobject(py)?.into_any().unbind(),
        ))
    }

    fn rename_field(&mut self, old_name: &str, new_name: &str) -> PyResult<usize> {
        self.inner
            .rename_field(old_name, new_name)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    fn drop_field(&mut self, name: &str) -> PyResult<usize> {
        self.inner
            .drop_field(name)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    fn merge_fields(&mut self, sources: Vec<String>, into: &str) -> PyResult<usize> {
        self.inner
            .merge_fields(&sources, into)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }
}

// ── Database ──────────────────────────────────────────────────────────────

#[pyclass(name = "RustDatabase")]
struct PyDatabase {
    inner: mdql_core::api::Database,
}

#[pymethods]
impl PyDatabase {
    #[new]
    fn new(path: &str) -> PyResult<Self> {
        let db = mdql_core::api::Database::new(PathBuf::from(path))
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        Ok(PyDatabase { inner: db })
    }

    #[getter]
    fn name(&self) -> &str {
        self.inner.name()
    }

    #[getter]
    fn table_names(&self) -> Vec<String> {
        self.inner.table_names()
    }

    fn table(&mut self, name: &str) -> PyResult<PyTable> {
        let table = self
            .inner
            .table(name)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        let py_table = mdql_core::api::Table::new(&table.path)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        Ok(PyTable { inner: py_table })
    }

    /// Execute a SQL SELECT query (including JOINs) across all tables.
    /// Returns (rows: list[dict], columns: list[str]).
    fn query(&self, py: Python<'_>, sql: &str) -> PyResult<(PyObject, PyObject)> {
        let stmt = qp::parse_query(sql)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

        let select = match stmt {
            qp::Statement::Select(q) => q,
            _ => return Err(PyValueError::new_err("Only SELECT queries supported")),
        };

        let (_config, tables, _errors) =
            mdql_core::loader::load_database(&self.inner.path)
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

        let (result_rows, columns) = if !select.joins.is_empty() {
            mdql_core::query_engine::execute_join_query(&select, &tables)
        } else {
            let (schema, rows) = tables
                .get(&select.table)
                .ok_or_else(|| PyValueError::new_err(format!("Table '{}' not found", select.table)))?;
            mdql_core::query_engine::execute_query(&select, rows, schema)
        }
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

        let py_rows = PyList::new(
            py,
            result_rows.iter().map(|r| row_to_dict(py, r).unwrap()),
        )?;
        let py_cols = PyList::new(py, &columns)?;
        Ok((
            py_rows.into_pyobject(py)?.into_any().unbind(),
            py_cols.into_pyobject(py)?.into_any().unbind(),
        ))
    }
}

// ── TableTransaction ─────────────────────────────────────────────────────

#[pyclass(name = "RustTableTransaction")]
struct PyTableTransaction {
    inner: Option<mdql_core::txn::TableTransaction>,
}

#[pymethods]
impl PyTableTransaction {
    #[new]
    fn new(folder: &str, operation: &str) -> PyResult<Self> {
        let txn = mdql_core::txn::TableTransaction::new(
            std::path::Path::new(folder),
            operation,
        )
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        Ok(PyTableTransaction { inner: Some(txn) })
    }

    fn backup(&mut self, path: &str) -> PyResult<()> {
        self.inner
            .as_mut()
            .ok_or_else(|| PyRuntimeError::new_err("Transaction already committed"))?
            .backup(std::path::Path::new(path))
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    fn record_create(&mut self, path: &str) -> PyResult<()> {
        self.inner
            .as_mut()
            .ok_or_else(|| PyRuntimeError::new_err("Transaction already committed"))?
            .record_create(std::path::Path::new(path))
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    fn record_delete(&mut self, path: &str, content: &str) -> PyResult<()> {
        self.inner
            .as_mut()
            .ok_or_else(|| PyRuntimeError::new_err("Transaction already committed"))?
            .record_delete(std::path::Path::new(path), content)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    fn commit(&mut self) -> PyResult<()> {
        self.inner
            .take()
            .ok_or_else(|| PyRuntimeError::new_err("Transaction already committed"))?
            .commit()
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }

    fn rollback(&self) -> PyResult<()> {
        self.inner
            .as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("Transaction already committed"))?
            .rollback()
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    }
}

// ── Schema helpers ──────────────────────────────────────────────────────

fn schema_to_py(py: Python<'_>, s: &mdql_core::schema::Schema) -> PyResult<PyObject> {
    let dict = PyDict::new(py);
    dict.set_item("table", &s.table)?;
    dict.set_item("primary_key", &s.primary_key)?;
    dict.set_item("h1_required", s.h1_required)?;

    let fm = PyDict::new(py);
    for (name, fd) in &s.frontmatter {
        let field_dict = PyDict::new(py);
        field_dict.set_item("type", fd.field_type.as_str())?;
        field_dict.set_item("required", fd.required)?;
        if let Some(ref ev) = fd.enum_values {
            let py_list = PyList::new(py, ev)?;
            field_dict.set_item("enum", py_list)?;
        }
        fm.set_item(name, field_dict)?;
    }
    dict.set_item("frontmatter", fm)?;

    let secs = PyDict::new(py);
    for (name, sd) in &s.sections {
        let sec_dict = PyDict::new(py);
        sec_dict.set_item("type", &sd.content_type)?;
        sec_dict.set_item("required", sd.required)?;
        secs.set_item(name, sec_dict)?;
    }
    dict.set_item("sections", secs)?;

    let rules = PyDict::new(py);
    rules.set_item("reject_unknown_frontmatter", s.rules.reject_unknown_frontmatter)?;
    rules.set_item("reject_unknown_sections", s.rules.reject_unknown_sections)?;
    rules.set_item("reject_duplicate_sections", s.rules.reject_duplicate_sections)?;
    rules.set_item("normalize_numbered_headings", s.rules.normalize_numbered_headings)?;
    dict.set_item("rules", rules)?;

    if let Some(ref field) = s.h1_must_equal_frontmatter {
        dict.set_item("h1_must_equal_frontmatter", field)?;
    }

    Ok(dict.into_pyobject(py)?.into_any().unbind())
}

// ── Free functions ──────────────────────────────────────────────────────

/// parse_file(path, relative_to=None, normalize=False) -> dict
#[pyfunction]
#[pyo3(signature = (path, relative_to=None, normalize=false))]
fn parse_file(py: Python<'_>, path: &str, relative_to: Option<&str>, normalize: bool) -> PyResult<PyObject> {
    let p = std::path::Path::new(path);
    let rel = relative_to.map(std::path::Path::new);
    let parsed = mdql_core::parser::parse_file(p, rel, normalize)
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

    let dict = PyDict::new(py);
    dict.set_item("path", &parsed.path)?;
    dict.set_item("raw_frontmatter", yaml_value_to_py(py, &parsed.raw_frontmatter))?;
    dict.set_item("h1", parsed.h1.as_deref())?;
    dict.set_item("h1_line_number", parsed.h1_line_number)?;

    let sections = PyList::empty(py);
    for sec in &parsed.sections {
        let sec_dict = PyDict::new(py);
        sec_dict.set_item("raw_heading", &sec.raw_heading)?;
        sec_dict.set_item("normalized_heading", &sec.normalized_heading)?;
        sec_dict.set_item("heading", &sec.normalized_heading)?;
        sec_dict.set_item("body", &sec.body)?;
        sec_dict.set_item("line_number", sec.line_number)?;
        sections.append(sec_dict)?;
    }
    dict.set_item("sections", sections)?;

    let errors = PyList::new(py, &parsed.parse_errors)?;
    dict.set_item("parse_errors", errors)?;

    Ok(dict.into_pyobject(py)?.into_any().unbind())
}

#[pyfunction]
fn normalize_heading(raw: &str) -> String {
    mdql_core::parser::normalize_heading(raw)
}

#[pyfunction]
fn load_schema(py: Python<'_>, folder: &str) -> PyResult<PyObject> {
    let s = mdql_core::schema::load_schema(std::path::Path::new(folder))
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
    schema_to_py(py, &s)
}

#[pyfunction]
fn validate_file(py: Python<'_>, parsed_dict: &Bound<'_, PyDict>, schema_folder: &str) -> PyResult<PyObject> {
    let path_str: String = parsed_dict.get_item("path")?.unwrap().extract()?;

    let schema = mdql_core::schema::load_schema(std::path::Path::new(schema_folder))
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

    let full_path = std::path::Path::new(schema_folder).join(&path_str);
    let parsed = mdql_core::parser::parse_file(
        &full_path,
        Some(std::path::Path::new(schema_folder)),
        schema.rules.normalize_numbered_headings,
    )
    .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

    let errors = mdql_core::validator::validate_file(&parsed, &schema);
    let py_errors = PyList::new(
        py,
        errors.iter().map(|e| {
            let d = PyDict::new(py);
            d.set_item("file_path", &e.file_path).unwrap();
            d.set_item("error_type", &e.error_type).unwrap();
            d.set_item("message", &e.message).unwrap();
            d.set_item("field", e.field.as_deref()).unwrap();
            d.set_item("line_number", e.line_number).unwrap();
            d
        }),
    )?;
    Ok(py_errors.into_pyobject(py)?.into_any().unbind())
}

#[pyfunction]
fn load_table(py: Python<'_>, folder: &str) -> PyResult<(PyObject, PyObject, PyObject)> {
    let (schema, rows, errors) = mdql_core::loader::load_table(std::path::Path::new(folder))
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

    let py_schema = schema_to_py(py, &schema)?;
    let py_rows = PyList::new(
        py,
        rows.iter().map(|r| row_to_dict(py, r).unwrap()),
    )?;
    let py_errors = PyList::new(py, errors.iter().map(|e| e.to_string()))?;

    Ok((py_schema, py_rows.into_pyobject(py)?.into_any().unbind(), py_errors.into_pyobject(py)?.into_any().unbind()))
}

/// Parse a SQL query string into an AST object.
#[pyfunction]
fn parse_query(py: Python<'_>, sql: &str) -> PyResult<PyObject> {
    let stmt = qp::parse_query(sql)
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

    match stmt {
        qp::Statement::Select(q) => select_query_to_py(py, &q),
        qp::Statement::Insert(q) => {
            let pyq = Py::new(py, PyInsertQuery {
                table: q.table,
                columns: q.columns,
                values_inner: q.values,
            })?;
            Ok(pyq.into_pyobject(py)?.into_any().unbind())
        }
        qp::Statement::Update(q) => {
            let pyq = Py::new(py, PyUpdateQuery {
                table: q.table,
                assignments_inner: q.assignments,
                where_inner: q.where_clause,
            })?;
            Ok(pyq.into_pyobject(py)?.into_any().unbind())
        }
        qp::Statement::Delete(q) => {
            let pyq = Py::new(py, PyDeleteQuery {
                table: q.table,
                where_inner: q.where_clause,
            })?;
            Ok(pyq.into_pyobject(py)?.into_any().unbind())
        }
        qp::Statement::AlterRename(q) => {
            let pyq = Py::new(py, PyAlterRenameFieldQuery {
                table: q.table,
                old_name: q.old_name,
                new_name: q.new_name,
            })?;
            Ok(pyq.into_pyobject(py)?.into_any().unbind())
        }
        qp::Statement::AlterDrop(q) => {
            let pyq = Py::new(py, PyAlterDropFieldQuery {
                table: q.table,
                field_name: q.field_name,
            })?;
            Ok(pyq.into_pyobject(py)?.into_any().unbind())
        }
        qp::Statement::AlterMerge(q) => {
            let pyq = Py::new(py, PyAlterMergeFieldsQuery {
                table: q.table,
                sources: q.sources,
                into: q.into,
            })?;
            Ok(pyq.into_pyobject(py)?.into_any().unbind())
        }
    }
}

/// Execute a SQL SELECT on in-memory rows. Returns (rows, columns).
#[pyfunction]
fn execute_query_rows(
    py: Python<'_>,
    sql: &str,
    rows: &Bound<'_, PyList>,
) -> PyResult<(PyObject, PyObject)> {
    let stmt = qp::parse_query(sql)
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

    let select = match stmt {
        qp::Statement::Select(q) => q,
        _ => return Err(PyValueError::new_err("Only SELECT queries supported")),
    };

    // Convert Python dicts to Rust rows
    let rust_rows: Vec<HashMap<String, Value>> = rows.iter()
        .map(|item| {
            let dict = item.downcast::<PyDict>().map_err(|_| {
                PyValueError::new_err("Rows must be list of dicts")
            })?;
            dict_to_row(dict)
        })
        .collect::<PyResult<Vec<_>>>()?;

    // Build a minimal schema (not needed for query execution, but the API requires it)
    let dummy_schema = mdql_core::schema::Schema {
        table: select.table.clone(),
        primary_key: "path".to_string(),
        frontmatter: indexmap::IndexMap::new(),
        h1_required: false,
        h1_must_equal_frontmatter: None,
        sections: indexmap::IndexMap::new(),
        rules: mdql_core::schema::Rules {
            reject_unknown_frontmatter: false,
            reject_unknown_sections: false,
            reject_duplicate_sections: false,
            normalize_numbered_headings: false,
        },
    };

    let (result_rows, columns) = mdql_core::query_engine::execute_query(&select, &rust_rows, &dummy_schema)
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

    let py_rows = PyList::new(
        py,
        result_rows.iter().map(|r| row_to_dict(py, r).unwrap()),
    )?;
    let py_cols = PyList::new(py, &columns)?;
    Ok((py_rows.into_pyobject(py)?.into_any().unbind(), py_cols.into_pyobject(py)?.into_any().unbind()))
}

/// Execute a SQL query against a folder on disk.
#[pyfunction]
fn execute_query_folder(py: Python<'_>, sql: &str, folder: &str) -> PyResult<(PyObject, PyObject)> {
    let stmt = qp::parse_query(sql)
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

    let (schema, rows, _) = mdql_core::loader::load_table(std::path::Path::new(folder))
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

    match stmt {
        qp::Statement::Select(q) => {
            let (result_rows, columns) = mdql_core::query_engine::execute_query(&q, &rows, &schema)
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
            let py_rows = PyList::new(
                py,
                result_rows.iter().map(|r| row_to_dict(py, r).unwrap()),
            )?;
            let py_cols = PyList::new(py, &columns)?;
            Ok((py_rows.into_pyobject(py)?.into_any().unbind(), py_cols.into_pyobject(py)?.into_any().unbind()))
        }
        _ => Err(PyValueError::new_err("Only SELECT queries supported in execute_query")),
    }
}

/// Stamp a single file: add/update created and modified timestamps.
#[pyfunction]
#[pyo3(signature = (path, today=None))]
fn stamp_file(py: Python<'_>, path: &str, today: Option<&str>) -> PyResult<PyObject> {
    let now = today.and_then(|s| {
        chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S").ok()
            .or_else(|| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f").ok())
            .or_else(|| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
                .map(|d| d.and_hms_opt(0, 0, 0).unwrap()))
    });
    let result = mdql_core::stamp::stamp_file(std::path::Path::new(path), now)
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
    let dict = PyDict::new(py);
    dict.set_item("created_set", result.created_set)?;
    dict.set_item("modified_updated", result.modified_updated)?;
    Ok(dict.into_pyobject(py)?.into_any().unbind())
}

/// Atomic write: write content to file via tempfile + rename.
#[pyfunction]
fn atomic_write(path: &str, content: &str) -> PyResult<()> {
    mdql_core::txn::atomic_write(std::path::Path::new(path), content)
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))
}

/// Recover journal if one exists. Returns true if recovery was performed.
#[pyfunction]
fn recover_journal(folder: &str) -> PyResult<bool> {
    mdql_core::txn::recover_journal(std::path::Path::new(folder))
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))
}

/// Migrate: rename frontmatter key in file.
#[pyfunction]
fn rename_frontmatter_key_in_file(path: &str, old_key: &str, new_key: &str) -> PyResult<bool> {
    mdql_core::migrate::rename_frontmatter_key_in_file(
        std::path::Path::new(path), old_key, new_key,
    )
    .map_err(|e| PyRuntimeError::new_err(e.to_string()))
}

#[pyfunction]
fn drop_frontmatter_key_in_file(path: &str, key: &str) -> PyResult<bool> {
    mdql_core::migrate::drop_frontmatter_key_in_file(std::path::Path::new(path), key)
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))
}

#[pyfunction]
fn rename_section_in_file(path: &str, old_name: &str, new_name: &str, normalize: bool) -> PyResult<bool> {
    mdql_core::migrate::rename_section_in_file(
        std::path::Path::new(path), old_name, new_name, normalize,
    )
    .map_err(|e| PyRuntimeError::new_err(e.to_string()))
}

#[pyfunction]
fn drop_section_in_file(path: &str, name: &str, normalize: bool) -> PyResult<bool> {
    mdql_core::migrate::drop_section_in_file(std::path::Path::new(path), name, normalize)
        .map_err(|e| PyRuntimeError::new_err(e.to_string()))
}

#[pyfunction]
fn merge_sections_in_file(path: &str, sources: Vec<String>, into: &str, normalize: bool) -> PyResult<bool> {
    mdql_core::migrate::merge_sections_in_file(
        std::path::Path::new(path), &sources, into, normalize,
    )
    .map_err(|e| PyRuntimeError::new_err(e.to_string()))
}

#[pyfunction]
#[pyo3(signature = (
    schema_path,
    rename_fm_old=None, rename_fm_new=None,
    drop_fm=None,
    rename_sec_old=None, rename_sec_new=None,
    drop_sec=None,
    merge_sources=None, merge_into=None,
))]
fn update_schema(
    schema_path: &str,
    rename_fm_old: Option<&str>,
    rename_fm_new: Option<&str>,
    drop_fm: Option<&str>,
    rename_sec_old: Option<&str>,
    rename_sec_new: Option<&str>,
    drop_sec: Option<&str>,
    merge_sources: Option<Vec<String>>,
    merge_into: Option<&str>,
) -> PyResult<()> {
    let rename_fm = match (rename_fm_old, rename_fm_new) {
        (Some(old), Some(new)) => Some((old, new)),
        _ => None,
    };
    let rename_sec = match (rename_sec_old, rename_sec_new) {
        (Some(old), Some(new)) => Some((old, new)),
        _ => None,
    };
    let merge = match (&merge_sources, merge_into) {
        (Some(sources), Some(into)) => Some((sources.as_slice(), into)),
        _ => None,
    };
    mdql_core::migrate::update_schema(
        std::path::Path::new(schema_path),
        rename_fm, drop_fm, rename_sec, drop_sec, merge,
    )
    .map_err(|e| PyRuntimeError::new_err(e.to_string()))
}

#[pyfunction]
#[pyo3(signature = (text, max_length=None))]
fn slugify(text: &str, max_length: Option<usize>) -> String {
    let max = max_length.unwrap_or(80);
    mdql_core::api::slugify(text, max)
}

// ── Module ───────────────────────────────────────────────────────────────

#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Classes
    m.add_class::<PyTable>()?;
    m.add_class::<PyDatabase>()?;
    m.add_class::<PyTableTransaction>()?;

    // Query AST classes
    m.add_class::<PyQuery>()?;
    m.add_class::<PyComparison>()?;
    m.add_class::<PyBoolOp>()?;
    m.add_class::<PyOrderSpec>()?;
    m.add_class::<PyJoinInfo>()?;
    m.add_class::<PyInsertQuery>()?;
    m.add_class::<PyUpdateQuery>()?;
    m.add_class::<PyDeleteQuery>()?;
    m.add_class::<PyAlterRenameFieldQuery>()?;
    m.add_class::<PyAlterDropFieldQuery>()?;
    m.add_class::<PyAlterMergeFieldsQuery>()?;

    // Functions
    m.add_function(wrap_pyfunction!(parse_file, m)?)?;
    m.add_function(wrap_pyfunction!(normalize_heading, m)?)?;
    m.add_function(wrap_pyfunction!(load_schema, m)?)?;
    m.add_function(wrap_pyfunction!(validate_file, m)?)?;
    m.add_function(wrap_pyfunction!(load_table, m)?)?;
    m.add_function(wrap_pyfunction!(parse_query, m)?)?;
    m.add_function(wrap_pyfunction!(execute_query_rows, m)?)?;
    m.add_function(wrap_pyfunction!(execute_query_folder, m)?)?;
    m.add_function(wrap_pyfunction!(stamp_file, m)?)?;
    m.add_function(wrap_pyfunction!(atomic_write, m)?)?;
    m.add_function(wrap_pyfunction!(recover_journal, m)?)?;
    m.add_function(wrap_pyfunction!(rename_frontmatter_key_in_file, m)?)?;
    m.add_function(wrap_pyfunction!(drop_frontmatter_key_in_file, m)?)?;
    m.add_function(wrap_pyfunction!(rename_section_in_file, m)?)?;
    m.add_function(wrap_pyfunction!(drop_section_in_file, m)?)?;
    m.add_function(wrap_pyfunction!(merge_sections_in_file, m)?)?;
    m.add_function(wrap_pyfunction!(update_schema, m)?)?;
    m.add_function(wrap_pyfunction!(slugify, m)?)?;

    Ok(())
}
