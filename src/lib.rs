use pyo3::prelude::*;
use pyo3::types::{PyAny, PyIterator, PyList};
use dissimilar::{diff as dmp_diff, Chunk as DmpChunk};

pub const DELTA_TYPE_DELETE: &str = "Delete";
pub const DELTA_TYPE_INSERT: &str = "Insert";
pub const DELTA_TYPE_CHANGE: &str = "Change";

fn collect_lines<'py>(_py: Python<'py>, seq: &Bound<'py, PyAny>) -> PyResult<Vec<String>> {
    let mut out = Vec::new();
    for item in PyIterator::from_object(seq)? {
        let item = item?;
        out.push(item.extract::<String>()?);
    }
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
    let before_lines = collect_lines(py, before)?;
    let after_lines = collect_lines(py, after)?;

    // DMP works on strings. We join lines with a unique separator to treat them as atomic units,
    // or we use the char-based approach if the lines are short.
    // However, dissimilar's diff(a, b) takes &str.
    // To implement line-based diff with DMP "semantic cleanup" logic:
    // We can use the same technique as many DMP wrappers: map each unique line to a character.

    let mut interner = Vec::new();
    let mut before_chars = String::with_capacity(before_lines.len());
    for line in &before_lines {
        let pos = interner.iter().position(|l| l == line);
        let char_code = match pos {
            Some(p) => p,
            None => {
                interner.push(line.clone());
                interner.len() - 1
            }
        };
        before_chars.push(std::char::from_u32(char_code as u32).unwrap());
    }

    let mut after_chars = String::with_capacity(after_lines.len());
    for line in &after_lines {
        let pos = interner.iter().position(|l| l == line);
        let char_code = match pos {
            Some(p) => p,
            None => {
                interner.push(line.clone());
                interner.len() - 1
            }
        };
        after_chars.push(std::char::from_u32(char_code as u32).unwrap());
    }

    let chunks = dmp_diff(&before_chars, &after_chars);

    let mut out = Vec::<PyObject>::new();
    let mut src_pos = 0;
    let mut tgt_pos = 0;

    for chunk in chunks {
        match chunk {
            DmpChunk::Equal(text) => {
                let len = text.chars().count() as i64;
                src_pos += len;
                tgt_pos += len;
            }
            DmpChunk::Delete(text) => {
                let lines: Vec<String> = text.chars().map(|c| interner[c as u32 as usize].clone()).collect();
                let len = lines.len() as i64;
                out.push(build_record(py, DELTA_TYPE_DELETE, src_pos, lines.clone(), tgt_pos, Vec::new())?);
                src_pos += len;
            }
            DmpChunk::Insert(text) => {
                let lines: Vec<String> = text.chars().map(|c| interner[c as u32 as usize].clone()).collect();
                let len = lines.len() as i64;
                out.push(build_record(py, DELTA_TYPE_INSERT, src_pos, Vec::new(), tgt_pos, lines.clone())?);
                tgt_pos += len;
            }
        }
    }

    Ok(out)
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
    src_lines: Vec<String>,
    tgt_pos: i64,
    tgt_lines: Vec<String>,
) -> PyResult<PyObject> {
    let src_list = PyList::new(py, &src_lines)?;
    let tgt_list = PyList::new(py, &tgt_lines)?;

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
