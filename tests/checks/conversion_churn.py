"""
Examples for the conversion-churn check.
"""


def to_device_in_loop(tensor, device, n):
    """Conversion churn: repeated .to(device) on invariant tensor"""
    out = []
    for _i in range(n):
        out.append(tensor.to(device))
    return out


def numpy_in_loop(tensor_list):
    """Conversion churn: repeated .cpu().numpy() in loop"""
    out = []
    for t in tensor_list:
        out.append(t.cpu().numpy())
    return out


# Clean examples (should NOT be flagged)


def convert_once(tensor, device, n):
    """Safe: single conversion before loop"""
    converted = tensor.to(device)
    out = []
    for _i in range(n):
        out.append(converted)
    return out
