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
        before_chars.push(std::char::from_u32(pos as u32).unwrap());
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
        after_chars.push(std::char::from_u32(pos as u32).unwrap());
    }

    // 使用 DMP 进行 diff
    let chunks = dmp_diff(&before_chars, &after_chars);

    // 暂存连续的删除和插入以合并为 Change
    #[derive(Default)]
    struct Pending {
        delete: Vec<u32>, // 存储字符索引
        insert: Vec<u32>,
    }

    let mut out = Vec::<PyObject>::new();
    let mut pending = Pending::default();
    let mut src_pos = 0;
    let mut tgt_pos = 0;

    let flush_pending = |out: &mut Vec<PyObject>,
                         pending: &mut Pending,
                         py: Python,
                         interner: &[Py<PyAny>],
                         src_pos: &mut i64,
                         tgt_pos: &mut i64|
     -> PyResult<()> {
        if !pending.delete.is_empty() && !pending.insert.is_empty() {
            // 合并为 Change
            let delete_lines: Vec<Py<PyAny>> = pending
                .delete
                .iter()
                .map(|&c| interner[c as usize].clone_ref(py))
                .collect();
            let insert_lines: Vec<Py<PyAny>> = pending
                .insert
                .iter()
                .map(|&c| interner[c as usize].clone_ref(py))
                .collect();
            out.push(build_record(
                py,
                DELTA_TYPE_CHANGE,
                *src_pos,
                delete_lines,
                *tgt_pos,
                insert_lines,
            )?);
            *src_pos += pending.delete.len() as i64;
            *tgt_pos += pending.insert.len() as i64;
        } else if !pending.delete.is_empty() {
            // 纯删除
            let lines: Vec<Py<PyAny>> = pending
                .delete
                .iter()
                .map(|&c| interner[c as usize].clone_ref(py))
                .collect();
            out.push(build_record(
                py,
                DELTA_TYPE_DELETE,
                *src_pos,
                lines,
                *tgt_pos,
                Vec::new(),
            )?);
            *src_pos += pending.delete.len() as i64;
        } else if !pending.insert.is_empty() {
            // 纯插入
            let lines: Vec<Py<PyAny>> = pending
                .insert
                .iter()
                .map(|&c| interner[c as usize].clone_ref(py))
                .collect();
            out.push(build_record(
                py,
                DELTA_TYPE_INSERT,
                *src_pos,
                Vec::new(),
                *tgt_pos,
                lines,
            )?);
            *tgt_pos += pending.insert.len() as i64;
        }
        pending.delete.clear();
        pending.insert.clear();
        Ok(())
    };

    for chunk in chunks {
        match chunk {
            DmpChunk::Equal(text) => {
                flush_pending(&mut out, &mut pending, py, &interner, &mut src_pos, &mut tgt_pos)?;
                let len = text.chars().count() as i64;
                src_pos += len;
                tgt_pos += len;
            }
            DmpChunk::Delete(text) => {
                pending.delete.extend(text.chars().map(|c| c as u32));
            }
            DmpChunk::Insert(text) => {
                pending.insert.extend(text.chars().map(|c| c as u32));
            }
        }
    }
    flush_pending(&mut out, &mut pending, py, &interner, &mut src_pos, &mut tgt_pos)?;

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
