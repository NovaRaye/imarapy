# imarapy

[中文文档](README_CN.md)

Python bindings for the [dissimilar](https://github.com/dtolnay/dissimilar) Rust library, providing efficient diffing for Python objects with identity preservation.

## Features

- **Fast Diffing**: Leverages the `dissimilar` Rust crate for high-performance diffing.
- **Object Support**: Unlike standard text diff tools, `imarapy` handles arbitrary Python objects by mapping them to unique tokens.
- **Identity Preservation**: Ensures that the original object instances from your input lists are returned in the diff results, respecting custom `__eq__` implementations without losing object identity.

## Installation

### Via PyPI

```bash
pip install imarapy
```

## Usage

Please refer to [demo.py](./demo.py) for detailed usage examples.
