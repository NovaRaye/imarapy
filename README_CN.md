# imarapy

[English Documentation](README.md)

[dissimilar](https://github.com/dtolnay/dissimilar) Rust 库的 Python 绑定，为 Python 对象提供高效的差异比较，并保持对象身份（Identity）。

## 特性

- **高性能 Diff**：利用 `dissimilar` Rust crate 实现极速的差异计算。
- **对象支持**：与标准的文本 diff 工具不同，`imarapy` 通过将 Python 对象映射到唯一 Token，支持对任意 Python 对象进行比较。
- **身份保持（Identity Preservation）**：确保 diff 结果中返回的是输入列表中的原始对象实例。尊重自定义的 `__eq__` 实现，同时不会丢失对象的身份。

## 安装

### 通过 PyPI 安装

```bash
pip install imarapy
```

## 使用方法

请参考 [demo.py](./demo.py) 获取详细的使用示例。
