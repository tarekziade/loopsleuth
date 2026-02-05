"""
Examples for the mask-built-in-layer-loop check.
"""


def build_mask_in_layer_loop(layers, x):
    """Mask built inside a layer loop"""
    import torch

    out = x
    seq_len = x.shape[1]
    for layer in layers:
        mask = torch.tril(torch.ones(seq_len, seq_len, device=x.device))
        out = layer(out, mask=mask)
    return out


# Clean examples (should NOT be flagged)


def build_mask_once(layers, x):
    """Safe: mask created once before loop"""
    import torch

    out = x
    seq_len = x.shape[1]
    mask = torch.tril(torch.ones(seq_len, seq_len, device=x.device))
    for layer in layers:
        out = layer(out, mask=mask)
    return out
