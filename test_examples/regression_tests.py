"""
Regression tests for bug fixes

This file contains examples that should NOT be flagged as performance issues.
These are regression tests for bugs that were previously causing false positives.
"""


# ============================================================================
# Bug Fix #1: Keyword in explanatory text causing false positives
# ============================================================================
# These functions should NOT be flagged - they don't have actual performance issues


def simple_loop(items):
    """Should NOT be flagged: Simple O(n) iteration"""
    result = []
    for item in items:
        result.append(item * 2)
    return result


def linear_search(haystack, needle):
    """Should NOT be flagged: Single linear search O(n)"""
    for item in haystack:
        if item == needle:
            return True
    return False


def process_with_builtin(data):
    """Should NOT be flagged: Using efficient built-in functions"""
    # Built-in functions are optimized in C
    return sorted([x for x in data if x > 0])


def single_pass_aggregation(numbers):
    """Should NOT be flagged: Single pass O(n) aggregation"""
    total = 0
    count = 0
    max_val = float('-inf')

    for num in numbers:
        total += num
        count += 1
        if num > max_val:
            max_val = num

    return {
        'sum': total,
        'count': count,
        'average': total / count if count > 0 else 0,
        'max': max_val
    }


def map_transform(items):
    """Should NOT be flagged: Simple transformation using dict"""
    lookup = {item.id: item for item in items}  # O(n) dict creation
    return lookup


# ============================================================================
# Bug Fix #2: __init__ methods being incorrectly flagged
# ============================================================================
# Constructor methods that perform one-time initialization should NOT be flagged


class Worker:
    """Example from molotov/worker.py that was incorrectly flagged"""

    def __init__(
        self,
        wid,
        results,
        console,
        args,
        statsd=None,
        delay=0,
        loop=None,
    ):
        """Should NOT be flagged: Simple initialization of instance attributes"""
        self.wid = wid
        self.results = results
        self.console = console
        self.loop = loop
        self.args = args
        self.statsd = statsd
        self.delay = delay
        self.count = 0
        self.worker_start = 0
        # Multiple function calls are OK in __init__ - they run once per object
        self._setup = self._get_fixture("setup")
        self._teardown = self._get_fixture("teardown")
        self._active_sessions = {}

    def _get_fixture(self, name):
        """Helper method"""
        return f"fixture_{name}"


class Database:
    """Another __init__ example with initialization logic"""

    def __init__(self, config):
        """Should NOT be flagged: Initialization with setup calls"""
        self.config = config
        self.connection = None
        self.pool_size = config.get('pool_size', 10)

        # One-time setup operations in __init__ are fine
        self._validate_config()
        self._setup_logging()
        self._initialize_pool()

    def _validate_config(self):
        """Validates configuration"""
        required = ['host', 'port', 'database']
        for key in required:
            if key not in self.config:
                raise ValueError(f"Missing required config: {key}")

    def _setup_logging(self):
        """Sets up logging"""
        pass

    def _initialize_pool(self):
        """Initializes connection pool"""
        pass


class DataProcessor:
    """__init__ with list comprehension"""

    def __init__(self, raw_data):
        """Should NOT be flagged: One-time data preparation in constructor"""
        self.raw_data = raw_data
        # These transformations happen once when object is created
        self.cleaned_data = [x.strip() for x in raw_data if x]
        self.indexed_data = {i: x for i, x in enumerate(self.cleaned_data)}
        self.metadata = {
            'count': len(self.cleaned_data),
            'first': self.cleaned_data[0] if self.cleaned_data else None,
            'last': self.cleaned_data[-1] if self.cleaned_data else None,
        }


class CachedCalculator:
    """__init__ that precomputes values"""

    def __init__(self, max_value):
        """Should NOT be flagged: Precomputation in constructor is intentional"""
        self.max_value = max_value
        # Precomputing squares for fast lookup - this is intentional optimization
        # Running once during initialization, not in hot path
        self.squares = [i * i for i in range(max_value + 1)]
        self.cubes = [i * i * i for i in range(max_value + 1)]

    def get_square(self, n):
        """O(1) lookup after precomputation"""
        return self.squares[n] if n <= self.max_value else n * n

    def get_cube(self, n):
        """O(1) lookup after precomputation"""
        return self.cubes[n] if n <= self.max_value else n * n * n


# ============================================================================
# Edge cases that should NOT be flagged
# ============================================================================


def process_small_fixed_data():
    """Should NOT be flagged: Nested loops over small fixed-size data"""
    # When data size is constant/small, O(n²) is acceptable
    colors = ['red', 'green', 'blue']
    sizes = ['S', 'M', 'L']

    combinations = []
    for color in colors:
        for size in sizes:
            combinations.append(f"{color}-{size}")

    return combinations


def necessary_comparison(items):
    """Should NOT be flagged: Some algorithms require quadratic time"""
    # Finding all pairs is inherently O(n²) - can't be optimized
    pairs = []
    for i in range(len(items)):
        for j in range(i + 1, len(items)):
            pairs.append((items[i], items[j]))
    return pairs


def conditional_early_exit(matrix):
    """Should NOT be flagged: Nested loop with early exit"""
    # Worst case O(n²) but typically much better with early exit
    for i, row in enumerate(matrix):
        for j, val in enumerate(row):
            if val == 0:
                return (i, j)  # Early exit on first zero
    return None
