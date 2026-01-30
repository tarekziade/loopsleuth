def bubble_sort(arr):
    """Quadratic: O(n²) - nested loops over same array"""
    n = len(arr)
    for i in range(n):
        for j in range(0, n - i - 1):
            if arr[j] > arr[j + 1]:
                arr[j], arr[j + 1] = arr[j + 1], arr[j]
    return arr


def find_duplicates(nums):
    """Quadratic: O(n²) - nested iteration to find duplicates"""
    duplicates = []
    for i in range(len(nums)):
        for j in range(i + 1, len(nums)):
            if nums[i] == nums[j] and nums[i] not in duplicates:
                duplicates.append(nums[i])
    return duplicates


def remove_elements(lst, to_remove):
    """Quadratic: O(n²) - list.remove() is O(n) inside loop"""
    for item in to_remove:
        if item in lst:
            lst.remove(item)  # O(n) operation
    return lst


def string_concatenation(words):
    """Quadratic: O(n²) - string concatenation in loop"""
    result = ""
    for word in words:
        result += word + " "  # Creates new string each time
    return result


def linear_search(arr, target):
    """Linear: O(n) - single loop"""
    for i, val in enumerate(arr):
        if val == target:
            return i
    return -1


def binary_search(arr, target):
    """Logarithmic: O(log n) - divide and conquer"""
    left, right = 0, len(arr) - 1
    while left <= right:
        mid = (left + right) // 2
        if arr[mid] == target:
            return mid
        elif arr[mid] < target:
            left = mid + 1
        else:
            right = mid - 1
    return -1


def count_elements(arr):
    """Linear: O(n) - single pass with dict"""
    counts = {}
    for item in arr:
        counts[item] = counts.get(item, 0) + 1
    return counts


def optimized_duplicates(nums):
    """Linear: O(n) - using set for efficient lookup"""
    seen = set()
    duplicates = set()
    for num in nums:
        if num in seen:
            duplicates.add(num)
        seen.add(num)
    return list(duplicates)


class DataProcessor:
    """Example class with methods to analyze"""

    def __init__(self, data):
        self.data = data

    def nested_comparison(self):
        """Quadratic: O(n²) - comparing all pairs"""
        pairs = []
        for i in range(len(self.data)):
            for j in range(i + 1, len(self.data)):
                if self.data[i] + self.data[j] == 10:
                    pairs.append((self.data[i], self.data[j]))
        return pairs

    def efficient_sum(self):
        """Linear: O(n) - single pass"""
        return sum(self.data)
