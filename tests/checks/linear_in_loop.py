"""
Examples for the linear-in-loop check.
"""


def remove_elements(lst, to_remove):
    """Linear-in-loop: list.remove() inside loop"""
    for item in to_remove:
        if item in lst:
            lst.remove(item)
    return lst


def remove_duplicates_slow(lst):
    """Linear-in-loop: membership test in loop"""
    result = []
    for item in lst:
        if item not in result:
            result.append(item)
    return result


def index_in_loop(values, order):
    """Linear-in-loop: list.index() inside loop"""
    out = []
    for item in values:
        out.append(order.index(item))
    return out


# Clean examples (should NOT be flagged)


def remove_duplicates_fast(lst):
    """Linear: using dict to preserve order"""
    return list(dict.fromkeys(lst))


def join_strings_fast(words):
    """Linear: using join()"""
    return " ".join(words)


def linear_search(arr, target):
    """Linear: single loop without hidden linear ops"""
    for i, val in enumerate(arr):
        if val == target:
            return i
    return -1
