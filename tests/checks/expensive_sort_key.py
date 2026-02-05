"""
Examples for the expensive-sort-key check.
"""


def sort_by_index(items, order):
    """Expensive sort key: list.index in key"""
    return sorted(items, key=lambda x: order.index(x))


def sort_by_membership(items, priority_list):
    """Expensive sort key: membership in key"""
    return sorted(items, key=lambda x: x in priority_list)


def sort_with_regex(items, pattern):
    """Expensive sort key: regex match in key"""
    import re

    return sorted(items, key=lambda x: re.match(pattern, x) is not None)


# Clean examples (should NOT be flagged)


def sort_by_attribute(items):
    """Cheap key: attribute access"""
    return sorted(items, key=lambda x: x.value)


def sort_by_len(items):
    """Cheap key: len is O(1)"""
    items.sort(key=len)
    return items
