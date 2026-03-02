import imarapy
import unittest

def diff(*args, **kwargs):
    kwargs.setdefault("algorithm", "dmp")
    return imarapy.diff(*args, **kwargs)

class TestImaraPy(unittest.TestCase):
    def test_basic_strings(self):
        before = ["line1", "line2", "line3"]
        after = ["line1", "new_line", "line3"]
        deltas = diff(before, after)
        self.assertEqual(len(deltas), 1)
        self.assertEqual(deltas[0].type, imarapy.DELTA_TYPE_CHANGE)
        self.assertEqual(deltas[0].source.position, 1)
        self.assertEqual(deltas[0].target.position, 1)
        self.assertEqual(deltas[0].source.lines, ["line2"])
        self.assertEqual(deltas[0].target.lines, ["new_line"])

    def test_insert_delete(self):
        before = ["A", "B"]
        after = ["A", "B", "C"]
        deltas = diff(before, after)
        self.assertEqual(len(deltas), 1)
        self.assertEqual(deltas[0].type, imarapy.DELTA_TYPE_INSERT)
        self.assertEqual(deltas[0].source.position, 2)
        self.assertEqual(deltas[0].target.position, 2)
        self.assertEqual(deltas[0].target.lines, ["C"])

        before = ["A", "B", "C"]
        after = ["A", "B"]
        deltas = diff(before, after)
        self.assertEqual(len(deltas), 1)
        self.assertEqual(deltas[0].type, imarapy.DELTA_TYPE_DELETE)
        self.assertEqual(deltas[0].source.position, 2)
        self.assertEqual(deltas[0].target.position, 2)
        self.assertEqual(deltas[0].source.lines, ["C"])

    def test_multiple_changes(self):
        before = ["A", "B", "C", "D", "E"]
        after = ["A", "X", "Y", "Z", "E"]
        deltas = diff(before, after)
        self.assertEqual(len(deltas), 1)
        self.assertEqual(deltas[0].source.position, 1)
        self.assertEqual(deltas[0].target.position, 1)
        self.assertEqual(len(deltas[0].source.lines), 3) # B, C, D merged

    def test_discrete_changes(self):
        before = ["A", "B", "C1", "C2", "C3", "C4", "C5", "D", "E"]
        after = ["A", "X", "C1", "C2", "C3", "C4", "C5", "Y", "E"]
        deltas = diff(before, after)
        self.assertEqual(len(deltas), 2)
        self.assertEqual(deltas[0].source.position, 1)
        self.assertEqual(deltas[0].target.position, 1)
        self.assertEqual(deltas[1].source.position, 7)
        self.assertEqual(deltas[1].target.position, 7)

    def test_swap_elements(self):
        before = ["A", "B"]
        after = ["B", "A"]
        deltas = diff(before, after)
        for delta in deltas:
            self.assertGreaterEqual(delta.source.position, 0)
            self.assertGreaterEqual(delta.target.position, 0)

    def test_custom_objects_identity(self):
        class Token:
            def __init__(self, name, version):
                self.name = name
                self.version = version
            def __eq__(self, other):
                if not isinstance(other, Token): return False
                return self.name == other.name
            def __hash__(self):
                return hash(self.name)
            def __repr__(self):
                return f"Token({self.name}, v{self.version})"
        obj_a1 = Token("A", 1)
        obj_a2 = Token("A", 2)
        obj_b1 = Token("B", 1)
        obj_b2 = Token("B", 2)
        before = [obj_a1, obj_b1]
        after = [obj_b2, obj_a2]
        deltas = diff(before, after)
        for delta in deltas:
            for line in delta.source.lines:
                self.assertTrue(any(line is orig for orig in before))
            for line in delta.target.lines:
                self.assertTrue(any(line is orig for orig in after))

    def test_normalization_simulation(self):
        class CaseInsensitiveStr:
            def __init__(self, s):
                self.s = s
            def __eq__(self, other):
                return self.s.lower() == other.s.lower()
            def __hash__(self):
                return hash(self.s.lower())
            def __repr__(self):
                return f"'{self.s}'"
        before = [CaseInsensitiveStr("APPLE"), CaseInsensitiveStr("BANANA")]
        after = [CaseInsensitiveStr("apple"), CaseInsensitiveStr("cherry")]
        deltas = diff(before, after)
        self.assertEqual(len(deltas), 1)
        self.assertEqual(deltas[0].source.position, 1)
        self.assertEqual(deltas[0].target.position, 1)

    def test_empty_and_single(self):
        self.assertEqual(len(diff(["A"], ["A"])), 0)
        self.assertEqual(len(diff([], [])), 0)
        deltas = diff(["A"], ["B"])
        self.assertEqual(len(deltas), 1)
        self.assertEqual(deltas[0].source.position, 0)
        self.assertEqual(deltas[0].target.position, 0)

    def test_repeated_elements(self):
        before = ["A", "A", "A"]
        after = ["A", "B", "A"]
        deltas = diff(before, after)
        self.assertEqual(len(deltas), 1)
        self.assertEqual(deltas[0].source.position, 1)
        self.assertEqual(deltas[0].target.position, 1)

    def test_non_hashable_objects(self):
        before = [{"id": 1}, {"id": 2}]
        after = [{"id": 1}, {"id": 3}]
        try:
            deltas = diff(before, after)
            self.assertEqual(len(deltas), 1)
            self.assertEqual(deltas[0].source.position, 1)
        except TypeError:
            pass

    def test_large_diff(self):
        before = [str(i) for i in range(100)]
        after = [str(i) for i in range(100)]
        after[50] = "changed"
        after.insert(20, "inserted")
        del after[80]
        deltas = diff(before, after)
        self.assertTrue(len(deltas) >= 3)

    def test_identity_preservation_complex(self):
        class Item:
            def __init__(self, val, id):
                self.val = val
                self.id = id
            def __eq__(self, other):
                return self.val == other.val
            def __hash__(self):
                return hash(self.val)
        i1_v1 = Item("A", 1)
        i1_v2 = Item("A", 2)
        deltas = diff([i1_v1], [i1_v2])
        self.assertEqual(len(deltas), 0)
        deltas = diff([i1_v1, Item("B", 1)], [i1_v2, Item("C", 1)])
        self.assertEqual(len(deltas), 1)
        self.assertEqual(deltas[0].source.position, 1)
        self.assertEqual(deltas[0].target.position, 1)

    def test_only_deletions(self):
        before = ["A", "B", "C"]
        after = []
        deltas = diff(before, after)
        self.assertEqual(len(deltas), 1)
        self.assertEqual(deltas[0].source.position, 0)
        self.assertEqual(deltas[0].target.position, 0)

    def test_only_insertions(self):
        before = []
        after = ["X", "Y", "Z"]
        deltas = diff(before, after)
        self.assertEqual(len(deltas), 1)
        self.assertEqual(deltas[0].source.position, 0)
        self.assertEqual(deltas[0].target.position, 0)

    def test_identical_objects_different_identity(self):
        before = ["string"]
        after = ["".join(["str", "ing"])]
        deltas = diff(before, after)
        self.assertEqual(len(deltas), 0)


class TestAlgorithms(unittest.TestCase):
    """确保 histogram、myers、dmp 三种算法结果语义一致。"""

    CASES = [
        (["A", "B", "C"], ["A", "X", "C"]),
        (["A", "B"], ["A", "B", "C"]),
        (["A", "B", "C"], ["A", "B"]),
        ([], ["X", "Y"]),
        (["X", "Y"], []),
        (["A"], ["A"]),
    ]

    @staticmethod
    def apply_diff(before, deltas):
        """将 diff 结果应用到 before，还原出 after。"""
        result = []
        src_pos = 0
        for d in sorted(deltas, key=lambda d: d.source.position):
            result.extend(before[src_pos:d.source.position])
            result.extend(d.target.lines)
            src_pos = d.source.position + len(d.source.lines)
        result.extend(before[src_pos:])
        return result

    def _check(self, algo):
        for before, after in self.CASES:
            with self.subTest(algo=algo, before=before, after=after):
                deltas = imarapy.diff(before, after, algorithm=algo)
                reconstructed = self.apply_diff(before, deltas)
                self.assertEqual(reconstructed, after)

    def test_histogram(self):
        self._check("histogram")

    def test_myers(self):
        self._check("myers")

    def test_dmp(self):
        self._check("dmp")

    def test_unknown_algorithm_raises(self):
        with self.assertRaises(ValueError):
            imarapy.diff(["A"], ["B"], algorithm="unknown")


if __name__ == "__main__":
    unittest.main()
