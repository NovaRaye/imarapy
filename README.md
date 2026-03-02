# imarapy

[中文文档](README_CN.md)

Python bindings for the [imara-diff](https://github.com/pascalkuthe/imara-diff) Rust library, providing efficient diffing for Python objects with identity preservation.

## Features

- **Fast Diffing**: Leverages the `imara-diff` Rust crate for high-performance diffing.
- **Multiple Algorithms**: Supports `histogram` (default), `myers`, and `dmp` (diff-match-patch).
- **Object Support**: Handles arbitrary Python objects by mapping them to unique tokens via `__eq__`.
- **Identity Preservation**: Returns the original object instances from your input lists in the diff results, respecting custom `__eq__` implementations without losing object identity.

## Installation

### Via PyPI

```bash
pip install imarapy
```

## Usage

```python
import imarapy

before = ["line1", "line2", "line3"]
after  = ["line1", "changed", "line3"]

# Default algorithm: histogram
deltas = imarapy.diff(before, after)

# Choose algorithm explicitly
deltas = imarapy.diff(before, after, algorithm="histogram")  # imara-diff Histogram
deltas = imarapy.diff(before, after, algorithm="myers")      # imara-diff Myers
deltas = imarapy.diff(before, after, algorithm="dmp")        # diff-match-patch

for d in deltas:
    print(d.type)               # "Insert" | "Delete" | "Change"
    print(d.source.position)    # position in before
    print(d.source.lines)       # affected lines in before
    print(d.target.position)    # position in after
    print(d.target.lines)       # affected lines in after
```

Constants: `imarapy.DELTA_TYPE_INSERT`, `imarapy.DELTA_TYPE_DELETE`, `imarapy.DELTA_TYPE_CHANGE`

Please refer to [demo.py](./demo.py) for more usage examples.
