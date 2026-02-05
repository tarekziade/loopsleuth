"""
Examples for the quadratic check.
"""


def bubble_sort(arr):
    """Quadratic: nested loops over same array"""
    n = len(arr)
    for i in range(n):
        for j in range(0, n - i - 1):
            if arr[j] > arr[j + 1]:
                arr[j], arr[j + 1] = arr[j + 1], arr[j]
    return arr


def find_duplicates(nums):
    """Quadratic: nested iteration to find duplicates"""
    duplicates = []
    for i in range(len(nums)):
        for j in range(i + 1, len(nums)):
            if nums[i] == nums[j] and nums[i] not in duplicates:
                duplicates.append(nums[i])
    return duplicates


def matrix_multiply_naive(a, b):
    """Quadratic-or-worse: naive triple loop"""
    n = len(a)
    result = [[0] * n for _ in range(n)]
    for i in range(n):
        for j in range(n):
            for k in range(n):
                result[i][j] += a[i][k] * b[k][j]
    return result


def check_duplicates_naive(arr1, arr2):
    """Quadratic: O(n*m) - checking intersection without sets"""
    common = []
    for item1 in arr1:
        for item2 in arr2:
            if item1 == item2 and item1 not in common:
                common.append(item1)
    return common


def sum_of_pairs(nums, target):
    """Quadratic: checking all pairs"""
    pairs = []
    for i in range(len(nums)):
        for j in range(i + 1, len(nums)):
            if nums[i] + nums[j] == target:
                pairs.append((nums[i], nums[j]))
    return pairs


def contains_subsequence_slow(text, pattern):
    """Quadratic: naive substring search"""
    n, m = len(text), len(pattern)
    for i in range(n - m + 1):
        match = True
        for j in range(m):
            if text[i + j] != pattern[j]:
                match = False
                break
        if match:
            return True
    return False


# Clean examples (should NOT be flagged)


def simple_loop(items):
    """Linear: simple O(n) iteration"""
    result = []
    for item in items:
        result.append(item * 2)
    return result


def linear_search(arr, target):
    """Linear: single loop"""
    for i, val in enumerate(arr):
        if val == target:
            return i
    return -1


def binary_search(arr, target):
    """Logarithmic: divide and conquer"""
    left, right = 0, len(arr) - 1
    while left <= right:
        mid = (left + right) // 2
        if arr[mid] == target:
            return mid
        if arr[mid] < target:
            left = mid + 1
        else:
            right = mid - 1
    return -1


def process_with_builtin(data):
    """Linear: efficient built-in functions"""
    return sorted([x for x in data if x > 0])


def count_elements(arr):
    """Linear: single pass with dict"""
    counts = {}
    for item in arr:
        counts[item] = counts.get(item, 0) + 1
    return counts


def optimized_duplicates(nums):
    """Linear: using set for efficient lookup"""
    seen = set()
    duplicates = set()
    for num in nums:
        if num in seen:
            duplicates.add(num)
        seen.add(num)
    return list(duplicates)


def necessary_comparison(items):
    """Should NOT be flagged: algorithm requires quadratic time"""
    pairs = []
    for i in range(len(items)):
        for j in range(i + 1, len(items)):
            pairs.append((items[i], items[j]))
    return pairs
