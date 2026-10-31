B4 = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]]


def bayer4(x, y):
    """Return the ordered threshold for a native-grid pixel."""
    return (B4[y % 4][x % 4] + 0.5) / 16.0
