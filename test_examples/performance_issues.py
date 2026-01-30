"""
Additional examples of quadratic complexity patterns
"""


def matrix_multiply_naive(a, b):
    """Quadratic: O(n²) for square matrices - naive matrix multiplication"""
    n = len(a)
    result = [[0] * n for _ in range(n)]
    for i in range(n):
        for j in range(n):
            for k in range(n):  # Actually O(n³)
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


def remove_duplicates_slow(lst):
    """Quadratic: O(n²) - using list.index() inside loop"""
    result = []
    for item in lst:
        if item not in result:  # O(n) lookup
            result.append(item)
    return result


def join_strings_slow(words):
    """Quadratic: O(n²) - string concatenation in loop"""
    sentence = ""
    for word in words:
        sentence = sentence + word + " "  # Creates new string each time
    return sentence.strip()


def cartesian_product(list1, list2):
    """Quadratic: O(n*m) - but this is intentional/necessary"""
    result = []
    for item1 in list1:
        for item2 in list2:
            result.append((item1, item2))
    return result


def sum_of_pairs(nums, target):
    """Quadratic: O(n²) - checking all pairs"""
    pairs = []
    for i in range(len(nums)):
        for j in range(i + 1, len(nums)):
            if nums[i] + nums[j] == target:
                pairs.append((nums[i], nums[j]))
    return pairs


def contains_subsequence_slow(text, pattern):
    """Quadratic: O(n*m) - naive substring search"""
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


# Optimized versions for comparison


def check_duplicates_fast(arr1, arr2):
    """Linear: O(n+m) - using sets"""
    set1 = set(arr1)
    set2 = set(arr2)
    return list(set1 & set2)


def remove_duplicates_fast(lst):
    """Linear: O(n) - using dict to preserve order"""
    return list(dict.fromkeys(lst))


def join_strings_fast(words):
    """Linear: O(n) - using join()"""
    return " ".join(words)


def sum_of_pairs_fast(nums, target):
    """Linear: O(n) - using hash map"""
    seen = {}
    pairs = []
    for num in nums:
        complement = target - num
        if complement in seen:
            pairs.append((complement, num))
        seen[num] = True
    return pairs


def contains_subsequence_fast(text, pattern):
    """Linear: O(n) - using in operator (optimized C implementation)"""
    return pattern in text
