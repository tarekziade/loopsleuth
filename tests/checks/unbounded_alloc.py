"""
Examples for the unbounded-alloc check.
"""


def build_string_slow(parts):
    """Unbounded alloc: string concatenation in loop"""
    s = ""
    for part in parts:
        s += part
    return s


def concat_list_slow(items):
    """Unbounded alloc: list concatenation in loop"""
    result = []
    for item in items:
        result = result + [item]
    return result


def cat_tensors_slow(tensors):
    """Unbounded alloc: torch.cat in loop"""
    import torch

    out = None
    for t in tensors:
        out = t if out is None else torch.cat([out, t], dim=0)
    return out


# Clean examples (should NOT be flagged)


def collect_list_fast(items):
    """Safe: list append"""
    result = []
    for item in items:
        result.append(item)
    return result


def counter_increment(items):
    """Safe: counter increment"""
    count = 0
    for _item in items:
        count += 1
    return count
