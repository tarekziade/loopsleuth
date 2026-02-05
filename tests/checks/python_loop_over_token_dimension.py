"""
Examples for the python-loop-over-token-dimension check.
"""


def scale_tokens_slow(hidden_states, scale):
    """ML loop over tokens: explicit loop over token dimension"""
    batch, seq_len, dim = hidden_states.shape
    out = hidden_states.clone()
    for i in range(seq_len):
        out[:, i, :] = hidden_states[:, i, :] * scale
    return out


def sum_tokens_slow(hidden_states):
    """ML loop over tokens: reduction via Python loop"""
    batch, seq_len, dim = hidden_states.shape
    out = hidden_states[:, 0, :].clone()
    for i in range(1, seq_len):
        out = out + hidden_states[:, i, :]
    return out


# Clean examples (should NOT be flagged)


def loop_over_layers(layers, hidden_states):
    """Safe: loop over layers, not token dimension"""
    out = hidden_states
    for layer in layers:
        out = layer(out)
    return out
