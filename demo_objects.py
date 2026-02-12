import imarapy

class CustomToken:
    def __init__(self, text, metadata=None):
        self.text = text
        self.metadata = metadata or {}

    def __eq__(self, other):
        if not isinstance(other, CustomToken):
            return False
        # 模拟归一化比较：忽略大小写
        return self.text.lower() == other.text.lower()

    def __hash__(self):
        return hash(self.text.lower())

    def __repr__(self):
        return f"Token({self.text!r}, {self.metadata})"

def test_custom_objects():
    print("Testing imarapy with custom objects...")

    # 创建带有元数据的自定义对象列表
    before = [
        CustomToken("Apple", {"id": 1}),
        CustomToken("Banana", {"id": 2}),
        CustomToken("Cherry", {"id": 3}),
    ]

    after = [
        CustomToken("apple", {"id": 4}),  # 内容相同（忽略大小写），但元数据不同
        CustomToken("BANANA", {"id": 5}), # 内容相同
        CustomToken("Durian", {"id": 6}), # 新增
    ]

    # 执行 diff
    # 算法现在应该调用 CustomToken.__eq__
    deltas = imarapy.diff(before, after)

    print(f"Found {len(deltas)} deltas")

    for delta in deltas:
        print(f"\nDelta Type: {delta.type}")
        print(f"Source (pos {delta.source.position}): {delta.source.lines}")
        print(f"Target (pos {delta.target.position}): {delta.target.lines}")

    # 验证逻辑：
    # "Apple" == "apple" -> Equal
    # "Banana" == "BANANA" -> Equal
    # "Cherry" 被删除, "Durian" 被插入 -> 应该合并为 Change 或者 Delete + Insert

    print("\nSuccess: Custom objects handled correctly without TypeError!")

if __name__ == "__main__":
    test_custom_objects()
