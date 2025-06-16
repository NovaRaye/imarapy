use pyo3::conversion::IntoPyObjectExt;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyIterator, PyList};

use imara_diff::{Algorithm, Diff, InternedInput, TokenSource};

struct TokenVec<'a>(&'a [u32]);
impl<'a> TokenSource for TokenVec<'a> {
    type Token = u32;
    type Tokenizer = std::iter::Copied<std::slice::Iter<'a, u32>>;
    #[inline]
    fn tokenize(&self) -> Self::Tokenizer {
        self.0.iter().copied()
    }
    #[inline]
    fn estimate_tokens(&self) -> u32 {
        self.0.len() as u32
    }
}

fn collect_rows<'py>(_py: Python<'py>, seq: &Bound<'py, PyAny>) -> PyResult<Vec<Py<PyAny>>> {
    let mut out = Vec::new();
    for item in PyIterator::from_object(seq)? {
        out.push(Py::from(item?));
    }
    Ok(out)
}

fn tokenize_exact<'py>(
    py: Python<'py>,
    rows: &[Py<PyAny>],
    interner: &mut Vec<Py<PyAny>>,
) -> PyResult<Vec<u32>> {
    let mut tokens = Vec::with_capacity(rows.len());
    for row in rows {
        let id = interner
            .iter()
            .position(|rep| {
                row.bind(py)
                    .rich_compare(rep.bind(py), pyo3::basic::CompareOp::Eq)
                    .and_then(|b| b.is_truthy())
                    .unwrap_or(false)
            })
            .map(|idx| idx as u32)
            .unwrap_or_else(|| {
                interner.push(row.clone_ref(py));
                (interner.len() - 1) as u32
            });
        tokens.push(id);
    }
    Ok(tokens)
}

#[pyfunction]
#[pyo3(signature = (before, after, algorithm = "histogram"))]
fn diff<'py>(
    py: Python<'py>,
    before: &Bound<'py, PyAny>,
    after: &Bound<'py, PyAny>,
    algorithm: &str,
) -> PyResult<Vec<PyObject>> {
    let alg = match algorithm.to_lowercase().as_str() {
        "histogram" => Algorithm::Histogram,
        "myers" => Algorithm::Myers,
        _ => {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Unknown algorithm: {}",
                algorithm
            )));
        }
    };

    let before_vec = collect_rows(py, before)?;
    let after_vec = collect_rows(py, after)?;

    let mut interner = Vec::new();
    let before_tok = tokenize_exact(py, &before_vec, &mut interner)?;
    let after_tok = tokenize_exact(py, &after_vec, &mut interner)?;

    let input = InternedInput::new(TokenVec(&before_tok), TokenVec(&after_tok));
    let mut diff = Diff::compute(alg, &input);
    diff.postprocess_no_heuristic(&input);

    let mut out = Vec::<PyObject>::new();
    for h in diff.hunks() {
        let src_pos = h.before.start as i64 + 1;
        let tgt_pos = h.after.start as i64 + 1;

        if h.is_pure_removal() {
            let rows = h
                .before
                .clone()
                .map(|i| before_vec[i as usize].clone_ref(py))
                .collect();
            out.push(build_record(
                py,
                "Delete",
                src_pos,
                rows,
                tgt_pos,
                Vec::new(),
            )?);
        } else if h.is_pure_insertion() {
            let rows = h
                .after
                .clone()
                .map(|j| after_vec[j as usize].clone_ref(py))
                .collect();
            out.push(build_record(
                py,
                "Insert",
                src_pos,
                Vec::new(),
                tgt_pos,
                rows,
            )?);
        } else {
            let src_rows = h
                .before
                .clone()
                .map(|i| before_vec[i as usize].clone_ref(py))
                .collect();
            let tgt_rows = h
                .after
                .clone()
                .map(|j| after_vec[j as usize].clone_ref(py))
                .collect();
            out.push(build_record(
                py, "Change", src_pos, src_rows, tgt_pos, tgt_rows,
            )?);
        }
    }
    Ok(out)
}

fn build_record<'py>(
    py: Python<'py>,
    kind: &str,
    src_pos: i64,
    src_rows: Vec<Py<PyAny>>,
    tgt_pos: i64,
    tgt_rows: Vec<Py<PyAny>>,
) -> PyResult<PyObject> {
    let source = PyDict::new(py);
    let src_list = PyList::new(py, &src_rows)?;
    source.set_item("position", src_pos)?;
    source.set_item("rows", src_list)?;

    let target = PyDict::new(py);
    let tgt_list = PyList::new(py, &tgt_rows)?;
    target.set_item("position", tgt_pos)?;
    target.set_item("rows", tgt_list)?;

    let rec = PyDict::new(py);
    rec.set_item("type", kind)?;
    rec.set_item("source", source)?;
    rec.set_item("target", target)?;

    Ok(rec.into_py_any(py)?)
}

#[pymodule]
fn imarapy<'py>(_py: Python<'py>, m: &Bound<'py, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(diff, m)?)?;
    Ok(())
}
