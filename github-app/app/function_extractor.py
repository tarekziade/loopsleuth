"""Map source line numbers to Python function names using the ast module."""

import ast


def extract_function_ranges(source: str) -> dict[str, tuple[int, int]]:
    """Return {qualified_name: (start_line, end_line)} for every function."""
    try:
        tree = ast.parse(source)
    except SyntaxError:
        return {}

    functions: dict[str, tuple[int, int]] = {}

    for node in ast.walk(tree):
        if isinstance(node, ast.ClassDef):
            for item in node.body:
                if isinstance(item, (ast.FunctionDef, ast.AsyncFunctionDef)):
                    name = f"{node.name}.{item.name}"
                    functions[name] = (item.lineno, item.end_lineno or item.lineno)
        elif isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
            # Skip methods already captured above
            if node.col_offset == 0:
                functions[node.name] = (
                    node.lineno,
                    node.end_lineno or node.lineno,
                )
    return functions


def find_changed_functions(source: str, changed_lines: list[int]) -> set[str]:
    """Return names of functions that overlap with any changed line."""
    ranges = extract_function_ranges(source)
    result: set[str] = set()
    for name, (start, end) in ranges.items():
        if any(start <= line <= end for line in changed_lines):
            result.add(name)
    return result
