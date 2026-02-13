use pyo3::prelude::*;
use pyo3::types::{PyAny, PyIterator, PyList};
use dissimilar::{diff as dmp_diff, Chunk as DmpChunk};

pub const DELTA_TYPE_DELETE: &str = "Delete";
pub const DELTA_TYPE_INSERT: &str = "Insert";
pub const DELTA_TYPE_CHANGE: &str = "Change";

fn collect_lines<'py>(_py: Python<'py>, seq: &Bound<'py, PyAny>) -> PyResult<Vec<Py<PyAny>>> {
    let mut out = Vec::new();
    for item in PyIterator::from_object(seq)? {
        out.push(Py::from(item?));
    }
    Ok(out)
}

fn py_eq<'py>(a: &Py<PyAny>, b: &Py<PyAny>, py: Python<'py>) -> PyResult<bool> {
    let a_bound = a.bind(py);
    let b_bound = b.bind(py);
    a_bound.eq(b_bound)
}

#[pyfunction]
#[pyo3(signature = (before, after, algorithm = "histogram"))]
fn diff<'py>(
    py: Python<'py>,
    before: &Bound<'py, PyAny>,
    after: &Bound<'py, PyAny>,
    algorithm: &str,
) -> PyResult<Vec<PyObject>> {
    let _ = algorithm;
    let before_lines = collect_lines(py, before)?;
    let after_lines = collect_lines(py, after)?;

    // 将每个唯一的 Python 对象映射到一个字符
    let mut interner: Vec<Py<PyAny>> = Vec::new();
    let mut before_chars = String::with_capacity(before_lines.len());
    for line in &before_lines {
        let pos = interner
            .iter()
            .position(|l| py_eq(l, line, py).unwrap_or(false))
            .unwrap_or_else(|| {
                interner.push(line.clone_ref(py));
                interner.len() - 1
            });
        // 增加偏移量 1，避免映射到 U+0000 导致 DMP 字符串截断
        before_chars.push(std::char::from_u32(pos as u32 + 1).unwrap());
    }

    let mut after_chars = String::with_capacity(after_lines.len());
    for line in &after_lines {
        let pos = interner
            .iter()
            .position(|l| py_eq(l, line, py).unwrap_or(false))
            .unwrap_or_else(|| {
                interner.push(line.clone_ref(py));
                interner.len() - 1
            });
        // 增加偏移量 1
        after_chars.push(std::char::from_u32(pos as u32 + 1).unwrap());
    }

    // 使用 DMP 进行 diff
    let chunks = dmp_diff(&before_chars, &after_chars);

    let mut out = Vec::<PyObject>::new();
    let mut src_pos: i64 = 0;
    let mut tgt_pos: i64 = 0;

    #[derive(Default)]
    struct Pending {
        delete: Vec<Py<PyAny>>,
        insert: Vec<Py<PyAny>>,
        src_start: i64,
        tgt_start: i64,
    }

    let mut pending = Pending {
        src_start: 0,
        tgt_start: 0,
        ..Default::default()
    };

    let flush_pending = |out: &mut Vec<PyObject>, pending: &mut Pending, py: Python| -> PyResult<()> {
        if !pending.delete.is_empty() && !pending.insert.is_empty() {
            out.push(build_record(
                py,
                DELTA_TYPE_CHANGE,
                pending.src_start,
                std::mem::take(&mut pending.delete),
                pending.tgt_start,
                std::mem::take(&mut pending.insert),
            )?);
        } else if !pending.delete.is_empty() {
            out.push(build_record(
                py,
                DELTA_TYPE_DELETE,
                pending.src_start,
                std::mem::take(&mut pending.delete),
                pending.tgt_start,
                Vec::new(),
            )?);
        } else if !pending.insert.is_empty() {
            out.push(build_record(
                py,
                DELTA_TYPE_INSERT,
                pending.src_start,
                Vec::new(),
                pending.tgt_start,
                std::mem::take(&mut pending.insert),
            )?);
        }
        Ok(())
    };

    for chunk in chunks {
        match chunk {
            DmpChunk::Equal(text) => {
                flush_pending(&mut out, &mut pending, py)?;
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
                pending.delete.extend(
                    before_lines[src_pos as usize..(src_pos + len) as usize]
                        .iter()
                        .map(|obj| obj.clone_ref(py)),
                );
                src_pos += len;
            }
            DmpChunk::Insert(text) => {
                if pending.delete.is_empty() && pending.insert.is_empty() {
                    pending.src_start = src_pos;
                    pending.tgt_start = tgt_pos;
                }
                let len = text.chars().count() as i64;
                pending.insert.extend(
                    after_lines[tgt_pos as usize..(tgt_pos + len) as usize]
                        .iter()
                        .map(|obj| obj.clone_ref(py)),
                );
                tgt_pos += len;
            }
        }
    }
    flush_pending(&mut out, &mut pending, py)?;

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
    src_lines: Vec<Py<PyAny>>,
    tgt_pos: i64,
    tgt_lines: Vec<Py<PyAny>>,
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
