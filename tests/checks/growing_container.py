"""
Examples for the growing-container check.
"""


def grow_same_list(items):
    """Growing container: appending to list being iterated"""
    for item in items:
        items.append(item)
    return items


def grow_same_list_snapshot(items):
    """Growing container: explicit growth during iteration"""
    for item in items:
        if item % 2 == 0:
            items.append(item + 1)
    return items


# Clean examples (should NOT be flagged)


def grow_other_list(items):
    """Safe: appending to a different list"""
    out = []
    for item in items:
        out.append(item)
    return out


def update_in_place(items):
    """Safe: modifying elements without growing"""
    for i in range(len(items)):
        items[i] = items[i] * 2
    return items
