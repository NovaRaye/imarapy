# imarapy

[English Documentation](README.md)

[imara-diff](https://github.com/pascalkuthe/imara-diff) Rust 库的 Python 绑定，为 Python 对象提供高效的差异比较，并保持对象身份（Identity）。

## 特性

- **高性能 Diff**：利用 `imara-diff` Rust crate 实现极速的差异计算。
- **多算法支持**：支持 `histogram`（默认）、`myers`、`dmp`（diff-match-patch）三种算法。
- **对象支持**：通过 `__eq__` 将任意 Python 对象映射到唯一 Token，支持对任意 Python 对象进行比较。
- **身份保持（Identity Preservation）**：确保 diff 结果中返回的是输入列表中的原始对象实例，尊重自定义的 `__eq__` 实现，同时不会丢失对象身份。

## 安装

### 通过 PyPI 安装

```bash
pip install imarapy
```

## 使用方法

```python
import imarapy

before = ["line1", "line2", "line3"]
after  = ["line1", "changed", "line3"]

# 默认算法：histogram
deltas = imarapy.diff(before, after)

# 显式指定算法
deltas = imarapy.diff(before, after, algorithm="histogram")  # imara-diff Histogram
deltas = imarapy.diff(before, after, algorithm="myers")      # imara-diff Myers
deltas = imarapy.diff(before, after, algorithm="dmp")        # diff-match-patch

for d in deltas:
    print(d.type)               # "Insert" | "Delete" | "Change"
    print(d.source.position)    # 在 before 中的位置
    print(d.source.lines)       # before 中受影响的行
    print(d.target.position)    # 在 after 中的位置
    print(d.target.lines)       # after 中受影响的行
```

常量：`imarapy.DELTA_TYPE_INSERT`、`imarapy.DELTA_TYPE_DELETE`、`imarapy.DELTA_TYPE_CHANGE`

更多使用示例请参考 [demo.py](./demo.py)。
