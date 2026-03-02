use pyo3::prelude::*;
use pyo3::types::{PyAny, PyIterator, PyList};

use dissimilar::{diff as dmp_diff, Chunk as DmpChunk};
use imara_diff::{Algorithm, Diff, InternedInput, TokenSource};

pub const DELTA_TYPE_DELETE: &str = "Delete";
pub const DELTA_TYPE_INSERT: &str = "Insert";
pub const DELTA_TYPE_CHANGE: &str = "Change";

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

fn collect_lines<'py>(_py: Python<'py>, seq: &Bound<'py, PyAny>) -> PyResult<Vec<Py<PyAny>>> {
    let mut out = Vec::new();
    for item in PyIterator::from_object(seq)? {
        out.push(Py::from(item?));
    }
    Ok(out)
}

fn tokenize_exact<'py>(
    py: Python<'py>,
    lines: &[Py<PyAny>],
    interner: &mut Vec<Py<PyAny>>,
) -> PyResult<Vec<u32>> {
    let mut tokens = Vec::with_capacity(lines.len());
    for line in lines {
        let id = interner
            .iter()
            .position(|rep| {
                line.bind(py)
                    .rich_compare(rep.bind(py), pyo3::basic::CompareOp::Eq)
                    .and_then(|b| b.is_truthy())
                    .unwrap_or(false)
            })
            .map(|idx| idx as u32)
            .unwrap_or_else(|| {
                interner.push(line.clone_ref(py));
                (interner.len() - 1) as u32
            });
        tokens.push(id);
    }
    Ok(tokens)
}

fn py_eq<'py>(a: &Py<PyAny>, b: &Py<PyAny>, py: Python<'py>) -> bool {
    a.bind(py).eq(b.bind(py)).unwrap_or(false)
}

fn diff_imara<'py>(
    py: Python<'py>,
    before_vec: &[Py<PyAny>],
    after_vec: &[Py<PyAny>],
    alg: Algorithm,
) -> PyResult<Vec<PyObject>> {
    let mut interner = Vec::new();
    let before_tok = tokenize_exact(py, before_vec, &mut interner)?;
    let after_tok = tokenize_exact(py, after_vec, &mut interner)?;

    let input = InternedInput::new(TokenVec(&before_tok), TokenVec(&after_tok));
    let mut diff = Diff::compute(alg, &input);
    diff.postprocess_no_heuristic(&input);

    let mut out = Vec::<PyObject>::new();
    for h in diff.hunks() {
        let src_pos = h.before.start as i64;
        let tgt_pos = h.after.start as i64;

        if h.is_pure_removal() {
            let lines = h.before.clone().map(|i| before_vec[i as usize].clone_ref(py)).collect();
            out.push(build_record(py, DELTA_TYPE_DELETE, src_pos, lines, tgt_pos, Vec::new())?);
        } else if h.is_pure_insertion() {
            let lines = h.after.clone().map(|j| after_vec[j as usize].clone_ref(py)).collect();
            out.push(build_record(py, DELTA_TYPE_INSERT, src_pos, Vec::new(), tgt_pos, lines)?);
        } else {
            let src_lines = h.before.clone().map(|i| before_vec[i as usize].clone_ref(py)).collect();
            let tgt_lines = h.after.clone().map(|j| after_vec[j as usize].clone_ref(py)).collect();
            out.push(build_record(py, DELTA_TYPE_CHANGE, src_pos, src_lines, tgt_pos, tgt_lines)?);
        }
    }
    Ok(out)
}

fn diff_dmp<'py>(
    py: Python<'py>,
    before_vec: &[Py<PyAny>],
    after_vec: &[Py<PyAny>],
) -> PyResult<Vec<PyObject>> {
    let mut interner: Vec<Py<PyAny>> = Vec::new();

    let mut before_chars = String::with_capacity(before_vec.len());
    for line in before_vec {
        let pos = interner
            .iter()
            .position(|l| py_eq(l, line, py))
            .unwrap_or_else(|| { interner.push(line.clone_ref(py)); interner.len() - 1 });
        before_chars.push(char::from_u32(pos as u32 + 1).unwrap());
    }

    let mut after_chars = String::with_capacity(after_vec.len());
    for line in after_vec {
        let pos = interner
            .iter()
            .position(|l| py_eq(l, line, py))
            .unwrap_or_else(|| { interner.push(line.clone_ref(py)); interner.len() - 1 });
        after_chars.push(char::from_u32(pos as u32 + 1).unwrap());
    }

    let chunks = dmp_diff(&before_chars, &after_chars);

    let mut out = Vec::<PyObject>::new();
    let mut src_pos: i64 = 0;
    let mut tgt_pos: i64 = 0;

    struct Pending { delete: Vec<Py<PyAny>>, insert: Vec<Py<PyAny>>, src_start: i64, tgt_start: i64 }
    let mut pending = Pending { delete: vec![], insert: vec![], src_start: 0, tgt_start: 0 };

    let flush = |out: &mut Vec<PyObject>, p: &mut Pending, py: Python| -> PyResult<()> {
        if !p.delete.is_empty() && !p.insert.is_empty() {
            out.push(build_record(py, DELTA_TYPE_CHANGE, p.src_start, std::mem::take(&mut p.delete), p.tgt_start, std::mem::take(&mut p.insert))?);
        } else if !p.delete.is_empty() {
            out.push(build_record(py, DELTA_TYPE_DELETE, p.src_start, std::mem::take(&mut p.delete), p.tgt_start, vec![])?);
        } else if !p.insert.is_empty() {
            out.push(build_record(py, DELTA_TYPE_INSERT, p.src_start, vec![], p.tgt_start, std::mem::take(&mut p.insert))?);
        }
        Ok(())
    };

    for chunk in chunks {
        match chunk {
            DmpChunk::Equal(text) => {
                flush(&mut out, &mut pending, py)?;
                let len = text.chars().count() as i64;
                src_pos += len;
                tgt_pos += len;
            }
            DmpChunk::Delete(text) => {
                if pending.delete.is_empty() && pending.insert.is_empty() {
                    pending.src_start = src_pos;
                    pending.tgt_start = tgt_pos;
                }
                let len = text.chars().count() as i64;
                pending.delete.extend(before_vec[src_pos as usize..(src_pos + len) as usize].iter().map(|o| o.clone_ref(py)));
                src_pos += len;
            }
            DmpChunk::Insert(text) => {
                if pending.delete.is_empty() && pending.insert.is_empty() {
                    pending.src_start = src_pos;
                    pending.tgt_start = tgt_pos;
                }
                let len = text.chars().count() as i64;
                pending.insert.extend(after_vec[tgt_pos as usize..(tgt_pos + len) as usize].iter().map(|o| o.clone_ref(py)));
                tgt_pos += len;
            }
        }
    }
    flush(&mut out, &mut pending, py)?;

    Ok(out)
}

#[pyfunction]
#[pyo3(signature = (before, after, algorithm = "histogram"))]
fn diff<'py>(
    py: Python<'py>,
    before: &Bound<'py, PyAny>,
    after: &Bound<'py, PyAny>,
    algorithm: &str,
) -> PyResult<Vec<PyObject>> {
    let before_vec = collect_lines(py, before)?;
    let after_vec = collect_lines(py, after)?;

    match algorithm.to_lowercase().as_str() {
        "dmp" => diff_dmp(py, &before_vec, &after_vec),
        "histogram" => diff_imara(py, &before_vec, &after_vec, Algorithm::Histogram),
        "myers" => diff_imara(py, &before_vec, &after_vec, Algorithm::Myers),
        _ => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "Unknown algorithm: {}", algorithm
        ))),
    }
}

#[pyclass]
struct Chunk {
    #[pyo3(get)]
    position: i64,
    #[pyo3(get)]
    lines: Py<PyList>,
}

#[pymethods]
impl Chunk {
    #[new]
    fn new(position: i64, lines: Py<PyList>) -> Self {
        Self { position, lines }
    }
}

#[pyclass]
struct Delta {
    #[pyo3(get, name = "type")]
    type_: String,
    #[pyo3(get)]
    source: Py<Chunk>,
    #[pyo3(get)]
    target: Py<Chunk>,
}

#[pymethods]
impl Delta {
    #[new]
    fn new(type_: String, source: Py<Chunk>, target: Py<Chunk>) -> Self {
        Self {
            type_,
            source,
            target,
        }
    }
}

fn build_record<'py>(
    py: Python<'py>,
    kind: &str,
    src_pos: i64,
    src_lines: Vec<Py<PyAny>>,
    tgt_pos: i64,
    tgt_lines: Vec<Py<PyAny>>,
) -> PyResult<PyObject> {
    let src_list = PyList::new(py, &src_lines)?;
    let tgt_list = PyList::new(py, &tgt_lines)?;

    // Create source and target objects
    let source = Py::new(
        py,
        Chunk {
            position: src_pos,
            lines: Py::from(src_list),
        },
    )?;

    let target = Py::new(
        py,
        Chunk {
            position: tgt_pos,
            lines: Py::from(tgt_list),
        },
    )?;

    let record = Delta {
        type_: kind.to_string(),
        source,
        target,
    };

    Ok(Py::new(py, record)?.into())
}

#[pymodule]
fn imarapy<'py>(_py: Python<'py>, m: &Bound<'py, PyModule>) -> PyResult<()> {
    m.add_class::<Chunk>()?;
    m.add_class::<Delta>()?;
    m.add_function(wrap_pyfunction!(diff, m)?)?;

    m.add("DELTA_TYPE_DELETE", DELTA_TYPE_DELETE)?;
    m.add("DELTA_TYPE_INSERT", DELTA_TYPE_INSERT)?;
    m.add("DELTA_TYPE_CHANGE", DELTA_TYPE_CHANGE)?;

    Ok(())
}
